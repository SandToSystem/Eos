//! Bump allocator backing the global `alloc` interface.
//!
//! HW-independent allocation *algorithm* over a HW-dependent heap *boundary*
//! (`_sheap`/`_eheap` from the linker) — the §3 "malloc HAL pattern".
//!
//! Allocation is a pointer bump rounded up to the request's alignment.
//! `dealloc` only counts: when the last outstanding allocation is freed the
//! bump pointer snaps back to the heap base, reclaiming everything at once.
//! This is the legacy Base-Runtime contract — minus its `(size + 3) % 4`
//! alignment bug, since [`Layout`] hands us the real alignment.
//!
//! The runtime is single-threaded (CLAUDE.md §8), so interior mutability is a
//! plain [`Cell`] and the `Sync` impl below is sound by that invariant alone.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use core::ptr;

/// Round `addr` up to the next multiple of `align` (a power of two).
#[inline]
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// A bump-pointer allocator over a fixed heap region.
pub struct BumpAllocator {
    heap_start: Cell<usize>,
    heap_end: Cell<usize>,
    next: Cell<usize>,
    allocations: Cell<usize>,
}

impl BumpAllocator {
    /// Construct the (uninitialised) allocator. The heap bounds are zero until
    /// [`init`](Self::init) runs in `crt0`.
    pub const fn new() -> Self {
        Self {
            heap_start: Cell::new(0),
            heap_end: Cell::new(0),
            next: Cell::new(0),
            allocations: Cell::new(0),
        }
    }

    /// Bind the allocator to `[heap_start, heap_end)`. Call once, from `crt0`,
    /// before the first allocation.
    pub fn init(&self, heap_start: usize, heap_end: usize) {
        self.heap_start.set(heap_start);
        self.heap_end.set(heap_end);
        self.next.set(heap_start);
        self.allocations.set(0);
    }

    /// Newlib-style `_sbrk`: move the program break by `increment` bytes and
    /// return the *previous* break, or `-1` (`usize::MAX` as a pointer) if the
    /// move would leave the heap region.
    ///
    /// Shares the bump pointer with [`GlobalAlloc`]; it exists so the `SBRK`
    /// syscall has something to dispatch to (the §10 libgloss example).
    pub fn sbrk(&self, increment: isize) -> *mut u8 {
        let prev = self.next.get();
        let new = prev.wrapping_add_signed(increment);
        let out_of_range = (increment > 0 && new > self.heap_end.get())
            || (increment < 0 && new < self.heap_start.get());
        if out_of_range {
            return usize::MAX as *mut u8;
        }
        self.next.set(new);
        prev as *mut u8
    }
}

impl Default for BumpAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let start = align_up(self.next.get(), layout.align());
        let end = match start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };
        if end > self.heap_end.get() {
            return ptr::null_mut(); // out of heap
        }
        self.next.set(end);
        self.allocations.set(self.allocations.get() + 1);
        start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let remaining = self.allocations.get().saturating_sub(1);
        self.allocations.set(remaining);
        if remaining == 0 {
            // Every outstanding allocation is gone — reclaim the whole heap.
            self.next.set(self.heap_start.get());
        }
    }
}

// SAFETY: the runtime is single-threaded; no two contexts ever touch the
// allocator concurrently, so the non-atomic `Cell` state is never raced.
unsafe impl Sync for BumpAllocator {}

/// The global allocator instance. `crt0` calls `ALLOCATOR.init(...)` and the
/// `alloc` crate routes `Box`/`Vec`/etc. through it.
#[global_allocator]
pub static ALLOCATOR: BumpAllocator = BumpAllocator::new();
