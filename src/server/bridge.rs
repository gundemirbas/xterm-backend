use crate::runtime::util;
use crate::sys;

pub(crate) fn run_bridge(ws_fd: usize, pty_fd: usize, child_pid: i32) -> Result<(), &'static str> {
    let epfd = sys::epoll::epoll_create1().map_err(|_| "epoll")?;
    let mut mask: u64 = 0;
    mask |= 1u64 << (2 - 1);
    mask |= 1u64 << (15 - 1);
    let _ = sys::signal::block_signals(&mask as *const u64, core::mem::size_of::<u64>());
    let sfd = match sys::signal::signalfd(&mask as *const u64, core::mem::size_of::<u64>(), 0) {
        Ok(fd) => fd,
        Err(_) => usize::MAX,
    };
    sys::epoll::epoll_add(epfd, ws_fd, sys::epoll::EPOLLIN).map_err(|_| "epoll add ws")?;
    sys::epoll::epoll_add(epfd, pty_fd, sys::epoll::EPOLLIN).map_err(|_| "epoll add pty")?;
    if sfd != usize::MAX {
        sys::epoll::epoll_add(epfd, sfd, sys::epoll::EPOLLIN).map_err(|_| "epoll add signalfd")?;
    }

    let buf_len = 64 * 1024;
    let scratch_len = 64 * 1024;
    let buf_ptr = match crate::runtime::allocator::page_alloc(buf_len) {
        Ok(p) => p,
        Err(_) => return Err("mmap buf"),
    };
    if buf_ptr.is_null() {
        return Err("mmap buf null");
    }
    let scratch_ptr = match crate::runtime::allocator::page_alloc(scratch_len) {
        Ok(p) => p,
        Err(_) => {
            let _ = crate::runtime::allocator::page_free(buf_ptr, buf_len);
            return Err("mmap scratch");
        }
    };
    if scratch_ptr.is_null() {
        let _ = crate::runtime::allocator::page_free(buf_ptr, buf_len);
        return Err("mmap scratch null");
    }

    let mut events = [sys::epoll::EpollEvent::default(); 32];
    let mut should_exit = false;
    let mut result: Result<(), &'static str> = Ok(());

    loop {
        let n = match sys::epoll::epoll_wait(epfd, &mut events, -1) {
            Ok(v) => v,
            Err(_) => {
                result = Err("wait");
                break;
            }
        };
        for event in events.iter().take(n) {
            let fd = event.fd();
            if fd == sfd {
                let _ = crate::sys::pty::kill(child_pid, 2);
                should_exit = true;
                break;
            }
            if fd == pty_fd {
                let r = match sys::fs::read(pty_fd, util::ptr_to_mut_slice(buf_ptr, buf_len)) {
                    Ok(v) => v,
                    Err(_) => {
                        result = Err("pty read");
                        should_exit = true;
                        break;
                    }
                };
                if r == 0 {
                    should_exit = true;
                    break;
                }
                let slice = util::ptr_to_slice(buf_ptr, r);
                if crate::net::ws::write_binary_frame(ws_fd, slice).is_err() {
                    result = Err("ws write");
                    should_exit = true;
                    break;
                }
            } else if fd == ws_fd {
                let r = match sys::net::recv(ws_fd, util::ptr_to_mut_slice(buf_ptr, buf_len)) {
                    Ok(v) => v,
                    Err(_) => {
                        result = Err("ws read");
                        should_exit = true;
                        break;
                    }
                };
                if r == 0 {
                    should_exit = true;
                    break;
                }
                let input = util::ptr_to_slice(buf_ptr, r);
                let out = util::ptr_to_mut_slice(scratch_ptr, scratch_len);
                match crate::net::ws::parse_and_unmask_frames(input, out) {
                    Ok(payload) => {
                        let mut saw_sigint = false;
                        for &b in payload {
                            if b == 0x03 {
                                saw_sigint = true;
                                break;
                            }
                        }
                        if saw_sigint {
                            let _ = crate::sys::pty::kill(child_pid, 2);
                        } else {
                            let _ = sys::fs::write(pty_fd, payload);
                        }
                    }
                    Err("close") => {
                        should_exit = true;
                        break;
                    }
                    Err(_) => {}
                }
            }
        }
        if should_exit {
            break;
        }
    }

    let _ = crate::runtime::allocator::page_free(buf_ptr, buf_len);
    let _ = crate::runtime::allocator::page_free(scratch_ptr, scratch_len);
    if sfd != usize::MAX {
        let _ = sys::fs::close(sfd);
    }

    result
}
