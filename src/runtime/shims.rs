#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        for i in 0..n {
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    unsafe {
        for i in 0..n {
            *dest.add(i) = c as u8;
        }
    }
    dest
}

#[unsafe(no_mangle)]
pub extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    unsafe {
        for i in 0..n {
            let av = *a.add(i);
            let bv = *b.add(i);
            if av != bv {
                return (av as i32) - (bv as i32);
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    unsafe {
        for i in 0..n {
            if *a.add(i) != *b.add(i) {
                return 1;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
