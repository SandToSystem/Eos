//! `matmul` — 8×8 integer matrix multiplication.
//!
//! RV32I has no `MUL` (M extension is excluded, CLAUDE.md §4), so the `*`
//! below lowers to a `__mulsi3` call from compiler-builtins — the software
//! multiply is invisible at the source level, which is exactly the point.
//!
//! Inputs are `A[i][k] = i + 1`, `B[k][j] = j + 1`, so
//! `C[i][j] = Σ_k (i+1)(j+1) = 8·(i+1)·(j+1)` — easy to eyeball in the output.

#![no_std]
#![no_main]

use runtime::{print, println};

const N: usize = 8;

#[no_mangle]
fn main() {
    let mut a = [[0i32; N]; N];
    let mut b = [[0i32; N]; N];
    for i in 0..N {
        for j in 0..N {
            a[i][j] = (i + 1) as i32;
            b[i][j] = (j + 1) as i32;
        }
    }

    let mut c = [[0i32; N]; N];
    for i in 0..N {
        for j in 0..N {
            // Accumulate the dot product; `*` is the software multiply.
            c[i][j] = (0..N).map(|k| a[i][k] * b[k][j]).sum();
        }
    }

    for row in &c {
        for v in row {
            print!("{v:4}");
        }
        println!();
    }
}
