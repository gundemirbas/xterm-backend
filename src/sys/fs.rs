use crate::runtime::syscall::{syscall1_checked, syscall3_checked};
use crate::sys::SysResult;
const SYS_CLOSE: usize = 3;
const SYS_OPEN: usize = 2;
const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;

pub fn close(fd: usize) -> SysResult<()> {
    let _ = syscall1_checked(SYS_CLOSE, fd)?;
    Ok(())
}
pub fn read(fd: usize, buf: &mut [u8]) -> SysResult<usize> {
    let r = syscall3_checked(SYS_READ, fd, buf.as_mut_ptr() as usize, buf.len())?;
    Ok(r as usize)
}
pub fn write(fd: usize, buf: &[u8]) -> SysResult<usize> {
    let r = syscall3_checked(SYS_WRITE, fd, buf.as_ptr() as usize, buf.len())?;
    Ok(r as usize)
}
pub fn open(path: *const u8, flags: usize, mode: usize) -> SysResult<usize> {
    let r = syscall3_checked(SYS_OPEN, path as usize, flags, mode)?;
    Ok(r as usize)
}
