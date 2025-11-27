use crate::sys::SysResult;
use crate::sys::syscall::{syscall1, syscall4};

const SYS_EPOLL_CREATE1: usize = 291;
const SYS_EPOLL_CTL: usize = 233;
const SYS_EPOLL_WAIT: usize = 232;

pub const EPOLLIN: u32 = 0x001;

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct EpollEvent {
    pub events: u32,
    pub data: u64,
}
impl EpollEvent {
    pub fn new(events: u32, fd: usize) -> Self {
        Self {
            events,
            data: fd as u64,
        }
    }
    pub fn fd(&self) -> usize {
        self.data as usize
    }
}

pub fn epoll_create1() -> SysResult<usize> {
    let r = unsafe { syscall1(SYS_EPOLL_CREATE1, 0) };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}
pub fn epoll_add(epfd: usize, fd: usize, events: u32) -> SysResult<()> {
    let mut ev = EpollEvent::new(events, fd);
    let r = unsafe {
        syscall4(
            SYS_EPOLL_CTL,
            epfd,
            1, /*ADD*/
            fd,
            &mut ev as *mut _ as usize,
        )
    };
    if r >= 0 { Ok(()) } else { Err(r) }
}
pub fn epoll_wait(epfd: usize, events: &mut [EpollEvent], timeout_ms: isize) -> SysResult<usize> {
    let r = unsafe {
        syscall4(
            SYS_EPOLL_WAIT,
            epfd,
            events.as_mut_ptr() as usize,
            events.len(),
            timeout_ms as usize,
        )
    };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}
