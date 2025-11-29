#![no_std]
#![no_main]

mod r#loop;
mod net;
mod pty;
mod runtime;
mod sys;
mod server;

// 1MB stack
#[unsafe(link_section = ".bss")]
static mut STACK: [u8; 1024 * 1024] = [0; 1024 * 1024];

#[unsafe(no_mangle)]
#[allow(static_mut_refs)]
pub extern "C" fn _start() -> ! {
    unsafe {
        let stack_top = STACK.as_ptr().add(STACK.len()) as usize;
        // Align stack to 16 bytes and switch to it
        core::arch::asm!(
            "mov rsp, {stack}",
            "and rsp, ~0xF",
            "call {main}",
            stack = in(reg) stack_top,
            main = sym main_with_stack,
            options(noreturn)
        );
    }
}

#[allow(static_mut_refs)]
fn main_with_stack() -> ! {
    crate::server::log(b"start\n");
    crate::server::server_main();
    // server_main returned (e.g. due to signal/shutdown). Exit process so children get PDEATHSIG.
    crate::server::exit_now(0);
}
// `server::server_main`, `exit_now`, and logging helpers live in `src/server.rs`
// after the refactor. `main.rs` only contains the bootstrap sequence.
