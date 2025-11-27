pub type SysResult<T> = core::result::Result<T, isize>;

pub mod syscall;
pub mod net;
pub mod epoll;
pub mod fs;
pub mod pty;
pub mod mmap;
