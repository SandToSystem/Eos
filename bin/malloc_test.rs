//! `malloc_test` — bump-allocator contract checks.
//!
//! Verifies, against the raw [`GlobalAlloc`] surface:
//!   1. an in-range allocation is non-null;
//!   2. alignment is honoured — two `align = 4` requests are 4 bytes apart
//!      (the idiomatic-Rust replacement for the C runtime's "always round to
//!      4"; here the stride comes from [`Layout`], not a hard-coded mask);
//!   3. an oversize request returns null rather than wandering past the heap;
//!   4. once every outstanding allocation is freed, the bump pointer snaps
//!      back and the next allocation reuses the base address.
//!
//! A failed check panics (→ console message + halt 1); success prints
//! `malloc_test passed`.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::alloc::{alloc, dealloc, Layout};

use runtime::println;

#[no_mangle]
fn main() {
    let word = Layout::from_size_align(1, 4).unwrap();

    // (1) basic allocation is non-null.
    // SAFETY: non-zero-size layout; pointer is freed below.
    let a = unsafe { alloc(word) };
    assert!(!a.is_null(), "first allocation returned null");

    // (2) alignment/stride: the next 4-aligned, 1-byte request sits 4 bytes on.
    let b = unsafe { alloc(word) };
    assert!(!b.is_null(), "second allocation returned null");
    let stride = b as usize - a as usize;
    assert!(stride == 4, "expected 4-byte stride, got {stride}");

    // (4-prep) free both; the allocator should reset to the base.
    unsafe {
        dealloc(a, word);
        dealloc(b, word);
    }

    // (3) an allocation larger than the whole 64 KiB heap must fail.
    let huge = Layout::from_size_align(1 << 20, 4).unwrap();
    let p = unsafe { alloc(huge) };
    assert!(p.is_null(), "oversize allocation should have failed");

    // (4) after the reset, the base address is reused.
    let c = unsafe { alloc(word) };
    assert!(c == a, "heap did not reset to base after all frees");

    // The returned storage is writable.
    unsafe {
        c.write_bytes(0x5A, 1);
        assert!(c.read() == 0x5A, "round-trip through allocated byte failed");
        dealloc(c, word);
    }

    println!("malloc_test passed");
}
