pub mod allocator;
pub mod panic;
pub mod shims;
pub mod syscall;
pub mod util;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Align stack to 16 bytes and call `crate::main` so the ABI is satisfied.
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
pub fn exit_now(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 60usize, // SYS_exit
            in("rdi") code as usize,
            options(noreturn)
        );
    }
}
