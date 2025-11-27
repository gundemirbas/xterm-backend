use crate::sys::SysResult;
use crate::sys::syscall::{syscall4_checked};

const SYS_RT_SIGPROCMASK: usize = 14;
const SYS_SIGNALFD4: usize = 289;

// Block signals in current thread/process using rt_sigprocmask
pub fn block_signals(mask_ptr: *const u64, sigsetsize: usize) -> SysResult<()> {
    // how = 0 -> SIG_BLOCK
    let _ = syscall4_checked(SYS_RT_SIGPROCMASK, 0, mask_ptr as usize, 0, sigsetsize)?;
    Ok(())
}

// Create a signalfd for the given mask. Returns fd.
pub fn signalfd(mask_ptr: *const u64, sigsetsize: usize, flags: usize) -> SysResult<usize> {
    // fd = -1 (use ~0usize) to create a new fd
    let fd_arg = !0usize;
    let r = syscall4_checked(SYS_SIGNALFD4, fd_arg, mask_ptr as usize, sigsetsize, flags)?;
    Ok(r as usize)
}
