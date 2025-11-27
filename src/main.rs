#![no_std]
#![no_main]

mod r#loop;
mod net;
mod pty;
mod runtime;
mod sys;

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
    // server_main returned (e.g. due to signal/shutdown). Exit process so children get PDEATHSIG.
    exit_now(0);
}

fn server_main() {
    log(b"listen begin\n");
    let listen_fd = match sys::net::tcp_listen(8000) {
        Ok(fd) => fd,
        Err(e) => {
            log(b"listen failed with errno: ");
            log_num(-e as i32);
            log(b"\n");
            exit_now(1);
        }
    };
    log(b"listen ok\n");
    // Use epoll + signalfd for accept loop so we can gracefully shutdown
    let epfd = match sys::epoll::epoll_create1() {
        Ok(e) => e,
        Err(_) => { log(b"epoll create failed\n"); exit_now(1); }
    };
    let _ = sys::epoll::epoll_add(epfd, listen_fd, sys::epoll::EPOLLIN);
    let mut mask: u64 = 0;
    mask |= 1u64 << (2 - 1); // SIGINT
    mask |= 1u64 << (15 - 1); // SIGTERM
    mask |= 1u64 << (17 - 1); // SIGCHLD
    let _ = sys::signal::block_signals(&mask as *const u64, core::mem::size_of::<u64>());
    let sfd = match sys::signal::signalfd(&mask as *const u64, core::mem::size_of::<u64>(), 0) {
        Ok(fd) => fd,
        Err(_) => usize::MAX,
    };
    if sfd != usize::MAX {
        let _ = sys::epoll::epoll_add(epfd, sfd, sys::epoll::EPOLLIN);
    }

    let mut events = [sys::epoll::EpollEvent::default(); 8];
    let mut active_workers: i32 = 0;
    const MAX_WORKERS: i32 = 15;
    loop {
        log(b"accept wait\n");
        let n = match sys::epoll::epoll_wait(epfd, &mut events, -1) {
            Ok(v) => v,
            Err(e) => {
                log(b"epoll wait failed errno: ");
                log_num(-e as i32);
                log(b"\n");
                // continue accept loop on transient epoll errors
                continue;
            }
        };
        let mut shutdown = false;
        for i in 0..n {
            let fd = events[i].fd();
            if fd == sfd {
                // read signalfd_siginfo (we only need signo at u32 offset 0)
                let mut info = [0u8; 128];
                if let Ok(r) = sys::fs::read(sfd, &mut info) {
                    if r >= 4 {
                        let signo = (info[0] as u32) | ((info[1] as u32) << 8) | ((info[2] as u32) << 16) | ((info[3] as u32) << 24);
                        if signo == 17u32 { // SIGCHLD
                            // reap any dead children
                            loop {
                                match crate::sys::pty::wait_any_nohang() {
                                    Ok(0) => break, // no more
                                    Ok(pid) if pid > 0 => {
                                        active_workers -= 1;
                                        let _ = crate::sys::fs::write(1, b"main: reaped worker pid ");
                                        let mut nb = itoa::Buffer::new();
                                        let s = nb.format(pid as i64);
                                        let _ = crate::sys::fs::write(1, s.as_bytes());
                                        let _ = crate::sys::fs::write(1, b"\n");
                                        continue;
                                    }
                                    Err(_) => break,
                                    _ => break,
                                }
                            }
                        } else if signo == 2u32 || signo == 15u32 {
                            log(b"main: signalfd readable (signal received)\n");
                            shutdown = true;
                            break;
                        }
                    }
                }
                continue;
            }
            if fd == listen_fd {
                let (fd2, _) = match sys::net::accept_blocking(listen_fd) {
                    Ok(v) => v,
                    Err(_) => { log(b"accept failed\n"); continue; }
                };
                let fd = fd2;
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
                    // Enforce max worker limit
                    if active_workers >= MAX_WORKERS {
                        let _ = crate::sys::fs::write(1, b"main: max workers reached, refusing connection\n");
                        // send minimal HTTP 503 response
                        let _ = crate::sys::fs::write(fd, b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n");
                        let _ = sys::fs::close(fd);
                        continue;
                    }

                    match crate::sys::pty::fork() {
                        Err(_) => {
                            log(b"fork failed\n");
                            let _ = sys::fs::close(fd);
                            continue;
                        }
                        Ok(p) if p > 0 => {
                            // parent: p is child pid
                            active_workers += 1;
                            let _ = crate::sys::fs::write(1, b"main: forked worker pid ");
                            let mut nb = itoa::Buffer::new();
                            let s = nb.format(p as i64);
                            let _ = crate::sys::fs::write(1, s.as_bytes());
                            let _ = crate::sys::fs::write(1, b"\n");
                            // parent should close accepted fd and continue accepting
                            let _ = sys::fs::close(fd);
                            continue;
                        }
                        Ok(0) => {
                            // child: close listen/epoll/signalfd to avoid inheriting them
                            let _ = sys::fs::close(listen_fd);
                            if sfd != usize::MAX { let _ = sys::fs::close(sfd); }
                            let _ = sys::fs::close(epfd);
                            // child continues below to perform upgrade and run bridge
                        }
                        _ => {
                            let _ = sys::fs::close(fd);
                            continue;
                        }
                    }
                    // Child path continues here (after fork returned 0)
                    match net::ws::upgrade_to_websocket(fd, &buf[..n]) {
                        Ok(ws) => {
                            log(b"ws upgrade ok\n");
                            match pty::pty::spawn_sh() {
                                Ok(p) => {
                                    log(b"pty spawn ok\n");
                                    // bridge::run will log detailed errors itself; avoid duplicate noisy logging here
                                    let _ = r#loop::bridge::run(ws.fd, p.master_fd, p.child_pid);
                                    // ensure child shell is terminated and reaped
                                    let mut buf = [0u8; 64];
                                    let mut idx = 0usize;
                                    let pre = b"main: killing child pid ";
                                    for &b in pre { buf[idx]=b; idx+=1; }
                                    let mut itoa_buf = itoa::Buffer::new();
                                    let pid_s = itoa_buf.format(p.child_pid as i64);
                                    for &b in pid_s.as_bytes() { buf[idx]=b; idx+=1; }
                                    buf[idx]=b'\n'; idx+=1;
                                    let _ = crate::sys::fs::write(1, &buf[..idx]);

                                    // Try SIGTERM first and blockingly wait for child to exit.
                                    match crate::sys::pty::kill(p.child_pid, 15) {
                                        Ok(()) => {
                                            let _ = crate::sys::fs::write(1, b"main: sent SIGTERM to child\n");
                                            match crate::sys::pty::waitpid(p.child_pid) {
                                                Ok(w) => {
                                                    let mut ob = [0u8; 64];
                                                    let mut oi = 0usize;
                                                    let pre2 = b"main: waitpid returned ";
                                                    for &b in pre2 { ob[oi]=b; oi+=1; }
                                                    let mut nb = itoa::Buffer::new();
                                                    let s = nb.format(w as i64);
                                                    for &b in s.as_bytes() { ob[oi]=b; oi+=1; }
                                                    ob[oi]=b'\n'; oi+=1;
                                                    let _ = crate::sys::fs::write(1, &ob[..oi]);
                                                }
                                                Err(_) => {
                                                    // If waitpid failed, try to escalate
                                                    let _ = crate::sys::fs::write(1, b"main: waitpid failed after SIGTERM, escalating to SIGKILL\n");
                                                    match crate::sys::pty::kill(p.child_pid, 9) {
                                                        Ok(()) => { let _ = crate::sys::fs::write(1, b"main: sent SIGKILL to child\n"); }
                                                        Err(e) => {
                                                            let _ = crate::sys::fs::write(1, b"main: kill(SIGKILL) failed errno: ");
                                                            log_num(-e as i32);
                                                            let _ = crate::sys::fs::write(1, b"\n");
                                                        }
                                                    }
                                                    // attempt non-blocking reap
                                                    match crate::sys::pty::waitpid_nohang(p.child_pid) {
                                                        Ok(0) => { let _ = crate::sys::fs::write(1, b"main: child still alive after SIGKILL\n"); }
                                                        Ok(w2) => {
                                                            let mut it = itoa::Buffer::new();
                                                            let s2 = it.format(w2 as i64);
                                                            let mut ob = [0u8; 64];
                                                            let mut oi = 0usize;
                                                            let pre3 = b"main: waitpid returned ";
                                                            for &b in pre3 { ob[oi]=b; oi+=1; }
                                                            for &b in s2.as_bytes() { ob[oi]=b; oi+=1; }
                                                            ob[oi]=b'\n'; oi+=1;
                                                            let _ = crate::sys::fs::write(1, &ob[..oi]);
                                                        }
                                                        Err(e) => {
                                                            let _ = crate::sys::fs::write(1, b"main: waitpid_nohang failed after SIGKILL errno: ");
                                                            log_num(-e as i32);
                                                            let _ = crate::sys::fs::write(1, b"\n");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = crate::sys::fs::write(1, b"main: kill(SIGTERM) failed errno: ");
                                            log_num(-e as i32);
                                            let _ = crate::sys::fs::write(1, b"\n");
                                            // try SIGKILL immediately
                                            match crate::sys::pty::kill(p.child_pid, 9) {
                                                Ok(()) => { let _ = crate::sys::fs::write(1, b"main: sent SIGKILL to child\n"); }
                                                Err(e2) => {
                                                    let _ = crate::sys::fs::write(1, b"main: kill(SIGKILL) failed errno: ");
                                                    log_num(-e2 as i32);
                                                    let _ = crate::sys::fs::write(1, b"\n");
                                                }
                                            }
                                            let _ = crate::sys::pty::waitpid_nohang(p.child_pid);
                                        }
                                    }
                                    let _ = sys::fs::close(p.master_fd);
                                    // child must exit now so it doesn't re-enter accept loop
                                    exit_now(0);
                                }
                                Err(_) => {
                                    log(b"pty spawn failed\n");
                                    let _ = sys::fs::close(ws.fd);
                                    exit_now(1);
                                }
                            }
                        }
                        Err(_) => {
                            log(b"ws upgrade failed\n");
                            let _ = sys::fs::close(fd);
                            exit_now(1);
                        }
                    }
                } else {
                    log(b"http serve html\n");
                    http::serve_html(fd, INDEX_HTML);
                    log(b"http served\n");
                }
            }
        }
        if shutdown { break; }
    }
    let _ = sys::fs::close(listen_fd);
    if sfd != usize::MAX { let _ = sys::fs::close(sfd); }
}

#[inline(always)]
fn log(msg: &[u8]) {
    let _ = sys::fs::write(1, msg);
}

#[inline(always)]
fn log_num(mut n: i32) {
    if n == 0 {
        log(b"0");
        return;
    }
    if n < 0 {
        log(b"-");
        n = -n;
    }
    let mut buf = [0u8; 16];
    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        let _ = sys::fs::write(1, &buf[i..i + 1]);
    }
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
