//
// Created by mk606 on 26.06.2026.
//

#ifndef CLKMONOFF_UTILS_H
#define CLKMONOFF_UTILS_H

#include <linux/module.h>
#include <linux/kprobes.h>
#include <linux/ptrace.h>
#include <linux/delay.h>
#include <linux/kallsyms.h>
#include <linux/xarray.h>
#include <linux/ftrace.h>
#include <linux/linkage.h>
#include <linux/miscdevice.h>
#include <linux/fs.h>
#include <linux/wait.h>
#include <linux/uaccess.h>
#include <linux/minmax.h>

typedef unsigned char uchar;
typedef unsigned long ulong;
typedef unsigned int  uint;

extern char active;
typedef void (*clk_active_hook)(char is_active);
extern clk_active_hook __start_active_hooks[];
extern clk_active_hook __stop_active_hooks[];
#define I____PASTE(a, b) a##b
#define I____UNIQUE(prefix, id) I____PASTE(prefix, id)
#define I____ON_ACTIVE_EXPAND(id) \
    static void I____UNIQUE(I____active_fn_, id)(char is_active); \
    static clk_active_hook I____UNIQUE(I____active_ptr_, id) \
    __attribute__((section("active_hooks"), used, aligned(sizeof(void*)))) = I____UNIQUE(I____active_fn_, id); \
    static void I____UNIQUE(I____active_fn_, id)(char is_active)
#define ON_ACTIVE I____ON_ACTIVE_EXPAND(__COUNTER__)

struct ftrace_hook {
    const char* name;
    void* hook;
    void* orig;
    ulong address;
    struct ftrace_ops ops;
    int (*init)(void);
    void (*exit)(void);

    /// bits:
    /// 0 - hook loaded
    /// 1 - hook initialized
    int __internal;
};

extern struct ftrace_hook __start_ftrace_hooks[];
extern struct ftrace_hook __stop_ftrace_hooks[];

#define FTRACE_SIMPLE(fn, type, args) \
    static type (*orig_##fn)args; \
    static type hook_##fn args; \
    struct ftrace_hook __hook_##fn \
    __attribute__((section("ftrace_hooks"), used, aligned(sizeof(void*)))) = { \
        .name = #fn, \
        .hook = hook_##fn, \
        .orig = &orig_##fn, \
        .init = NULL, \
        .exit = NULL, \
        .__internal = 0 \
    }; \
    static type __attribute__((optimize("-fno-optimize-sibling-calls"))) hook_##fn args

#define FTRACE(fn, type, load_fn, unload_fn, args) \
    static type (*orig_##fn)args; \
    static type hook_##fn args; \
    static int I____load_fn_##fn(void) load_fn \
    static void I____unload_fn_##fn(void) unload_fn \
    struct ftrace_hook __hook_##fn \
    __attribute__((section("ftrace_hooks"), used, aligned(sizeof(void*)))) = { \
        .name = #fn, \
        .hook = hook_##fn, \
        .orig = &orig_##fn, \
        .init = I____load_fn_##fn, \
        .exit = I____unload_fn_##fn, \
        .__internal = 0 \
    }; \
    static type __attribute__((optimize("-fno-optimize-sibling-calls"))) hook_##fn args

#define WORK_CREATE(name, data, in_method_name, body) struct name { \
    struct work_struct I____work; \
    bool need_free; \
    struct data; \
}; \
static void I____fn_##name(struct work_struct* work) { \
    struct name* in_method_name = container_of(work, struct name, I____work); \
    body \
    if (in_method_name->need_free) kfree(in_method_name); \
} \
static __always_inline struct name* new_##name(gfp_t alloc) { \
    struct name* s = kmalloc(sizeof(*s), alloc); \
    if (s) { \
        s->need_free = true; \
        INIT_WORK(&s->I____work, I____fn_##name); \
    } \
    return s; \
} \
static __always_inline struct name* init_##name(struct name* s, bool need_free) { \
    if (s) { \
        INIT_WORK(&s->I____work, I____fn_##name); \
        s->need_free = need_free; \
    } \
    return s; \
}

#define WORK_NEW(name, alloc) new_##name(alloc)
#define WORK_RUN(s) schedule_work(&s->I____work)
#define WORK_RUN_V(s) schedule_work(&s.I____work)
#define WORK_NEW_RUN(name, alloc, vname, fn) do { \
    struct name* vname = new_##name(alloc); \
    if (vname) { \
        fn; \
        WORK_RUN(vname); \
    } \
} while (0)
#define WORK_INIT(name, s, need_free) init_##name(s, need_free)

#define LOCK_SPIN(sl, body) do { \
    ulong __FLAGS_SAVED_STATE__; \
    spin_lock_irqsave(&sl, __FLAGS_SAVED_STATE__); \
    body; \
    spin_unlock_irqrestore(&sl, __FLAGS_SAVED_STATE__); \
} while (0)

extern ulong (*kallsyms_lookup_name_ptr)(const char* name);
int hook(struct ftrace_hook* hook);
typedef void (*unhook_fn)(struct ftrace_hook* hook);
void unhook(struct ftrace_hook* hook);
void unhook_stage0(struct ftrace_hook* hook);
void unhook_stage1(struct ftrace_hook* hook);
int multi_hook(struct ftrace_hook* hooks, size_t count);
void multi_unhook(unhook_fn fn, struct ftrace_hook* hooks, size_t count);
char set_active(char is_active);

#endif //CLKMONOFF_UTILS_H
