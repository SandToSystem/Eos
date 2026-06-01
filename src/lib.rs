//! Snake SoC bare-metal RV32I runtime.
//!
//! An idiomatic-Rust rewrite of `comporg-labs/runtime`. It bridges a student
//! program's `fn main` to the Snake SoC hardware across three layers
//! (CLAUDE.md §5, the "HAL ⊂ Runtime" pillar):
//!
//! ```text
//!   bin/*.rs          user main()                 — application
//!   io / allocator    println!, Box, Vec          — HW-independent stdlib
//!   hal/*             uart, clint, csr, halt, trap — HW-dependent HAL
//!   ───────────────────────────────────────────────────────────────
//!   Snake SoC MMIO + Zicsr CSRs
//! ```
//!
//! ## Boot path
//!
//! Reset PC is `0x0` (Boot ROM); the ROM stub jumps to `_start` at
//! `0x8000_0000`. [`_start`](start) (asm) sets `gp`/`sp`, then [`start`] (Rust)
//! zeroes `.bss`, initialises the heap and `mtvec`, calls `main`, and
//! [`terminate`]s with code 0. A program returns a non-zero status by calling
//! [`hal::halt`] / [`syscall::exit`] explicitly.
//!
//! ## Writing a program
//!
//! ```ignore
//! #![no_std]
//! #![no_main]
//! use runtime::println;
//!
//! #[no_mangle]
//! fn main() {
//!     println!("Hello, Snake SoC!");
//! }
//! ```

#![no_std]

extern crate alloc;

use core::arch::global_asm;
use core::panic::PanicInfo;

pub mod allocator;
pub mod hal;
pub mod io;
pub mod syscall;

// Flat re-exports for the surface programs use most.
pub use hal::{halt, terminate, Status, Uart};
pub use io::{getchar, try_getchar};

// crt0 stage 1 (assembly): establish the C ABI environment, then tail-call the
// Rust startup. `gp` is loaded with relaxation disabled so the assembler does
// not rewrite the `la` into a (not-yet-valid) gp-relative form mid-sequence.
global_asm!(
    ".section .text.init",
    ".global _start",
    "_start:",
    ".option push",
    ".option norelax",
    "    la   gp, __global_pointer$",
    ".option pop",
    "    la   sp, __stack_top",
    "    tail start",
);

/// crt0 stage 2 (Rust): zero `.bss`, arm the heap and trap vector, run `main`,
/// then terminate. Reached via `tail start` from `_start` with `gp`/`sp` set.
///
/// # Safety
///
/// Called exactly once, by `_start`, on a freshly reset core. Touches raw
/// linker symbols and installs `mtvec`; not meant for any other caller.
#[no_mangle]
unsafe extern "C" fn start() -> ! {
    // Linker-provided region bounds. These are *addresses*, so we declare them
    // as zero-sized externs and take their address — never read the "value".
    extern "C" {
        static _sbss: u8;
        static _ebss: u8;
        static _sheap: u8;
        static _eheap: u8;
    }

    // Zero .bss in word strides (the link script 4-byte-aligns both bounds).
    let mut addr = core::ptr::addr_of!(_sbss) as usize;
    let bss_end = core::ptr::addr_of!(_ebss) as usize;
    while addr < bss_end {
        (addr as *mut u32).write_volatile(0);
        addr += 4;
    }

    // Arm the heap and the M-mode trap vector before any allocation or ecall.
    let heap_start = core::ptr::addr_of!(_sheap) as usize;
    let heap_end = core::ptr::addr_of!(_eheap) as usize;
    allocator::ALLOCATOR.init(heap_start, heap_end);
    hal::trap::init();

    // Hand off to the user program.
    extern "Rust" {
        fn main();
    }
    main();

    terminate(0)
}

/// Last-resort handler: report the panic on the console, then halt with a
/// non-zero code so the simulator exit status reflects the failure.
#[panic_handler]
fn on_panic(info: &PanicInfo<'_>) -> ! {
    use core::fmt::Write;
    let _ = writeln!(io::Stdout, "panic: {info}");
    halt(1)
}
