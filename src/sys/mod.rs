pub type SysResult<T> = core::result::Result<T, isize>;

pub mod epoll;
pub mod fs;
pub mod mmap;
pub mod net;
pub mod pty;
pub mod signal;
pub mod syscall;
pub mod util;
