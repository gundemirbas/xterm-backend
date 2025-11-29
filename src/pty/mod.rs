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
	let (rfd, wfd) = sys::pipe2(0).map_err(|_| "pipe")?;
	let pid = sys::fork().map_err(|_| "fork")?;
	if pid == 0 {
		let _ = crate::sys::fs::close(rfd);
		let pr = sys::prctl_set_pdeathsig(15);
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
		let mut fdc = 3usize;
		while fdc <= 1024 {
			if fdc != 0 && fdc != 1 && fdc != 2 {
				let _ = crate::sys::fs::close(fdc);
			}
			fdc += 1;
		}
		const SH_ARG: [u8; 8] = [b'/', b'b', b'i', b'n', b'/', b's', b'h', 0];
		let argv = [SH_ARG.as_ptr(), core::ptr::null()];
		let envp = [core::ptr::null()];
		sys::execve(SH_ARG.as_ptr(), argv.as_ptr(), envp.as_ptr());
	}
	let _ = crate::sys::fs::close(wfd);
	let mut buf = [0u8; 8];
	let _ = crate::sys::fs::read(rfd, &mut buf);
	let _ = crate::sys::fs::close(rfd);
	let _ = sys::tcsetpgrp(sfd, pid as i32);
	Ok(Pty {
		master_fd: mfd,
		child_pid: pid as i32,
	})
}
