//
// Created by mk606 on 26.06.2026.
//
#include "utils/utils.h"

#include <sound/pcm.h>
#include <sound/core.h>
#include <sound/control.h>
#include <linux/skbuff.h>
#include <net/bluetooth/bluetooth.h>
#include <net/bluetooth/l2cap.h>
#include <net/bluetooth/hci_core.h>

static DEFINE_XARRAY(muted_pcms);
ON_ACTIVE {
    if (!is_active) {
        ulong id;
        struct snd_pcm_substream* substream;
        xa_for_each(&muted_pcms, id, substream) {
            snd_pcm_stop_xrun(substream);
            xa_erase(&muted_pcms, id);
        }
    }
}

WORK_CREATE(pcm_stop_work, {
    struct snd_pcm_substream* substream;
}, s, {
    snd_pcm_stop_xrun(s->substream);
})

FTRACE_SIMPLE(snd_pcm_period_elapsed, void, (struct snd_pcm_substream* substream)) {
    if (substream && READ_ONCE(active)) {
        if (substream->pcm->nonatomic) {
            WORK_NEW_RUN(pcm_stop_work, GFP_ATOMIC, w, {
                w->substream = substream;
            });
        } else snd_pcm_stop_xrun(substream);
        return;
    }
    orig_snd_pcm_period_elapsed(substream);
}

FTRACE_SIMPLE(snd_pcm_update_hw_ptr, int, (struct snd_pcm_substream* substream)) {
    if (substream && READ_ONCE(active)) {
        struct snd_pcm_runtime* runtime = substream->runtime;
        if (runtime && runtime->dma_area && runtime->dma_bytes > 0) {
            memset(runtime->dma_area, 0, runtime->dma_bytes);
        }
        WORK_NEW_RUN(pcm_stop_work, GFP_ATOMIC, w, {
            w->substream = substream;
        });
        return -EPIPE;
    }
    return orig_snd_pcm_update_hw_ptr(substream);
}

FTRACE(snd_pcm_do_start, int, { return 0; }, {
    ulong id;
    struct snd_pcm_substream* sub;
    xa_for_each(&muted_pcms, id, sub) xa_erase(&muted_pcms, id);
    xa_destroy(&muted_pcms);
}, (struct snd_pcm_substream* substream, int state)) {
    if (substream && READ_ONCE(active)) {
        xa_store(&muted_pcms, (ulong) substream, substream, GFP_ATOMIC);
        return 0;
    }
    return orig_snd_pcm_do_start(substream, state);
}

FTRACE_SIMPLE(snd_pcm_release_substream, void, (struct snd_pcm_substream* substream)) {
    if (substream) xa_erase(&muted_pcms, (ulong) substream);
    orig_snd_pcm_release_substream(substream);
}

FTRACE_SIMPLE(l2cap_chan_send, int, (struct l2cap_chan* chan, struct msghdr* msg, size_t len)) {
    if (chan && READ_ONCE(active) && chan->psm == 0x0019) return len;
    return orig_l2cap_chan_send(chan, msg, len);
}

static int hci_fn(int (*orig)(struct hci_dev* hdev, struct sk_buff* skb), struct hci_dev* hdev, struct sk_buff* skb) {
    if (skb && READ_ONCE(active)) {
        __u8 type = bt_cb(skb)->pkt_type;
        if (type == HCI_SCODATA_PKT || type == HCI_ISODATA_PKT) {
            dev_kfree_skb_any(skb);
            return 0;
        }
    }
    return orig(hdev, skb);
}

FTRACE_SIMPLE(hci_send_frame, int, (struct hci_dev* hdev, struct sk_buff* skb)) {
    return hci_fn(orig_hci_send_frame, hdev, skb);
}

FTRACE_SIMPLE(hci_recv_frame, int, (struct hci_dev* hdev, struct sk_buff* skb)) {
    return hci_fn(orig_hci_recv_frame, hdev, skb);
}