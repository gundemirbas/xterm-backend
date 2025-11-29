//! Low-level Linux syscall wrappers using inline assembly.
//!
//! # Safety
//! All functions are unsafe because they directly invoke kernel syscalls.
//! Callers must ensure:
//! - Syscall numbers are valid for the target kernel
//! - Arguments match the syscall ABI (type, ownership, lifetimes)
//! - Pointers reference valid memory for the duration of the syscall

use crate::sys::SysResult;
use core::arch::asm;

/// # Safety
/// Caller must ensure syscall number `n` is valid and requires no arguments.
pub(crate) unsafe fn syscall0(n: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
/// # Safety
/// Caller must ensure syscall and argument are valid per kernel ABI.
pub(crate) unsafe fn syscall1(n: usize, a0: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
/// # Safety
/// Caller must ensure syscall and arguments are valid per kernel ABI.
pub(crate) unsafe fn syscall2(n: usize, a0: usize, a1: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, in("rsi") a1, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
/// # Safety
/// Caller must ensure syscall and arguments are valid per kernel ABI.
pub(crate) unsafe fn syscall3(n: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, in("rsi") a1, in("rdx") a2, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
/// # Safety
/// Caller must ensure syscall and arguments are valid per kernel ABI.
pub(crate) unsafe fn syscall4(n: usize, a0: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, in("rsi") a1, in("rdx") a2, in("r10") a3, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
/// # Safety
/// Caller must ensure syscall and arguments are valid per kernel ABI.
pub(crate) unsafe fn syscall6(
    n: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall",
            in("rax") n, in("rdi") a0, in("rsi") a1, in("rdx") a2, in("r10") a3, in("r8") a4, in("r9") a5,
            lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}

// Convenience checked wrappers that centralize the common "call syscall
// and convert negative return values into `Err(isize)`" pattern. These
// keep the `unsafe` inline-assembly in one place and allow callers to use
// a safe API surface. They return `SysResult<isize>` so callers that need
// to interpret positive values can do so.
pub fn syscall0_checked(n: usize) -> SysResult<isize> {
    let r = unsafe { syscall0(n) };
    if r >= 0 { Ok(r) } else { Err(r) }
}
pub fn syscall1_checked(n: usize, a0: usize) -> SysResult<isize> {
    let r = unsafe { syscall1(n, a0) };
    if r >= 0 { Ok(r) } else { Err(r) }
}
pub fn syscall2_checked(n: usize, a0: usize, a1: usize) -> SysResult<isize> {
    let r = unsafe { syscall2(n, a0, a1) };
    if r >= 0 { Ok(r) } else { Err(r) }
}
pub fn syscall3_checked(n: usize, a0: usize, a1: usize, a2: usize) -> SysResult<isize> {
    let r = unsafe { syscall3(n, a0, a1, a2) };
    if r >= 0 { Ok(r) } else { Err(r) }
}
pub fn syscall4_checked(
    n: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
) -> SysResult<isize> {
    let r = unsafe { syscall4(n, a0, a1, a2, a3) };
    if r >= 0 { Ok(r) } else { Err(r) }
}
pub fn syscall6_checked(
    n: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> SysResult<isize> {
    let r = unsafe { syscall6(n, a0, a1, a2, a3, a4, a5) };
    if r >= 0 { Ok(r) } else { Err(r) }
}
