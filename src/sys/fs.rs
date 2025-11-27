use crate::sys::{SysResult};
use crate::sys::syscall::{syscall1, syscall3};
const SYS_CLOSE: usize = 3;
const SYS_OPEN: usize = 2;
const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;

pub fn close(fd: usize) -> SysResult<()> {
    let r = unsafe { syscall1(SYS_CLOSE, fd) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
pub fn read(fd: usize, buf: &mut [u8]) -> SysResult<usize> {
    let r = unsafe { syscall3(SYS_READ, fd, buf.as_mut_ptr() as usize, buf.len()) };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}
pub fn write(fd: usize, buf: &[u8]) -> SysResult<usize> {
    let r = unsafe { syscall3(SYS_WRITE, fd, buf.as_ptr() as usize, buf.len()) };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}
pub fn open(path: *const u8, flags: usize, mode: usize) -> SysResult<usize> {
    let r = unsafe { syscall3(SYS_OPEN, path as usize, flags, mode) };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}
