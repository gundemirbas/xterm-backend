#![no_std]
#![no_main]

mod r#loop;
mod net;
mod pty;
mod runtime;
mod server;
mod sys;

fn main() -> ! {
    crate::server::log(b"start\n");
    crate::server::server_main();
    // server_main returned (e.g. due to signal/shutdown). Exit process so children get PDEATHSIG.
    crate::server::exit_now(0);
}
// `server::server_main`, `exit_now`, and logging helpers live in `src/server.rs`
// after the refactor. `main.rs` only contains the bootstrap sequence.
