//! `msort` — recursive merge sort on a reverse-sorted array.
//!
//! Exercises the heap (`Vec`/`Box` → bump allocator), recursion (stack
//! growth), and integer formatting. Output is two lines, `before:` and
//! `after:`.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use runtime::{print, println};

/// Merge sort returning a freshly-allocated sorted copy. The two halves and
/// the merged result all live on the bump heap; they are freed as the `Vec`s
/// drop, and the heap resets once the last outstanding allocation is gone.
fn merge_sort(values: &[i32]) -> Vec<i32> {
    if values.len() <= 1 {
        return values.to_vec();
    }

    let mid = values.len() / 2;
    let left = merge_sort(&values[..mid]);
    let right = merge_sort(&values[mid..]);

    let mut merged = Vec::with_capacity(values.len());
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            merged.push(left[i]);
            i += 1;
        } else {
            merged.push(right[j]);
            j += 1;
        }
    }
    merged.extend_from_slice(&left[i..]);
    merged.extend_from_slice(&right[j..]);
    merged
}

fn print_row(label: &str, values: &[i32]) {
    print!("{label}");
    for v in values {
        print!(" {v}");
    }
    println!();
}

#[no_mangle]
fn main() {
    // 16 elements, strictly descending — the worst case for an insertion sort,
    // a clean exercise for merge sort.
    let input: Vec<i32> = (0..16).rev().collect();
    print_row("before:", &input);

    let sorted = merge_sort(&input);
    print_row("after: ", &sorted);
}
