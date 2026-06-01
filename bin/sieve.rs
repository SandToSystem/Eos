//! `sieve` — Sieve of Eratosthenes up to 10 000, self-checked.
//!
//! Allocates a 10 001-entry `Vec<bool>` on the bump heap, marks composites, and
//! asserts the prime count equals π(10 000) = 1229 (a fixed, well-known value)
//! and that the first six primes are `[2, 3, 5, 7, 11, 13]`. A wrong count
//! `panic!`s (→ `halt(1)`); success prints the count and falls through to
//! `halt(0)`.
//!
//! Exercises: heap allocation of a sizeable buffer, nested loops, and the
//! software multiply/divide path (`i * i`, the modulo in `windows` is avoided).

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use runtime::println;

/// Upper bound (inclusive) of the sieve.
const LIMIT: usize = 10_000;
/// π(10 000): the number of primes ≤ 10 000.
const EXPECTED_PRIMES: usize = 1229;

fn main_sieve() -> Vec<usize> {
    // `is_composite[k]` is true once `k` has been crossed out.
    let mut is_composite = alloc::vec![false; LIMIT + 1];
    let mut primes = Vec::new();

    let mut p = 2;
    while p <= LIMIT {
        if !is_composite[p] {
            primes.push(p);
            // Start crossing out at p*p; smaller multiples already handled.
            let mut multiple = p * p;
            while multiple <= LIMIT {
                is_composite[multiple] = true;
                multiple += p;
            }
        }
        p += 1;
    }
    primes
}

#[no_mangle]
fn main() {
    let primes = main_sieve();

    assert_eq!(
        primes.len(),
        EXPECTED_PRIMES,
        "prime count mismatch: got {}, expected {EXPECTED_PRIMES}",
        primes.len()
    );

    let first_six = &primes[..6];
    assert!(
        first_six == [2, 3, 5, 7, 11, 13],
        "first six primes wrong: {first_six:?}"
    );

    // The largest prime ≤ 10 000 is 9973 — a cheap extra anchor.
    assert_eq!(*primes.last().unwrap(), 9973, "largest prime wrong");

    println!("sieve: {} primes up to {LIMIT}", primes.len());
    println!("PASS sieve");
}
