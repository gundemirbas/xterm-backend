//! Re-exports of the checked syscall helpers for convenient use across the
//! codebase. This module intentionally provides a shorter import path for the
//! `syscall*_checked` functions so callers don't need to reference
//! `crate::sys::syscall` directly.

pub use crate::sys::syscall::{
    syscall0_checked, syscall1_checked, syscall2_checked, syscall3_checked,
    syscall4_checked, syscall6_checked,
};
