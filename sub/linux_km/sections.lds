SECTIONS {
    active_hooks : {
        __start_active_hooks = .;
        KEEP(*(active_hooks))
        __stop_active_hooks = .;
    }
    ftrace_hooks : {
        __start_ftrace_hooks = .;
        KEEP(*(ftrace_hooks))
        __stop_ftrace_hooks = .;
    }
}