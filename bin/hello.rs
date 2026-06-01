//! `hello` — minimal UART smoke test.
//!
//! Exercises the end-to-end path: `println!` → `core::fmt` → `Stdout` →
//! `Uart::put_byte` → UART MMIO. Returning from `main` reaches `terminate(0)`
//! → `EBREAK` with `a0 = 0`.

#![no_std]
#![no_main]

use runtime::println;

#[no_mangle]
fn main() {
    let answer = 42;
    println!("Hello, Snake SoC! The answer is {answer}, in hex 0x{answer:x}.");
}
