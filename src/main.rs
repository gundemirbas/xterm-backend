#![no_std]
#![no_main]

mod net;
mod pty;
mod runtime;
mod server;
mod sys;

fn main() -> ! {
    let (listen_fd, epfd, sfd) = crate::server::setup_listener();

    let mut events = [crate::sys::epoll::EpollEvent::default(); 8];
    let mut active_workers: i32 = 0;
    const MAX_WORKERS: i32 = 15;
    loop {
        let n = match crate::sys::epoll::epoll_wait(epfd, &mut events, -1) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let mut shutdown = false;
        for event in events.iter().take(n) {
            let fd = event.fd();
            if fd == sfd {
                if crate::server::handle_signal_event(sfd, &mut active_workers) {
                    shutdown = true;
                    break;
                }
                continue;
            }
            if fd == listen_fd
                && crate::server::handle_listener_event(
                    listen_fd,
                    &mut active_workers,
                    MAX_WORKERS,
                    sfd,
                    epfd,
                )
                .is_err()
            {
                // errors are logged inside handler; continue accepting
                continue;
            }
        }
        if shutdown {
            break;
        }
    }
    let _ = crate::sys::fs::close(listen_fd);
    if sfd != usize::MAX {
        let _ = crate::sys::fs::close(sfd);
    }
    crate::server::exit_now(0)
}
