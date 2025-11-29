use crate::net;
use crate::pty;
use crate::sys;
mod bridge;
pub static INDEX_HTML: &[u8] = include_bytes!("../../assets/terminal.html");

pub(crate) fn setup_listener() -> (usize, usize, usize) {
    let listen_fd = match sys::net::tcp_listen(8000) {
        Ok(fd) => fd,
        Err(e) => {
            log(b"listen failed with errno: ");
            log_num(-e as i32);
            log(b"\n");
            exit_now(1);
        }
    };
    let epfd = match sys::epoll::epoll_create1() {
        Ok(e) => e,
        Err(_) => {
            log(b"epoll create failed\n");
            exit_now(1);
        }
    };
    let _ = sys::epoll::epoll_add(epfd, listen_fd, sys::epoll::EPOLLIN);
    let mut mask: u64 = 0;
    mask |= 1u64 << (2 - 1);
    mask |= 1u64 << (15 - 1);
    mask |= 1u64 << (17 - 1);
    let _ = sys::signal::block_signals(&mask as *const u64, core::mem::size_of::<u64>());
    let sfd = match sys::signal::signalfd(&mask as *const u64, core::mem::size_of::<u64>(), 0) {
        Ok(fd) => fd,
        Err(_) => usize::MAX,
    };
    if sfd != usize::MAX {
        let _ = sys::epoll::epoll_add(epfd, sfd, sys::epoll::EPOLLIN);
    }
    (listen_fd, epfd, sfd)
}

pub(crate) fn handle_signal_event(sfd: usize, active_workers: &mut i32) -> bool {
    let mut info = [0u8; 128];
    if let Ok(r) = sys::fs::read(sfd, &mut info)
        && r >= 4
    {
        let signo = (info[0] as u32)
            | ((info[1] as u32) << 8)
            | ((info[2] as u32) << 16)
            | ((info[3] as u32) << 24);
        if signo == 17u32 {
            loop {
                match crate::sys::pty::wait_any_nohang() {
                    Ok(0) => break,
                    Ok(pid) if pid > 0 => {
                        *active_workers -= 1;
                        continue;
                    }
                    Err(_) => break,
                    _ => break,
                }
            }
            return false;
        } else if signo == 2u32 || signo == 15u32 {
            return true;
        }
    }
    false
}

pub(crate) fn handle_listener_event(
    listen_fd: usize,
    active_workers: &mut i32,
    max_workers: i32,
    sfd: usize,
    epfd: usize,
) -> Result<(), &'static str> {
    let (fd2, _) = sys::net::accept_blocking(listen_fd).map_err(|_| "accept")?;
    let fd = fd2;

    let mut buf = [0u8; 8192];
    let n = sys::net::recv(fd, &mut buf).map_err(|_| {
        let _ = sys::fs::close(fd);
        "recv"
    })?;

    if net::http::is_websocket_upgrade(&buf[..n]) && net::http::path_is_term(&buf[..n]) {
        if *active_workers >= max_workers {
            let _ = crate::sys::fs::write(
                fd,
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n",
            );
            let _ = sys::fs::close(fd);
            return Ok(());
        }

        match crate::sys::pty::fork() {
            Err(_) => {
                log(b"fork failed\n");
                let _ = sys::fs::close(fd);
                return Err("fork");
            }
            Ok(p) if p > 0 => {
                *active_workers += 1;
                let _ = sys::fs::close(fd);
                return Ok(());
            }
            Ok(0) => {
                let _ = sys::fs::close(listen_fd);
                if sfd != usize::MAX {
                    let _ = sys::fs::close(sfd);
                }
                let _ = sys::fs::close(epfd);
            }
            _ => {
                let _ = sys::fs::close(fd);
                return Err("fork-other");
            }
        }

        match net::ws::upgrade_to_websocket(fd, &buf[..n]) {
            Ok(ws) => match pty::spawn_sh() {
                Ok(p) => {
                    let _ = bridge::run_bridge(ws.fd, p.master_fd, p.child_pid);
                    let _ = crate::sys::pty::kill(p.child_pid, 15);
                    if crate::sys::pty::waitpid(p.child_pid).is_err() {
                        let _ = crate::sys::pty::kill(p.child_pid, 9);
                        let _ = crate::sys::pty::waitpid_nohang(p.child_pid);
                    }
                    let _ = sys::fs::close(p.master_fd);
                    exit_now(0);
                }
                Err(_) => {
                    log(b"pty spawn failed\n");
                    let _ = sys::fs::close(ws.fd);
                    exit_now(1);
                }
            },
            Err(_) => {
                log(b"ws upgrade failed\n");
                let _ = sys::fs::close(fd);
                exit_now(1);
            }
        }
    } else {
        net::http::serve_html(fd, INDEX_HTML);
    }
    Ok(())
}

#[inline(always)]
pub(crate) fn log(msg: &[u8]) {
    let _ = sys::fs::write(1, msg);
}

#[inline(always)]
pub(crate) fn log_num(mut n: i32) {
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
pub(crate) fn exit_now(code: i32) -> ! {
    crate::runtime::exit_now(code)
}
