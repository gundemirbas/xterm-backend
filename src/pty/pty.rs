#![allow(clippy::manual_c_str_literals)]

use crate::sys::pty as sys;

pub struct Pty {
    pub master_fd: usize,
    pub child_pid: i32,
}

pub fn spawn_sh() -> Result<Pty, &'static str> {
    let mfd = sys::open_ptmx().map_err(|_| "ptmx")?;
    let n = sys::pts_number(mfd).map_err(|_| "ptsnum")?;
    sys::grantpt(mfd).map_err(|_| "grant")?;
    sys::unlockpt(mfd).map_err(|_| "unlock")?;
    let sfd = sys::open_pts(n).map_err(|_| "open pts")?;
    // create a pipe for child->parent notification of prctl result
    let (rfd, wfd) = sys::pipe2(0).map_err(|_| "pipe")?;
    let pid = sys::fork().map_err(|_| "fork")?;
    if pid == 0 {
        // child
        let _ = crate::sys::fs::close(rfd);
        // ensure child will receive SIGTERM if parent dies (avoid orphaned background shells)
        let pr = sys::prctl_set_pdeathsig(15);
        // notify parent with errno (0 = success) as 8-byte little-endian i64
        let mut errno: i64 = 0;
        if let Err(e) = pr {
            errno = -(e as i64);
        }
        let mut eb = [0u8; 8];
        eb.copy_from_slice(&errno.to_le_bytes());
        let _ = crate::sys::fs::write(wfd, &eb);
        let _ = crate::sys::fs::close(wfd);
        let _ = sys::setsid();
        let _ = sys::ioctl_set_ctty(sfd);
        let _ = sys::dup2(sfd, 0);
        let _ = sys::dup2(sfd, 1);
        let _ = sys::dup2(sfd, 2);
        // Close any remaining fds >= 3 now that the slave is duplicated
        // onto 0/1/2. This prevents the exec'd shell from keeping the
        // listener, signalfd, epoll fds, etc. open.
        let mut fdc = 3usize;
        while fdc <= 1024 {
            if fdc != 0 && fdc != 1 && fdc != 2 {
                let _ = crate::sys::fs::close(fdc);
            }
            fdc += 1;
        }
        let argv = [b"/bin/sh\0".as_ptr(), core::ptr::null()];
        let envp = [core::ptr::null()];
        sys::execve(b"/bin/sh\0".as_ptr(), argv.as_ptr(), envp.as_ptr());
    }
    // parent
    let _ = crate::sys::fs::close(wfd);
    // read 8 bytes from child (prctl errno as i64 le), ignore if fails
    let mut buf = [0u8; 8];
    let _ = crate::sys::fs::read(rfd, &mut buf);
    let _ = crate::sys::fs::close(rfd);
    let errno = i64::from_le_bytes(buf);
    if errno != 0 {
        // write a diagnostic to fd 1: "child prctl errno: <n>\n"
        let _ = crate::sys::fs::write(1, b"child prctl errno: ");
        // convert errno to decimal ascii
        let mut n = errno;
        let mut neg = false;
        if n < 0 {
            neg = true;
            n = -n;
        }
        let mut digs = [0u8; 32];
        let mut di = 0usize;
        if n == 0 {
            digs[di] = b'0';
            di += 1;
        }
        while n > 0 && di < digs.len() {
            digs[di] = b'0' + (n % 10) as u8;
            n /= 10;
            di += 1;
        }
        if neg {
            let _ = crate::sys::fs::write(1, b"-");
        }
        while di > 0 {
            di -= 1;
            let _ = crate::sys::fs::write(1, &digs[di..di + 1]);
        }
        let _ = crate::sys::fs::write(1, b"\n");
    }
    // Parent: set the child's process group as the foreground pgrp on the slave PTY.
    // Child did calls to setsid() so its pgid should equal its pid.
    let _ = sys::tcsetpgrp(sfd, pid as i32);
    // If tcsetpgrp fails, continue; we'll log explicit failure where useful elsewhere.
    Ok(Pty {
        master_fd: mfd,
        child_pid: pid as i32,
    })
}
