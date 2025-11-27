use crate::sys::SysResult;
use crate::sys::syscall::{syscall2, syscall6};

const SYS_MMAP: usize = 9;
const SYS_MUNMAP: usize = 11;

pub const PROT_READ: usize = 0x1;
pub const PROT_WRITE: usize = 0x2;
pub const MAP_PRIVATE: usize = 0x02;
pub const MAP_ANONYMOUS: usize = 0x20;

pub fn mmap_alloc(len: usize) -> SysResult<*mut u8> {
    // SAFETY: mmap syscall with valid parameters for anonymous private mapping
    let r = unsafe {
        syscall6(
            SYS_MMAP,
            0,
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            usize::MAX,
            0,
        )
    };
    if r >= 0 { Ok(r as *mut u8) } else { Err(r) }
}
pub fn munmap_free(ptr: *mut u8, len: usize) -> SysResult<()> {
    // SAFETY: munmap syscall; caller must ensure ptr/len are from valid mmap
    let r = unsafe { syscall2(SYS_MUNMAP, ptr as usize, len) };
    if r >= 0 { Ok(()) } else { Err(r) }
}
