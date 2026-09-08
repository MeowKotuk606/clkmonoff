//
// Created by mk606 on 26.06.2026.
//
#include "utils/utils.h"

#include <drm/display/drm_dp_helper.h>
#include <drm/drm_atomic.h>
#include <drm/drm_plane.h>
#include <drm/drm_connector.h>
#include <drm/drm_probe_helper.h>
#include <drm/drm_drv.h>
#include <drm/drm_file.h>
#include <linux/i2c.h>

#include <linux/version.h>

#if LINUX_VERSION_CODE >= KERNEL_VERSION(7, 2, 0)
    #define drm_atomic_state drm_atomic_commit
    #define drm_atomic_state_alloc drm_atomic_commit_alloc
    #define drm_atomic_state_clear drm_atomic_commit_clear
    #define drm_atomic_state_put drm_atomic_commit_put
#endif

static struct drm_color_ctm BLACK_CTM = {0};
static struct xarray* drm_minors_xa_ptr = NULL;
static DEFINE_XARRAY(saved_ctm);

static bool is_black(struct drm_property_blob* blob) {
    if (!blob || blob->length != sizeof(struct drm_color_ctm)) return false;
    const __u64* m = blob->data;
    return (m[0] | m[1] | m[2] | m[3] | m[4] | m[5] | m[6] | m[7] | m[8]) == 0;
}

static void set_vcp_ddc(struct i2c_adapter* adapter, u8 buf[7], u8 feat, u8 v) {
    buf[0] = 0x51;
    buf[1] = 0x84;
    buf[2] = 0x03;
    buf[3] = feat;
    buf[4] = 0x00;
    buf[5] = v;
    buf[6] = 0x6E ^ 0x51 ^ 0x84 ^ 0x03 ^ feat ^ 0x00 ^ v;
    struct i2c_msg msg = {
        .addr = 0x37,
        .flags = 0,
        .len = 7,
        .buf = buf,
    };
    int ret = i2c_transfer(adapter, &msg, 1);
    if (ret != 1) pr_err("i2c_transfer failed on %px: %d\n", adapter, ret);
}

WORK_CREATE(ddc_send_work, {
    struct i2c_adapter* adapter;
    u8 buf[7];
    u8 feat;
    u8 v;
}, s, {
    set_vcp_ddc(s->adapter, s->buf, s->feat, s->v);
})

static int __send_to_all_aux(struct device* dev, void* data) {
    struct i2c_adapter* a = i2c_verify_adapter(dev);
    if (a && a->algo_data) {
        struct drm_dp_aux* aux = a->algo_data;
        if (&aux->ddc == a) {
            WORK_NEW_RUN(ddc_send_work, GFP_KERNEL, w, {
                w->adapter = a;
                w->feat = 0xD6;
                w->v = *(u8*)data;
            });
        }
    }
    return 0;
}

