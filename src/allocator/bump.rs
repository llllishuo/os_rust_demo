use core::{alloc::GlobalAlloc, ptr::null_mut};

use spin::{Mutex, MutexGuard};

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocation: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocation: 0,
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = alloc_start.wrapping_add(layout.size());

        if alloc_end > bump.heap_end {
            null_mut()
        } else {
            bump.next = alloc_end as usize;
            bump.allocation += 1;
            alloc_start as *mut u8
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut bump = self.lock();

        bump.allocation -= 1;
        if bump.allocation <= 0 {
            bump.next = bump.heap_start;
        }
    }
}

pub struct Locked<A> {
    inner: Mutex<A>,
}
impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }
    pub fn lock(&self) -> MutexGuard<A> {
        self.inner.lock()
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
}
