//! `sorts` — a self-checking tour of five classic sorting algorithms.
//!
//! Bubble, insertion, selection, quicksort, and heapsort each sort their own
//! copy of one fixed, LCG-shuffled array (plus already-sorted and reverse
//! edge cases). Every result is checked two ways: it must be ascending, and it
//! must equal a trusted reference (a plain insertion sort). A binary search
//! over the sorted output then confirms a present and an absent key. Any
//! mismatch `panic!`s (→ `halt(1)`); success prints `PASS <algo>` per algorithm
//! and falls through to the implicit `halt(0)`.
//!
//! Exercises: heap (`Vec`), array indexing/swaps, recursion (quicksort),
//! comparisons, and software multiply (the LCG) on RV32I.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use runtime::println;

/// Number of elements in the shuffled working set.
const N: usize = 128;

/// An in-place sort over a slice of `i32`.
type SortFn = fn(&mut [i32]);

/// A tiny deterministic LCG (glibc constants) so every run is reproducible.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        // `wrapping_mul` lowers to `__mulsi3` — there is no `MUL` on RV32I.
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }
}

/// A reproducible shuffled array of `N` values in `0..N`.
fn shuffled() -> Vec<i32> {
    let mut data: Vec<i32> = (0..N as i32).collect();
    let mut rng = Lcg(0x1234_5678);
    // Fisher–Yates with LCG-derived indices.
    for i in (1..N).rev() {
        let j = (rng.next() as usize) % (i + 1);
        data.swap(i, j);
    }
    data
}

fn is_sorted(values: &[i32]) -> bool {
    values.windows(2).all(|w| w[0] <= w[1])
}

// --- The algorithms. Each sorts `a` in place. ---

fn bubble_sort(a: &mut [i32]) {
    for end in (1..a.len()).rev() {
        for i in 0..end {
            if a[i] > a[i + 1] {
                a.swap(i, i + 1);
            }
        }
    }
}

fn insertion_sort(a: &mut [i32]) {
    for i in 1..a.len() {
        let mut j = i;
        while j > 0 && a[j - 1] > a[j] {
            a.swap(j - 1, j);
            j -= 1;
        }
    }
}

fn selection_sort(a: &mut [i32]) {
    for i in 0..a.len() {
        let mut min = i;
        for j in (i + 1)..a.len() {
            if a[j] < a[min] {
                min = j;
            }
        }
        a.swap(i, min);
    }
}

fn quicksort(a: &mut [i32]) {
    if a.len() <= 1 {
        return;
    }
    // Lomuto partition around the last element.
    let pivot = a.len() - 1;
    let mut store = 0;
    for i in 0..pivot {
        if a[i] < a[pivot] {
            a.swap(i, store);
            store += 1;
        }
    }
    a.swap(store, pivot);
    let (left, right) = a.split_at_mut(store);
    quicksort(left);
    quicksort(&mut right[1..]);
}

fn heapsort(a: &mut [i32]) {
    let n = a.len();
    // Build a max-heap.
    for start in (0..n / 2).rev() {
        sift_down(a, start, n);
    }
    // Repeatedly move the max to the end and restore the heap.
    for end in (1..n).rev() {
        a.swap(0, end);
        sift_down(a, 0, end);
    }
}

fn sift_down(a: &mut [i32], mut root: usize, end: usize) {
    loop {
        let mut largest = root;
        let (l, r) = (2 * root + 1, 2 * root + 2);
        if l < end && a[l] > a[largest] {
            largest = l;
        }
        if r < end && a[r] > a[largest] {
            largest = r;
        }
        if largest == root {
            return;
        }
        a.swap(root, largest);
        root = largest;
    }
}

/// Classic binary search: returns the index of `key`, or `None`.
fn binary_search(a: &[i32], key: i32) -> Option<usize> {
    let (mut lo, mut hi) = (0, a.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if a[mid] == key {
            return Some(mid);
        } else if a[mid] < key {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    None
}

/// Run `sort` on a fresh copy of `input`, then assert the result is ascending
/// and equal to `reference`.
fn check(name: &str, input: &[i32], reference: &[i32], sort: SortFn) {
    let mut work = input.to_vec();
    sort(&mut work);
    assert!(is_sorted(&work), "{name}: output is not ascending");
    assert!(work == reference, "{name}: output does not match reference");
    println!("PASS {name}");
}

#[no_mangle]
fn main() {
    let base = shuffled();

    // Trusted reference, produced by an independent insertion sort.
    let mut reference = base.clone();
    insertion_sort(&mut reference);
    assert!(is_sorted(&reference), "reference sort failed");

    // Edge-case inputs share the same reference (all are permutations of 0..N).
    let ascending: Vec<i32> = (0..N as i32).collect();
    let descending: Vec<i32> = (0..N as i32).rev().collect();

    for (label, input) in [
        ("shuffled", &base),
        ("ascending", &ascending),
        ("descending", &descending),
    ] {
        let algos: [(&str, SortFn); 5] = [
            ("bubble", bubble_sort),
            ("insertion", insertion_sort),
            ("selection", selection_sort),
            ("quicksort", quicksort),
            ("heapsort", heapsort),
        ];
        for (algo, sort) in algos {
            // e.g. "bubble/shuffled"
            let mut name = alloc::string::String::from(algo);
            name.push('/');
            name.push_str(label);
            check(&name, input, &reference, sort);
        }
    }

    // Binary search over the sorted reference: present and absent keys.
    assert_eq!(
        binary_search(&reference, 0),
        Some(0),
        "bsearch: first element"
    );
    assert_eq!(
        binary_search(&reference, (N - 1) as i32),
        Some(N - 1),
        "bsearch: last element"
    );
    assert_eq!(binary_search(&reference, -1), None, "bsearch: below range");
    assert_eq!(
        binary_search(&reference, N as i32),
        None,
        "bsearch: above range"
    );
    println!("PASS binary_search");

    println!("sorts: all algorithms agree");
}
