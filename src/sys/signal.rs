use crate::runtime::syscall::syscall4_checked;
use crate::sys::SysResult;

const SYS_RT_SIGPROCMASK: usize = 14;
const SYS_SIGNALFD4: usize = 289;

pub fn block_signals(mask_ptr: *const u64, sigsetsize: usize) -> SysResult<()> {
    let _ = syscall4_checked(SYS_RT_SIGPROCMASK, 0, mask_ptr as usize, 0, sigsetsize)?;
    Ok(())
}

pub fn signalfd(mask_ptr: *const u64, sigsetsize: usize, flags: usize) -> SysResult<usize> {
    let fd_arg = !0usize;
    let r = syscall4_checked(SYS_SIGNALFD4, fd_arg, mask_ptr as usize, sigsetsize, flags)?;
    Ok(r as usize)
}
