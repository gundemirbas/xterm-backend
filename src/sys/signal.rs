use crate::sys::SysResult;
use crate::sys::syscall::syscall4;

const SYS_RT_SIGPROCMASK: usize = 14;
const SYS_SIGNALFD4: usize = 289;

// Block signals in current thread/process using rt_sigprocmask
pub fn block_signals(mask_ptr: *const u64, sigsetsize: usize) -> SysResult<()> {
    // how = 0 -> SIG_BLOCK
    let r = unsafe { syscall4(SYS_RT_SIGPROCMASK, 0, mask_ptr as usize, 0, sigsetsize) };
    if r >= 0 { Ok(()) } else { Err(r) }
}

// Create a signalfd for the given mask. Returns fd.
pub fn signalfd(mask_ptr: *const u64, sigsetsize: usize, flags: usize) -> SysResult<usize> {
    // fd = -1 (use ~0usize) to create a new fd
    let fd_arg = !0usize;
    let r = unsafe { syscall4(SYS_SIGNALFD4, fd_arg, mask_ptr as usize, sigsetsize, flags) };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}
