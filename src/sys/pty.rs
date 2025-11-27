use crate::sys::{SysResult};
use crate::sys::fs::open;
use crate::sys::syscall::{syscall0, syscall1, syscall2, syscall3};
use crate::sys::fs::{write};

const SYS_FORK: usize = 57;
const SYS_EXECVE: usize = 59;
const SYS_EXIT: usize = 60;
const SYS_SETSID: usize = 112;
const SYS_DUP2: usize = 33;
const SYS_IOCTL: usize = 16;

const TIOCGPTN: usize = 0x80045430;
const TIOCSPTLCK: usize = 0x40045431;
const TIOCSCTTY: usize = 0x540E;

pub fn open_ptmx() -> SysResult<usize> {
    open(b"/dev/ptmx\0".as_ptr(), 0o0002 /*O_RDWR*/, 0)
}
pub fn pts_number(master_fd: usize) -> SysResult<u32> {
    let mut n: u32 = 0;
    let r = unsafe { syscall3(SYS_IOCTL, master_fd, TIOCGPTN, &mut n as *mut _ as usize) };
    if r >= 0 { Ok(n) } else { Err(r) }
}
pub fn grantpt(master_fd: usize) -> SysResult<()> {
    let mut unlock: i32 = 0;
    let r = unsafe { syscall3(SYS_IOCTL, master_fd, TIOCSPTLCK, &mut unlock as *mut _ as usize) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
pub fn unlockpt(master_fd: usize) -> SysResult<()> {
    let mut unlock: i32 = 0;
    let r = unsafe { syscall3(SYS_IOCTL, master_fd, TIOCSPTLCK, &mut unlock as *mut _ as usize) };
    if r >= 0 { Ok(()) } else { Err(r) }
}

pub fn open_pts(n: u32) -> SysResult<usize> {
    let base = b"/dev/pts/";
    let mut path = [0u8; 32];
    let mut idx = 0;
    for &b in base { path[idx] = b; idx += 1; }
    let mut tmp = n;
    let mut digits = [0u8; 10];
    let mut d = 0;
    if tmp == 0 { digits[0] = b'0'; d = 1; }
    while tmp > 0 {
        digits[d] = b'0' + (tmp % 10) as u8; d += 1; tmp /= 10;
    }
    for j in (0..d).rev() { path[idx] = digits[j]; idx += 1; }
    path[idx] = 0;
    open(path.as_ptr(), 0o0002, 0)
}

pub fn ioctl_set_ctty(fd: usize) -> SysResult<()> {
    let r = unsafe { syscall2(SYS_IOCTL, fd, TIOCSCTTY) };
    if r >= 0 { Ok(()) } else { Err(r) }
}

pub fn fork() -> SysResult<i32> {
    let r = unsafe { syscall0(SYS_FORK) };
    if r >= 0 { Ok(r as i32) } else { Err(r) }
}
pub fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> ! {
    unsafe { syscall3(SYS_EXECVE, path as usize, argv as usize, envp as usize); }
    exit(127);
}
pub fn exit(code: i32) -> ! {
    unsafe { syscall1(SYS_EXIT, code as usize); }
    loop {}
}
pub fn setsid() -> SysResult<()> {
    let r = unsafe { syscall0(SYS_SETSID) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
pub fn dup2(oldfd: usize, newfd: usize) -> SysResult<()> {
    let r = unsafe { syscall2(SYS_DUP2, oldfd, newfd) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
