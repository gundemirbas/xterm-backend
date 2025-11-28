//! Small sys-level utilities to centralize unsafe pointer operations and other
//! tiny helpers that are used across `src/sys` and runtime code.
//!
//! Keep unsafe in one place and provide small, documented safe wrappers.

/// Convert a raw `*mut u8` and length into a mutable slice.
///
/// # Safety
/// The returned slice is created with `unsafe` internally. Callers must ensure
/// that `ptr` points to valid memory for `len` bytes and that no other alias
/// violates Rust's aliasing rules for mutable access while the slice is used.
pub fn ptr_to_mut_slice<'a>(ptr: *mut u8, len: usize) -> &'a mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(ptr, len) }
}

/// Convert a raw `*mut u8` and length into an immutable slice.
///
/// # Safety
/// The returned slice is created with `unsafe` internally. Callers must ensure
/// that `ptr` points to valid memory for `len` bytes for the duration of the
/// slice's usage.
pub fn ptr_to_slice<'a>(ptr: *mut u8, len: usize) -> &'a [u8] {
    unsafe { core::slice::from_raw_parts(ptr, len) }
}
