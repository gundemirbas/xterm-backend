use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};

const PAGE_SIZE: usize = 4096;
const ARENA_SIZE: usize = 16 * 1024 * 1024;

fn round_up_page(n: usize) -> usize {
    if n == 0 {
        return PAGE_SIZE;
    }
    n.div_ceil(PAGE_SIZE) * PAGE_SIZE
}

fn align_up(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

static ARENA_BASE: AtomicUsize = AtomicUsize::new(0);
static ARENA_OFF: AtomicUsize = AtomicUsize::new(0);

pub struct MmapAllocator;

impl MmapAllocator {
    unsafe fn ensure_arena(&self) -> *mut u8 {
        let base = ARENA_BASE.load(Ordering::SeqCst);
        if base != 0 {
            return base as *mut u8;
        }
        let p = match crate::sys::mmap::mmap_alloc(ARENA_SIZE) {
            Ok(ptr) => ptr,
            Err(_) => return null_mut(),
        };
        let prev = ARENA_BASE.compare_exchange(0, p as usize, Ordering::SeqCst, Ordering::SeqCst);
        match prev {
            Ok(_) => p,
            Err(existing) => {
                let _ = crate::sys::mmap::munmap_free(p, ARENA_SIZE);
                existing as *mut u8
            }
        }
    }
}

unsafe impl GlobalAlloc for MmapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(1);
        let align = layout.align().max(1);
        let base = unsafe { self.ensure_arena() };
        if !base.is_null() {
            let off = ARENA_OFF.fetch_add(align_up(size, align), Ordering::SeqCst);
            if off + size <= ARENA_SIZE {
                return unsafe { base.add(off) };
            }
        }
        let sz = round_up_page(size);
        match crate::sys::mmap::mmap_alloc(sz) {
            Ok(p) => p,
            Err(_) => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        let base = ARENA_BASE.load(Ordering::SeqCst) as *mut u8;
        if !base.is_null() {
            let b = base as usize;
            let p = ptr as usize;
            if p >= b && p < b + ARENA_SIZE {
                return;
            }
        }
        let size = round_up_page(layout.size().max(1));
        let _ = crate::sys::mmap::munmap_free(ptr, size);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { self.alloc(layout) };
        if !p.is_null() {
            unsafe {
                core::ptr::write_bytes(p, 0, layout.size().max(1));
            }
        }
        p
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            return unsafe {
                self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()))
            };
        }
        let base = ARENA_BASE.load(Ordering::SeqCst) as *mut u8;
        let p_usize = ptr as usize;
        let arena_base_usize = base as usize;
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        let new_ptr = unsafe { self.alloc(new_layout) };
        if new_ptr.is_null() {
            return null_mut();
        }
        let copy_len = core::cmp::min(layout.size(), new_size);
        unsafe {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_len);
        }
        if base.is_null()
            || !(p_usize >= arena_base_usize && p_usize < arena_base_usize + ARENA_SIZE)
        {
            let _ = crate::sys::mmap::munmap_free(ptr, round_up_page(layout.size().max(1)));
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: MmapAllocator = MmapAllocator;

pub fn page_alloc(len: usize) -> Result<*mut u8, &'static str> {
    let size = round_up_page(len.max(1));
    let base = ARENA_BASE.load(Ordering::SeqCst) as *mut u8;
    if !base.is_null() {
        let off = ARENA_OFF.fetch_add(size, Ordering::SeqCst);
        if off + size <= ARENA_SIZE {
            return unsafe { Ok(base.add(off)) };
        }
    }
    match crate::sys::mmap::mmap_alloc(size) {
        Ok(p) => Ok(p),
        Err(_) => Err("mmap"),
    }
}

pub fn page_free(ptr: *mut u8, len: usize) -> Result<(), &'static str> {
    if ptr.is_null() {
        return Ok(());
    }
    let size = round_up_page(len.max(1));
    let base = ARENA_BASE.load(Ordering::SeqCst) as *mut u8;
    if !base.is_null() {
        let b = base as usize;
        let p = ptr as usize;
        if p >= b && p < b + ARENA_SIZE {
            return Ok(());
        }
    }
    match crate::sys::mmap::munmap_free(ptr, size) {
        Ok(()) => Ok(()),
        Err(_) => Err("munmap"),
    }
}
