use crate::runtime::syscall::{
    syscall2_checked, syscall3_checked, syscall4_checked, syscall6_checked,
};
use crate::sys::SysResult;

const SYS_SOCKET: usize = 41;
const SYS_BIND: usize = 49;
const SYS_LISTEN: usize = 50;
const SYS_ACCEPT4: usize = 288;
const SYS_SETSOCKOPT: usize = 54;
const SYS_SENDTO: usize = 44;
const SYS_RECVFROM: usize = 45;

pub const AF_INET: usize = 2;
pub const SOCK_STREAM: usize = 1;
pub const SOCK_CLOEXEC: usize = 524288;
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
    let r = syscall3_checked(SYS_SOCKET, domain, ty, proto)?;
    Ok(r as usize)
}
pub fn bind(fd: usize, addr: *const SockAddrIn, len: usize) -> SysResult<()> {
    let _ = syscall3_checked(SYS_BIND, fd, addr as usize, len)?;
    Ok(())
}
pub fn listen(fd: usize, backlog: usize) -> SysResult<()> {
    let _ = syscall2_checked(SYS_LISTEN, fd, backlog)?;
    Ok(())
}
pub fn setsockopt(fd: usize, lvl: usize, opt: usize, val: *const u8, len: usize) -> SysResult<()> {
    let _ = syscall6_checked(SYS_SETSOCKOPT, fd, lvl, opt, val as usize, len, 0)?;
    Ok(())
}
pub fn accept_blocking(fd: usize) -> SysResult<(usize, u32)> {
    let r = syscall4_checked(
        SYS_ACCEPT4,
        fd,
        core::ptr::null_mut::<u8>() as usize,
        0,
        SOCK_CLOEXEC,
    )?;
    Ok((r as usize, 0))
}
pub fn send_all(fd: usize, buf: &[u8]) -> SysResult<()> {
    let mut off = 0;
    while off < buf.len() {
        let remaining = &buf[off..];
        let r = syscall6_checked(
            SYS_SENDTO,
            fd,
            remaining.as_ptr() as usize,
            remaining.len(),
            0,
            0,
            0,
        )?;
        if r == 0 {
            break;
        }
        off += r as usize;
    }
    Ok(())
}
pub fn recv(fd: usize, buf: &mut [u8]) -> SysResult<usize> {
    let r = syscall6_checked(
        SYS_RECVFROM,
        fd,
        buf.as_mut_ptr() as usize,
        buf.len(),
        0,
        0,
        0,
    )?;
    Ok(r as usize)
}

pub fn tcp_listen(port: u16) -> SysResult<usize> {
    let fd = socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0)?;
    let one: i32 = 1;
    setsockopt(
        fd,
        SOL_SOCKET,
        SO_REUSEADDR,
        &one as *const _ as *const u8,
        core::mem::size_of::<i32>(),
    )?;
    let addr = SockAddrIn {
        sin_family: AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: u32::from_be_bytes([0, 0, 0, 0]),
        sin_zero: [0; 8],
    };
    bind(fd, &addr as *const _, core::mem::size_of::<SockAddrIn>())?;
    listen(fd, 128)?;
    const SYS_FCNTL: usize = 72;
    const F_SETFD: usize = 2;
    const FD_CLOEXEC: usize = 1;
    let _ = syscall3_checked(SYS_FCNTL, fd, F_SETFD, FD_CLOEXEC)?;
    Ok(fd)
}
