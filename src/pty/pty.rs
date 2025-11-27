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
        // notify parent whether prctl succeeded
        let _ = match pr {
            Ok(()) => crate::sys::fs::write(wfd, b"1"),
            Err(_) => crate::sys::fs::write(wfd, b"0"),
        };
        let _ = crate::sys::fs::close(wfd);
        let _ = sys::setsid();
        let _ = sys::ioctl_set_ctty(sfd);
        let _ = sys::dup2(sfd, 0);
        let _ = sys::dup2(sfd, 1);
        let _ = sys::dup2(sfd, 2);
        let argv = [b"/bin/sh\0".as_ptr(), core::ptr::null()];
        let envp = [core::ptr::null()];
        sys::execve(b"/bin/sh\0".as_ptr(), argv.as_ptr(), envp.as_ptr());
    }
    // parent
    let _ = crate::sys::fs::close(wfd);
    // read a single byte from child (prctl result), ignore if fails
    let mut buf = [0u8;1];
    let _ = crate::sys::fs::read(rfd, &mut buf);
    let _ = crate::sys::fs::close(rfd);
    if buf[0] == b'0' {
        // prctl failed in child; log for debugging so operator can see
        let _ = crate::sys::fs::write(1, b"child prctl failed\n");
    }
    Ok(Pty { master_fd: mfd, child_pid: pid as i32 })
}
