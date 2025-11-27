use crate::sys::SysResult;
use crate::sys::syscall::{syscall2, syscall3, syscall4, syscall6};

const SYS_SOCKET: usize = 41;
const SYS_BIND: usize = 49;
const SYS_LISTEN: usize = 50;
const SYS_ACCEPT4: usize = 288;
const SYS_SETSOCKOPT: usize = 54;
const SYS_SENDTO: usize = 44;
const SYS_RECVFROM: usize = 45;

pub const AF_INET: usize = 2;
pub const SOCK_STREAM: usize = 1;
pub const SOL_SOCKET: usize = 1;
pub const SO_REUSEADDR: usize = 2;

#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

pub fn socket(domain: usize, ty: usize, proto: usize) -> SysResult<usize> {
    let r = unsafe { syscall3(SYS_SOCKET, domain, ty, proto) };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}
pub fn bind(fd: usize, addr: *const SockAddrIn, len: usize) -> SysResult<()> {
    let r = unsafe { syscall3(SYS_BIND, fd, addr as usize, len) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
pub fn listen(fd: usize, backlog: usize) -> SysResult<()> {
    let r = unsafe { syscall2(SYS_LISTEN, fd, backlog) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
pub fn setsockopt(fd: usize, lvl: usize, opt: usize, val: *const u8, len: usize) -> SysResult<()> {
    let r = unsafe { syscall6(SYS_SETSOCKOPT, fd, lvl, opt, val as usize, len, 0) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
pub fn accept_blocking(fd: usize) -> SysResult<(usize, u32)> {
    let r = unsafe { syscall4(SYS_ACCEPT4, fd, core::ptr::null_mut::<u8>() as usize, 0, 0) };
    if r >= 0 { Ok((r as usize, 0)) } else { Err(r) }
}
pub fn send_all(fd: usize, buf: &[u8]) -> SysResult<()> {
    let mut off = 0;
    while off < buf.len() {
        let remaining = &buf[off..];
        let r = unsafe {
            syscall6(
                SYS_SENDTO,
                fd,
                remaining.as_ptr() as usize,
                remaining.len(),
                0,
                0,
                0,
            )
        };
        if r < 0 {
            return Err(r);
        }
        if r == 0 {
            break;
        }
        off += r as usize;
    }
    Ok(())
}
pub fn recv(fd: usize, buf: &mut [u8]) -> SysResult<usize> {
    let r = unsafe {
        syscall6(
            SYS_RECVFROM,
            fd,
            buf.as_mut_ptr() as usize,
            buf.len(),
            0,
            0,
            0,
        )
    };
    if r >= 0 { Ok(r as usize) } else { Err(r) }
}

pub fn tcp_listen(port: u16) -> SysResult<usize> {
    let fd = socket(AF_INET, SOCK_STREAM, 0)?;
    let one: i32 = 1;
    let _ = setsockopt(
        fd,
        SOL_SOCKET,
        SO_REUSEADDR,
        &one as *const _ as *const u8,
        core::mem::size_of::<i32>(),
    )?;
    let addr = SockAddrIn {
        sin_family: AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: u32::from_be_bytes([0, 0, 0, 0]), // INADDR_ANY
        sin_zero: [0; 8],
    };
    bind(fd, &addr as *const _, core::mem::size_of::<SockAddrIn>())?;
    listen(fd, 128)?;
    Ok(fd)
}
