use crate::net::ws;
use crate::sys::epoll;
use crate::sys::{fs, mmap, net, signal};

pub fn run(ws_fd: usize, pty_fd: usize) -> Result<(), &'static str> {
    let epfd = epoll::epoll_create1().map_err(|_| "epoll")?;
    // block SIGINT(2) and SIGTERM(15) for this thread, and create a signalfd
    let mut mask: u64 = 0;
    // signals are 1-indexed in the mask
    mask |= 1u64 << (2 - 1); // SIGINT
    mask |= 1u64 << (15 - 1); // SIGTERM
    let _ = signal::block_signals(&mask as *const u64, core::mem::size_of::<u64>());
    let sfd = match signal::signalfd(&mask as *const u64, core::mem::size_of::<u64>(), 0) {
        Ok(fd) => fd,
        Err(_) => usize::MAX,
    };
    epoll::epoll_add(epfd, ws_fd, epoll::EPOLLIN).map_err(|_| "epoll add ws")?;
    epoll::epoll_add(epfd, pty_fd, epoll::EPOLLIN).map_err(|_| "epoll add pty")?;

    let buf_len = 64 * 1024;
    let scratch_len = 64 * 1024;
    let buf_ptr = match mmap::mmap_alloc(buf_len) {
        Ok(p) => p,
        Err(_) => return Err("mmap buf"),
    };
    if buf_ptr.is_null() {
        return Err("mmap buf null");
    }
    let scratch_ptr = match mmap::mmap_alloc(scratch_len) {
        Ok(p) => p,
        Err(_) => {
            let _ = mmap::munmap_free(buf_ptr, buf_len);
            return Err("mmap scratch");
        }
    };
    if scratch_ptr.is_null() {
        let _ = mmap::munmap_free(buf_ptr, buf_len);
        return Err("mmap scratch null");
    }

    let mut events = [epoll::EpollEvent::default(); 32];
    let mut should_exit = false;
    let mut result: Result<(), &'static str> = Ok(());

    loop {
        let n = match epoll::epoll_wait(epfd, &mut events, -1) {
            Ok(v) => v,
            Err(_) => {
                result = Err("wait");
                break;
            }
        };
        for i in 0..n {
            let fd = events[i].fd();
            if fd == sfd {
                // signal received -> exit
                should_exit = true;
                break;
            }
            if fd == pty_fd {
                let r = match fs::read(pty_fd, unsafe {
                    core::slice::from_raw_parts_mut(buf_ptr, buf_len)
                }) {
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
                let slice = unsafe { core::slice::from_raw_parts(buf_ptr, r) };
                if let Err(_) = ws::write_binary_frame(ws_fd, slice) {
                    result = Err("ws write");
                    should_exit = true;
                    break;
                }
            } else if fd == ws_fd {
                let r = match net::recv(ws_fd, unsafe {
                    core::slice::from_raw_parts_mut(buf_ptr, buf_len)
                }) {
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
                let input = unsafe { core::slice::from_raw_parts(buf_ptr, r) };
                let out = unsafe { core::slice::from_raw_parts_mut(scratch_ptr, scratch_len) };
                match ws::parse_and_unmask_frames(input, out) {
                    Ok(payload) => {
                        let _ = fs::write(pty_fd, payload);
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

    let _ = mmap::munmap_free(buf_ptr, buf_len);
    let _ = mmap::munmap_free(scratch_ptr, scratch_len);
    if sfd != usize::MAX { let _ = fs::close(sfd); }

    result
}
