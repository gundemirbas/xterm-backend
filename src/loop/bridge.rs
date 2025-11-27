use crate::net::ws;
use crate::sys::epoll;
use crate::sys::{fs, mmap, net, signal};

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
            Err(e) => {
                // log errno
                let mut b = [0u8; 64];
                let mut i = 0usize;
                let hdr = b"bridge: epoll_wait errno ";
                for &c in hdr { b[i]=c; i+=1; }
                let mut nb = itoa::Buffer::new();
                let s = nb.format((-e) as i64);
                for &c in s.as_bytes() { if i < b.len() { b[i]=c; i+=1; } }
                if i < b.len() { b[i]=b'\n'; i+=1; }
                let _ = fs::write(1, &b[..i]);
                result = Err("wait");
                break;
            }
        };
        for i in 0..n {
            let fd = events[i].fd();
            if fd == sfd {
                // signal received -> forward SIGINT to child (so shell receives it)
                let _ = fs::write(1, b"bridge: signalfd readable, forwarding SIGINT to child\n");
                let _ = crate::sys::pty::kill(child_pid, 2); // SIGINT
                should_exit = true;
                break;
            }
            if fd == pty_fd {
                let r = match fs::read(pty_fd, unsafe {
                    core::slice::from_raw_parts_mut(buf_ptr, buf_len)
                }) {
                    Ok(v) => v,
                    Err(e) => {
                        // log errno from read
                        let mut b = [0u8; 64];
                        let mut i = 0usize;
                        let hdr = b"bridge: pty read errno ";
                        for &c in hdr { b[i]=c; i+=1; }
                        let mut nb = itoa::Buffer::new();
                        let s = nb.format((-e) as i64);
                        for &c in s.as_bytes() { if i < b.len() { b[i]=c; i+=1; } }
                        if i < b.len() { b[i]=b'\n'; i+=1; }
                        let _ = fs::write(1, &b[..i]);
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
                    Err(e) => {
                        // e is a negative syscall error; -e is errno number
                        let errno = -e as i32;
                        if errno == 104 { // ECONNRESET - client reset/closed connection
                            let _ = fs::write(1, b"bridge: ws closed by peer (ECONNRESET)\n");
                        } else {
                            // log errno from recv
                            let mut b = [0u8; 64];
                            let mut i = 0usize;
                            let hdr = b"bridge: ws recv errno ";
                            for &c in hdr { b[i]=c; i+=1; }
                            let mut nb = itoa::Buffer::new();
                            let s = nb.format(errno as i64);
                            for &c in s.as_bytes() { if i < b.len() { b[i]=c; i+=1; } }
                            if i < b.len() { b[i]=b'\n'; i+=1; }
                            let _ = fs::write(1, &b[..i]);
                        }
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
                            let _ = fs::write(1, b"bridge: forwarding SIGINT to child\n");
                            let _ = crate::sys::pty::kill(child_pid, 2); // SIGINT
                        } else {
                            // write payload to pty; log errno if write fails
                            match fs::write(pty_fd, payload) {
                                Ok(_) => {}
                                Err(e) => {
                                    let mut b = [0u8; 64];
                                    let mut i = 0usize;
                                    let hdr = b"bridge: pty write errno ";
                                    for &c in hdr { b[i]=c; i+=1; }
                                    let mut nb = itoa::Buffer::new();
                                    let s = nb.format((-e) as i64);
                                    for &c in s.as_bytes() { if i < b.len() { b[i]=c; i+=1; } }
                                    if i < b.len() { b[i]=b'\n'; i+=1; }
                                    let _ = fs::write(1, &b[..i]);
                                }
                            }
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
    if sfd != usize::MAX { let _ = fs::close(sfd); }

    // If we are returning an error, write a clearer log message for diagnostics.
    if let Err(e) = result {
        // assemble single-buffer message to reduce interleaving
        let mut b = [0u8; 64];
        let mut i = 0usize;
        let hdr = b"bridge: error: ";
        for &c in hdr { b[i]=c; i+=1; }
        for &c in e.as_bytes() { if i < b.len() { b[i]=c; i+=1; } }
        if i < b.len() { b[i]=b'\n'; i+=1; }
        let _ = fs::write(1, &b[..i]);
    }

    result
}
