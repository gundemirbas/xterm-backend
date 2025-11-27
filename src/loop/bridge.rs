use crate::net::ws;
use crate::sys::epoll;
use crate::sys::{fs, mmap, net, signal};

// Helpers to centralize the few unchecked pointer->slice conversions.
// These wrap `unsafe` calls in one place so the rest of the module can
// remain free of inline `unsafe` blocks. Callers must ensure the mmap'd
// memory remains valid for the duration of the slice usage.
fn as_mut_slice<'a>(ptr: *mut u8, len: usize) -> &'a mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(ptr, len) }
}
fn as_slice<'a>(ptr: *mut u8, len: usize) -> &'a [u8] {
    unsafe { core::slice::from_raw_parts(ptr, len) }
}

pub fn run(ws_fd: usize, pty_fd: usize, child_pid: i32) -> Result<(), &'static str> {
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
    if sfd != usize::MAX {
        epoll::epoll_add(epfd, sfd, epoll::EPOLLIN).map_err(|_| "epoll add signalfd")?;
    }

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
        for event in events.iter().take(n) {
            let fd = event.fd();
            if fd == sfd {
                let _ = crate::sys::pty::kill(child_pid, 2); // SIGINT
                should_exit = true;
                break;
            }
            if fd == pty_fd {
                // buf_ptr is mmap'd memory of `buf_len` bytes
                let r = match fs::read(pty_fd, as_mut_slice(buf_ptr, buf_len)) {
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
                // buf_ptr is valid for `r` bytes as returned by read()
                let slice = as_slice(buf_ptr, r);
                if ws::write_binary_frame(ws_fd, slice).is_err() {
                    result = Err("ws write");
                    should_exit = true;
                    break;
                }
            } else if fd == ws_fd {
                // buf_ptr is mmap'd memory of `buf_len` bytes
                let r = match net::recv(ws_fd, as_mut_slice(buf_ptr, buf_len)) {
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
                // buf_ptr valid for `r` bytes, scratch_ptr valid for scratch_len bytes (both mmap'd)
                let input = as_slice(buf_ptr, r);
                let out = as_mut_slice(scratch_ptr, scratch_len);
                match ws::parse_and_unmask_frames(input, out) {
                    Ok(payload) => {
                        // If websocket payload contains a Ctrl-C (0x03), forward SIGINT
                        // directly to the child process instead of relying on terminal
                        // driver behavior.
                        let mut saw_sigint = false;
                        for &b in payload {
                            if b == 0x03 {
                                saw_sigint = true;
                                break;
                            }
                        }
                        if saw_sigint {
                            let _ = crate::sys::pty::kill(child_pid, 2); // SIGINT
                        } else {
                            // write payload to pty; log errno if write fails
                            let _ = fs::write(pty_fd, payload);
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

    let _ = mmap::munmap_free(buf_ptr, buf_len);
    let _ = mmap::munmap_free(scratch_ptr, scratch_len);
    if sfd != usize::MAX {
        let _ = fs::close(sfd);
    }

    result
}
