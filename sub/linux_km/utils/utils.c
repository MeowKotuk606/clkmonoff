//
// Created by mk606 on 26.06.2026.
//
#include "utils.h"

MODULE_LICENSE("GPL");
MODULE_AUTHOR("MeowKotuk606");
MODULE_DESCRIPTION("ClkMonOff helper");

char active = 0;
static char active_ready = 0;
static DECLARE_WAIT_QUEUE_HEAD(wq);
static ssize_t I____dev_read(struct file* file, char __user* buf, size_t count, loff_t* ppos) {
    if (!active_ready) {
        if (file->f_flags & O_NONBLOCK) return -EAGAIN;
        if (wait_event_interruptible(wq, active_ready)) return -ERESTARTSYS;
    }
    if (copy_to_user(buf, &active, 1)) return -EFAULT;
    active_ready = 0;
    return 1;
}
static const struct file_operations fops = {
    .owner = THIS_MODULE,
    .read = I____dev_read,
};
static struct miscdevice clk_dev = {
    .minor = MISC_DYNAMIC_MINOR,
    .name = "clkmonoff",
    .fops = &fops,
};

ulong (*kallsyms_lookup_name_ptr)(const char* name) = NULL;
WORK_CREATE(set_active_work, {
    char val;
}, s, {
    char is_active = s->val;
    if (active == is_active) return;
    pr_err("set_active_work: %d\n", is_active);
    active = is_active;
    active_ready = 1;
    wake_up_interruptible(&wq);
    for (clk_active_hook* ptr = __start_active_hooks; ptr < __stop_active_hooks; ptr++) if (*ptr) (*ptr)(is_active);
});
static struct set_active_work set_act_w;
char set_active(char is_active) {
    set_act_w.val = is_active;
    WORK_RUN_V(set_act_w);
    return is_active;
}

static int __init init_mod(void) {
    struct kprobe kp = { .symbol_name = "kallsyms_lookup_name" };
    int ret;
    if ((ret = register_kprobe(&kp))) return ret;
    kallsyms_lookup_name_ptr = (void*) kp.addr;
    unregister_kprobe(&kp);
    size_t hooks = __stop_ftrace_hooks - __start_ftrace_hooks;
    if ((ret = multi_hook(__start_ftrace_hooks, hooks))) return ret;
    WORK_INIT(set_active_work, &set_act_w, false);
    if ((ret = misc_register(&clk_dev))) multi_unhook(unhook, __start_ftrace_hooks, hooks);
    return ret;
}
static void exit_mod(void) {
    size_t hooks = __stop_ftrace_hooks - __start_ftrace_hooks;
    multi_unhook(unhook_stage0, __start_ftrace_hooks, hooks);
    flush_scheduled_work();
    multi_unhook(unhook_stage1, __start_ftrace_hooks, hooks);
    misc_deregister(&clk_dev);
}
module_init(init_mod);
module_exit(exit_mod);

__attribute__((optimize("-fno-optimize-sibling-calls")))
static void notrace fh_ftrace_thunk(ulong ip, ulong parent_ip, struct ftrace_ops* ops, struct ftrace_regs* fregs) {
    struct pt_regs* regs = ftrace_get_regs(fregs);
    struct ftrace_hook* hook = container_of(ops, struct ftrace_hook, ops);
    if (regs && !within_module(parent_ip, THIS_MODULE)) regs->ip = (ulong) hook->hook;
}

int hook(struct ftrace_hook* hook) {
    hook->address = kallsyms_lookup_name_ptr(hook->name);
    if (!hook->address) {
        pr_err("Failed to resolve %s\n", hook->name);
        return -ENOENT;
    }
    *((ulong*) hook->orig) = hook->address;
    hook->ops.func = fh_ftrace_thunk;
    hook->ops.flags = FTRACE_OPS_FL_SAVE_REGS | FTRACE_OPS_FL_IPMODIFY | FTRACE_OPS_FL_RECURSION;
    int ret = ftrace_set_filter_ip(&hook->ops, hook->address, 0, 0);
    if (ret) {
        pr_err("ftrace_set_filter_ip failed for %s: %d\n", hook->name, ret);
        return ret;
    }
    ret = register_ftrace_function(&hook->ops);
    if (ret) {
        pr_err("register_ftrace_function failed for %s: %d\n", hook->name, ret);
        ftrace_set_filter_ip(&hook->ops, hook->address, 1, 0);
        return ret;
    }
    return 0;
}


void unhook(struct ftrace_hook* hook) {
    unhook_stage0(hook);
    unhook_stage1(hook);
}
void unhook_stage0(struct ftrace_hook* hook) {
    if (!(hook->__internal & BIT(0))) return;
    unregister_ftrace_function(&hook->ops);
    ftrace_set_filter_ip(&hook->ops, hook->address, 1, 0);
    hook->__internal &= ~BIT(0);
}
void unhook_stage1(struct ftrace_hook* hook) {
    if (!hook->exit || !(hook->__internal & BIT(1))) return;
    hook->exit();
    hook->__internal &= ~BIT(1);
}

int multi_hook(struct ftrace_hook* hooks, size_t count) {
    int ret;
    for (size_t i = 0; i < count; i++) {
        if ((ret = hook(&hooks[i]))) goto error;
        hooks[i].__internal |= BIT(0);
        if (hooks[i].init) {
            if ((ret = hooks[i].init())) goto error;
            hooks[i].__internal |= BIT(1);
        }
        continue;
    error:
        while (1) {
            unhook(&hooks[i]);
            if (i == 0) break;
            i--;
        }
        return ret;
    }
    return 0;
}

void multi_unhook(unhook_fn fn, struct ftrace_hook* hooks, size_t count) {
    for (size_t i = 0; i < count; i++) fn(&hooks[i]);
}