//
// Created by mk606 on 26.06.2026.
//
#include "utils/utils.h"

#include <linux/input.h>
#include <linux/bitmap.h>

static ulong key_bit_off[BITS_TO_LONGS(KEY_CNT)];
static ulong key_bit_on[BITS_TO_LONGS(KEY_CNT)];
static ulong key_bit[BITS_TO_LONGS(KEY_CNT)];
static uchar key[KEY_CNT] = { [0 ... KEY_CNT-1] = 0 };
static DEFINE_SPINLOCK(key_lock);

static char kb_chk(char is_active) {
    if (is_active && bitmap_equal(key_bit_off, key_bit, KEY_CNT)) return set_active(false);
    if (!is_active && bitmap_subset(key_bit_on, key_bit, KEY_CNT)) return set_active(true);
    return is_active;
}

static char kb_up(uint code) {
    char v;
    LOCK_SPIN(key_lock, {
        key[code] += 1;
        __set_bit(code, key_bit);
        v = kb_chk(READ_ONCE(active));
    });
    return v;
}

static char kb_down(uint code) {
    char v;
    LOCK_SPIN(key_lock, {
        uchar c = key[code];
        if (c > 0) c = (key[code] -= 1);
        v = READ_ONCE(active);
        if (!c) {
            __clear_bit(code, key_bit);
            if (v) v = kb_chk(v);
        }
    });
    return v;
}

static char* init_keys_on  = NULL;
static char* init_keys_off = NULL;
module_param_named(keys_on,  init_keys_on,  charp, 0);
module_param_named(keys_off, init_keys_off, charp, 0);
FTRACE(input_event, void, {
    if (!init_keys_on)  return -EINVAL;
    if (!init_keys_off) return -EINVAL;
    int ret;
    if ((ret = bitmap_parselist(init_keys_on,  key_bit_on,  KEY_CNT))) return ret;
    if ((ret = bitmap_parselist(init_keys_off, key_bit_off, KEY_CNT))) return ret;
    return 0;
}, {}, (struct input_dev* dev, uint type, uint code, int value)) {
    char act;
    bool dwn = type == EV_KEY && value == 0;
    if (type == EV_KEY && value == 1) {
        act = kb_up(code);
    } else if (dwn) {
        act = kb_down(code);
    } else {
        act = READ_ONCE(active);
    }
    if (!act || type == EV_SYN || dwn) orig_input_event(dev, type, code, value);
}