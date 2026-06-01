//! `numerics` — number-theory kernels, cross-checked against known values.
//!
//! Each kernel is computed two independent ways (or against a hard-coded
//! expected value) and the results must agree:
//!   * GCD — recursive Euclid vs. iterative Euclid, checked against known gcds;
//!   * Fibonacci — iterative vs. naive recursive, `fib(20)` must be 6765;
//!   * factorial — `10! = 3_628_800`;
//!   * modular exponentiation — `7^256 mod 13` and Fermat's little theorem.
//!
//! A disagreement `panic!`s (→ `halt(1)`); success prints `PASS` lines and
//! falls through to `halt(0)`. Exercises recursion and the software
//! `*` / `/` / `%` path on RV32I.

#![no_std]
#![no_main]

use runtime::println;

fn gcd_recursive(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd_recursive(b, a % b)
    }
}

fn gcd_iterative(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn fib_iterative(n: u32) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        let next = a + b;
        a = b;
        b = next;
    }
    a
}

fn fib_recursive(n: u32) -> u64 {
    if n < 2 {
        n as u64
    } else {
        fib_recursive(n - 1) + fib_recursive(n - 2)
    }
}

fn factorial(n: u64) -> u64 {
    (1..=n).product()
}

/// `base^exp mod modulus` by repeated squaring.
fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = (result * base) % modulus;
        }
        exp >>= 1;
        base = (base * base) % modulus;
    }
    result
}

#[no_mangle]
fn main() {
    // --- GCD: two algorithms must agree, and match known answers. ---
    let cases = [(48u64, 18u64, 6u64), (1071, 462, 21), (17, 5, 1), (100, 0, 100)];
    for (a, b, want) in cases {
        let r = gcd_recursive(a, b);
        let i = gcd_iterative(a, b);
        assert_eq!(r, i, "gcd({a},{b}): recursive {r} != iterative {i}");
        assert_eq!(r, want, "gcd({a},{b}): got {r}, expected {want}");
    }
    println!("PASS gcd");

    // --- Fibonacci: iterative vs recursive, plus a fixed anchor. ---
    for n in 0..=20u32 {
        assert_eq!(
            fib_iterative(n),
            fib_recursive(n),
            "fib({n}): iterative != recursive"
        );
    }
    assert_eq!(fib_iterative(20), 6765, "fib(20) wrong");
    assert_eq!(fib_iterative(50), 12_586_269_025, "fib(50) wrong");
    println!("PASS fibonacci");

    // --- Factorial. ---
    assert_eq!(factorial(0), 1, "0! wrong");
    assert_eq!(factorial(10), 3_628_800, "10! wrong");
    assert_eq!(factorial(20), 2_432_902_008_176_640_000, "20! wrong");
    println!("PASS factorial");

    // --- Modular exponentiation. ---
    assert_eq!(mod_pow(7, 256, 13), 9, "7^256 mod 13 wrong");
    assert_eq!(mod_pow(2, 10, 1000), 24, "2^10 mod 1000 wrong");
    // Fermat: for prime p and gcd(a,p)=1, a^(p-1) ≡ 1 (mod p).
    for a in 2..=12u64 {
        assert_eq!(mod_pow(a, 12, 13), 1, "Fermat failed for a={a}, p=13");
    }
    println!("PASS mod_pow");

    println!("numerics: all kernels agree");
}
