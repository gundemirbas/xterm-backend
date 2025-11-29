#![allow(clippy::manual_c_str_literals)]

use crate::runtime::syscall::{
    syscall0_checked, syscall2_checked, syscall3_checked, syscall6_checked,
};
use crate::sys::SysResult;
use crate::sys::fs::open;

const SYS_FORK: usize = 57;
const SYS_EXECVE: usize = 59;
const SYS_EXIT: usize = 60;
const SYS_SETSID: usize = 112;
const SYS_DUP2: usize = 33;
const SYS_IOCTL: usize = 16;

const TIOCGPTN: usize = 0x80045430;
const TIOCSPTLCK: usize = 0x40045431;
const TIOCSCTTY: usize = 0x540E;
const TIOCSPGRP: usize = 0x5410;

pub fn open_ptmx() -> SysResult<usize> {
    open(b"/dev/ptmx\0".as_ptr(), 0o0002 /*O_RDWR*/, 0)
}
const SYS_PIPE2: usize = 293;

pub fn pipe2(flags: usize) -> SysResult<(usize, usize)> {
    let mut fds: [i32; 2] = [0, 0];
    let _ = syscall2_checked(SYS_PIPE2, fds.as_mut_ptr() as usize, flags)?;
    Ok((fds[0] as usize, fds[1] as usize))
}
pub fn pts_number(master_fd: usize) -> SysResult<u32> {
    let mut n: u32 = 0;
    let _ = syscall3_checked(SYS_IOCTL, master_fd, TIOCGPTN, &mut n as *mut _ as usize)?;
    Ok(n)
}
pub fn grantpt(master_fd: usize) -> SysResult<()> {
    let mut unlock: i32 = 0;
    let _ = syscall3_checked(
        SYS_IOCTL,
        master_fd,
        TIOCSPTLCK,
        &mut unlock as *mut _ as usize,
    )?;
    Ok(())
}
pub fn unlockpt(master_fd: usize) -> SysResult<()> {
    let mut unlock: i32 = 0;
    let _ = syscall3_checked(
        SYS_IOCTL,
        master_fd,
        TIOCSPTLCK,
        &mut unlock as *mut _ as usize,
    )?;
    Ok(())
}

pub fn open_pts(n: u32) -> SysResult<usize> {
    let base = b"/dev/pts/";
    let mut path = [0u8; 32];
    let mut idx = 0;
    for &b in base {
        path[idx] = b;
        idx += 1;
    }
    let mut tmp = n;
    let mut digits = [0u8; 10];
    let mut d = 0;
    if tmp == 0 {
        digits[0] = b'0';
        d = 1;
    }
    while tmp > 0 {
        digits[d] = b'0' + (tmp % 10) as u8;
        d += 1;
        tmp /= 10;
    }
    for j in (0..d).rev() {
        path[idx] = digits[j];
        idx += 1;
    }
    path[idx] = 0;
    open(path.as_ptr(), 0o0002, 0)
}

pub fn ioctl_set_ctty(fd: usize) -> SysResult<()> {
    let _ = syscall2_checked(SYS_IOCTL, fd, TIOCSCTTY)?;
    Ok(())
}

pub fn tcsetpgrp(fd: usize, pgrp: i32) -> SysResult<()> {
    let _ = syscall3_checked(SYS_IOCTL, fd, TIOCSPGRP, &pgrp as *const _ as usize)?;
    Ok(())
}

pub fn fork() -> SysResult<i32> {
    let r = syscall0_checked(SYS_FORK)?;
    Ok(r as i32)
}
pub fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> ! {
    let _ = crate::runtime::syscall::syscall3_checked(
        SYS_EXECVE,
        path as usize,
        argv as usize,
        envp as usize,
    );
    let _ = crate::runtime::syscall::syscall1_checked(SYS_EXIT, 127);
    loop {
        core::hint::spin_loop();
    }
}
#[allow(dead_code)]
pub fn exit(code: i32) -> ! {
    let _ = crate::runtime::syscall::syscall1_checked(SYS_EXIT, code as usize);
    loop {
        core::hint::spin_loop();
    }
}
pub fn setsid() -> SysResult<()> {
    let _ = syscall0_checked(SYS_SETSID)?;
    Ok(())
}
pub fn dup2(oldfd: usize, newfd: usize) -> SysResult<()> {
    let _ = syscall2_checked(SYS_DUP2, oldfd, newfd)?;
    Ok(())
}

const SYS_PRCTL: usize = 157;
const PR_SET_PDEATHSIG: usize = 1;

pub fn prctl_set_pdeathsig(sig: usize) -> SysResult<()> {
    let _ = syscall6_checked(SYS_PRCTL, PR_SET_PDEATHSIG, sig, 0, 0, 0, 0)?;
    Ok(())
}

const SYS_KILL: usize = 62;
const SYS_WAIT4: usize = 61;

pub fn kill(pid: i32, sig: i32) -> SysResult<()> {
    let _ = syscall2_checked(SYS_KILL, pid as usize, sig as usize)?;
    Ok(())
}

pub fn waitpid(pid: i32) -> SysResult<i32> {
    let mut status: i32 = 0;
    let r = crate::runtime::syscall::syscall4_checked(
        SYS_WAIT4,
        pid as usize,
        &mut status as *mut _ as usize,
        0,
        0,
    )?;
    Ok(r as i32)
}

pub fn waitpid_nohang(pid: i32) -> SysResult<i32> {
    let mut status: i32 = 0;
    const WNOHANG: usize = 1;
    let r = crate::runtime::syscall::syscall4_checked(
        SYS_WAIT4,
        pid as usize,
        &mut status as *mut _ as usize,
        WNOHANG,
        0,
    )?;
    Ok(r as i32)
}

pub fn wait_any_nohang() -> SysResult<i32> {
    let mut status: i32 = 0;
    const WNOHANG: usize = 1;
    // pid = -1 (wait for any child) -> pass usize::MAX
    let r = crate::runtime::syscall::syscall4_checked(
        SYS_WAIT4,
        !0usize,
        &mut status as *mut _ as usize,
        WNOHANG,
        0,
    )?;
    Ok(r as i32)
}
