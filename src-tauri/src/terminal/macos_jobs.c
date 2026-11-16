// Use SDK layouts for Mach ports and audit tokens. No task-control port or
// task memory access is needed. Forward only version-bound process identities.
#include <dlfcn.h>
#include <errno.h>
#include <libproc.h>
#include <mach/mach.h>
#include <signal.h>
#include <string.h>
#include <unistd.h>

int gibbon_job_token(pid_t owner, pid_t candidate, unsigned int words[8]) {
    if (owner <= 1 || candidate <= 1 || candidate == owner ||
        candidate == getpid() || getsid(candidate) != owner)
        return 0;
    mach_port_t name = MACH_PORT_NULL;
    if (task_name_for_pid(mach_task_self(), candidate, &name) != KERN_SUCCESS)
        return -1;
    audit_token_t token;
    mach_msg_type_number_t count = TASK_AUDIT_TOKEN_COUNT;
    kern_return_t result = task_info(name, TASK_AUDIT_TOKEN, (task_info_t)&token, &count);
    mach_port_deallocate(mach_task_self(), name);
    if (result != KERN_SUCCESS || count != TASK_AUDIT_TOKEN_COUNT) return -1;
    if (getsid(candidate) != owner) return 0;
    _Static_assert(sizeof(token) == 8 * sizeof(*words), "audit token layout");
    memcpy(words, &token, sizeof(token));
    return 1;
}

int gibbon_stop_job(const unsigned int words[8]) {
    // Older macOS versions may lack version-checked signalling. Never fall
    // back to kill(pid) when the kernel cannot verify this exact identity.
    typedef int (*signal_fn)(audit_token_t *, int);
    signal_fn send = (signal_fn)dlsym(RTLD_DEFAULT, "proc_signal_with_audittoken");
    if (send) {
        audit_token_t token;
        memcpy(&token, words, sizeof(token));
        return send(&token, SIGKILL);
    }
    return ENOSYS;
}