ON_ACTIVE {
    int ret;
    ulong id = 0;
    struct drm_minor* minor;
    xa_for_each(drm_minors_xa_ptr, id, minor) {
        struct drm_device* dev = minor->dev;
        if (minor->type == DRM_MINOR_PRIMARY && dev) {
            struct drm_modeset_acquire_ctx ctx;
            struct drm_crtc* crtc;
            drm_modeset_acquire_init(&ctx, 0);

            retry_commit:
            struct drm_atomic_state* state = drm_atomic_state_alloc(dev);
            if (!state) {
                drm_modeset_drop_locks(&ctx);
                drm_modeset_acquire_fini(&ctx);
                continue;
            }
            state->acquire_ctx = &ctx;
            drm_for_each_crtc(crtc, dev) {
                struct drm_crtc_state* crtc_state = drm_atomic_get_crtc_state(state, crtc);
                if (IS_ERR(crtc_state)) {
                    ret = PTR_ERR(crtc_state);
                    goto fail_commit;
                }
                if (is_active) {
                    struct drm_property_blob* old = crtc->state->ctm;
                    if (old) {
                        drm_property_blob_get(old);
                        struct drm_property_blob* saved = xa_store(&saved_ctm, crtc->base.id, old, GFP_KERNEL);
                        if (saved && !xa_is_err(saved)) drm_property_blob_put(saved);
                    }
                    struct drm_property_blob* blob = drm_property_create_blob(dev, sizeof(struct drm_color_ctm), &BLACK_CTM);
                    if (!IS_ERR(blob)) {
                        drm_property_replace_blob(&crtc_state->ctm, blob);
                        drm_property_blob_put(blob);
                        crtc_state->color_mgmt_changed = true;
                    }
                } else {
                    struct drm_property_blob* blob = xa_erase(&saved_ctm, crtc->base.id);
                    drm_property_replace_blob(&crtc_state->ctm, blob);
                    if (blob) drm_property_blob_put(blob);
                    crtc_state->color_mgmt_changed = true;
                }
            }
            ret = drm_atomic_commit(state);

            fail_commit:
            if (ret == -EDEADLK) {
                drm_atomic_state_clear(state);
                drm_modeset_backoff(&ctx);
                goto retry_commit;
            }
            if (ret >= 0) {
                const u8 v = is_active ? 0x05 : 0x01;
                i2c_for_each_dev((void*)&v, __send_to_all_aux);

                struct drm_connector* connector;
                struct drm_connector_list_iter conn_iter;
                drm_connector_list_iter_begin(dev, &conn_iter);
                drm_for_each_connector_iter(connector, &conn_iter) {
                    if (connector->connector_type != DRM_MODE_CONNECTOR_DisplayPort && connector->ddc && connector->status == connector_status_connected) {
                        WORK_NEW_RUN(ddc_send_work, GFP_KERNEL, w, {
                            w->adapter = connector->ddc;
                            w->feat = 0xD6;
                            w->v = v;
                        });
                    }
                }
                drm_connector_list_iter_end(&conn_iter);
            }
            drm_atomic_state_put(state);
            drm_modeset_drop_locks(&ctx);
            drm_modeset_acquire_fini(&ctx);
        }
    }
}

WORK_CREATE(blob_put_work, {
    struct drm_property_blob* blob;
}, s, {
    drm_property_blob_put(s->blob);
})

static inline int blackout(struct drm_atomic_state* state, int(*orig)(struct drm_atomic_state* state) ) {
    if (state && READ_ONCE(active)) {
        struct drm_crtc* crtc;
        struct drm_crtc_state *old_crtc_state, *new_crtc_state;
        int i;
        for_each_oldnew_crtc_in_state(state, crtc, old_crtc_state, new_crtc_state, i) {
            if (new_crtc_state && old_crtc_state) {
                struct drm_property_blob* new = new_crtc_state->ctm;
                struct drm_property_blob* old = old_crtc_state->ctm;
                if (new != old && !is_black(new)) {
                    if (new) drm_property_blob_get(new);
                    struct drm_property_blob* saved = xa_store(&saved_ctm, crtc->base.id, new, GFP_ATOMIC);
                    if (saved && !xa_is_err(saved)) WORK_NEW_RUN(blob_put_work, GFP_ATOMIC, w, {
                        w->blob = saved;
                    });
                    drm_property_replace_blob(&new_crtc_state->ctm, old);
                }
            }
        }
    }
    return orig(state);
}

FTRACE(drm_atomic_commit, int, {
    drm_minors_xa_ptr = (struct xarray*) kallsyms_lookup_name_ptr("drm_minors_xa");
    if (drm_minors_xa_ptr == 0) {
        pr_err("Failed to resolve drm_minors_xa_ptr\n");
        return -ENOENT;
    }
    return 0;
}, {
    ulong id;
    struct drm_property_blob* blob;
    xa_for_each(&saved_ctm, id, blob) drm_property_blob_put(blob);
    xa_destroy(&saved_ctm);
}, (struct drm_atomic_state* state)) {
    return blackout(state, orig_drm_atomic_commit);
}

FTRACE_SIMPLE(drm_atomic_nonblocking_commit, int, (struct drm_atomic_state* state)) {
    return blackout(state, orig_drm_atomic_nonblocking_commit);
}