use core::arch::asm;

pub(crate) unsafe fn syscall0(n: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
pub(crate) unsafe fn syscall1(n: usize, a0: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
pub(crate) unsafe fn syscall2(n: usize, a0: usize, a1: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, in("rsi") a1, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
pub(crate) unsafe fn syscall3(n: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, in("rsi") a1, in("rdx") a2, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
pub(crate) unsafe fn syscall4(n: usize, a0: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let r: isize;
    unsafe {
        asm!("syscall", in("rax") n, in("rdi") a0, in("rsi") a1, in("rdx") a2, in("r10") a3, lateout("rax") r, clobber_abi("sysv64"));
    }
    r
}
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
