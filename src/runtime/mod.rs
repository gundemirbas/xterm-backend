pub mod allocator;
pub mod panic;
pub mod shims;
pub mod syscall;
pub mod util;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        core::arch::asm!(
            "and rsp, ~0xF",
            "call {main}",
            main = sym crate::main,
            options(noreturn)
        );
    }
}

#[inline(always)]
pub(crate) fn exit_now(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 60usize,
            in("rdi") code as usize,
            options(noreturn)
        );
    }
}
