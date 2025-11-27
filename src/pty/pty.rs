use crate::sys::pty as sys;

pub struct Pty { pub master_fd: usize }

pub fn spawn_sh() -> Result<Pty, &'static str> {
    let mfd = sys::open_ptmx().map_err(|_| "ptmx")?;
    let n = sys::pts_number(mfd).map_err(|_| "ptsnum")?;
    sys::grantpt(mfd).map_err(|_| "grant")?;
    sys::unlockpt(mfd).map_err(|_| "unlock")?;
    let sfd = sys::open_pts(n).map_err(|_| "open pts")?;
    let pid = sys::fork().map_err(|_| "fork")?;
    if pid == 0 {
        let _ = sys::setsid();
        let _ = sys::ioctl_set_ctty(sfd);
        let _ = sys::dup2(sfd, 0);
        let _ = sys::dup2(sfd, 1);
        let _ = sys::dup2(sfd, 2);
        let argv = [b"/bin/sh\0".as_ptr(), core::ptr::null()];
        let envp = [core::ptr::null()];
        sys::execve(b"/bin/sh\0".as_ptr(), argv.as_ptr(), envp.as_ptr());
    }
    Ok(Pty { master_fd: mfd })
}
