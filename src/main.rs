#![no_std]
#![no_main]

mod runtime;
mod sys;
mod net;
mod pty;
mod r#loop;

use net::http;

static INDEX_HTML: &[u8] = include_bytes!("../assets/terminal.html");

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
    log(b"start\n");
    server_main();
    loop {}
}

fn server_main() {
    log(b"listen begin\n");
    let listen_fd = match sys::net::tcp_listen(8000) {
        Ok(fd) => fd,
        Err(_) => { log(b"listen failed\n"); exit_now(1); }
    };
    log(b"listen ok\n");

    loop {
        log(b"accept wait\n");
        let (fd, _) = match sys::net::accept_blocking(listen_fd) {
            Ok(v) => v,
            Err(_) => { log(b"accept failed\n"); continue; }
        };
        log(b"accepted\n");

        let mut buf = [0u8; 8192];
        let n = match sys::net::recv(fd, &mut buf) {
            Ok(n) => n,
            Err(_) => { log(b"recv failed\n"); let _ = sys::fs::close(fd); continue; }
        };
        log(b"recv ok\n");

        if net::http::is_websocket_upgrade(&buf[..n]) {
            log(b"http says upgrade\n");
        } else {
            log(b"http no upgrade\n");
        }

        if net::http::path_is_term(&buf[..n]) {
            log(b"path /term\n");
        } else {
            log(b"path not /term\n");
        }

        if net::http::is_websocket_upgrade(&buf[..n]) && net::http::path_is_term(&buf[..n]) {
            log(b"ws upgrade try\n");
            match net::ws::upgrade_to_websocket(fd, &buf[..n]) {
                Ok(ws) => {
                    log(b"ws upgrade ok\n");
                    match pty::pty::spawn_sh() {
                        Ok(p) => {
                            log(b"pty spawn ok\n");
                            match r#loop::bridge::run(ws.fd, p.master_fd) {
                                Ok(_) => log(b"bridge exit ok\n"),
                                Err(_) => log(b"bridge err\n"),
                            }
                        }
                        Err(_) => {
                            log(b"pty spawn failed\n");
                            let _ = sys::fs::close(ws.fd);
                        }
                    }
                }
                Err(_) => {
                    log(b"ws upgrade failed\n");
                    let _ = sys::fs::close(fd);
                }
            }
        } else {
            log(b"http serve html\n");
            http::serve_html(fd, INDEX_HTML);
            log(b"http served\n");
        }
    }
}

#[inline(always)]
fn log(msg: &[u8]) {
    let _ = sys::fs::write(1, msg);
}

#[inline(always)]
fn exit_now(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 60usize, // SYS_exit
            in("rdi") code as usize,
            lateout("rax") _, clobber_abi("sysv64")
        );
    }
    loop {}
}
