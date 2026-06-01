# Eos — Snake SoC bare-metal runtime (Rust)

An idiomatic-Rust rewrite of the `comporg-labs/runtime` C runtime: a `no_std`
RV32I runtime that bridges a student program's `fn main` to the Snake SoC
hardware. It keeps the **Snake SoC platform semantics** (direct MMIO UART,
`EBREAK` voluntary halt, Model B memory at `0x8000_0000`, RV32I_Zicsr with no M
extension) while replacing the hand-rolled C `printf`/`malloc`/crt0 with their
idiomatic Rust counterparts: `core::fmt`, a `GlobalAlloc` bump allocator, a
`global_asm!` crt0, a `#[panic_handler]`, and a naked-function trap vector.

## Layering (HAL ⊂ Runtime, CLAUDE.md §5)

```
  bin/*.rs          user main()                   — application
  io / allocator    println!, Box, Vec            — hardware-INDEPENDENT stdlib
  hal/*             uart, clint, csr, halt, trap   — hardware-DEPENDENT HAL
  ──────────────────────────────────────────────────────────────────
  Snake SoC MMIO + Zicsr CSRs
```

Everything in `io`/`allocator` depends only on HAL *interfaces*, never on an
MMIO address — the same split that lets the formatting and allocation logic be
reasoned about independently of the device.

## Layout

| Path                | Purpose                                                        |
|---------------------|---------------------------------------------------------------|
| `link.ld`           | Model B linker script — single 64 MiB DRAM region @ `0x8000_0000`. |
| `build.rs`          | Stages `link.ld` into `OUT_DIR` for the linker.               |
| `.cargo/config.toml`| Target (`riscv32i-unknown-none-elf`) + `-T link.ld`.          |
| `src/lib.rs`        | crt0 (`_start` asm → `start`), `terminate`, panic handler.    |
| `src/hal/mmio.rs`   | `Reg32` — the one place volatile MMIO `unsafe` lives.         |
| `src/hal/uart.rs`   | Full-duplex UART (`put_byte` / `get_byte` / `try_get_byte`).  |
| `src/hal/halt.rs`   | `halt` / `terminate` via `EBREAK` (exit code in `a0`).        |
| `src/hal/clint.rs`  | `mtime` / `mtimecmp` / `msip`.                                |
| `src/hal/csr.rs`    | `csr_read!`/`csr_write!`/`csr_set!`/`csr_clear!` + IRQ gates. |
| `src/hal/trap.rs`   | `mtvec` setup, naked `trap_entry`, `rust_trap_handler`.       |
| `src/allocator.rs`  | Bump allocator (`GlobalAlloc`) + Newlib-style `sbrk`.         |
| `src/io.rs`         | `Stdout` (`fmt::Write`), `print!`/`println!`, `getchar`.      |
| `src/syscall.rs`    | `ecall` dispatch (`write`/`exit`/`sbrk`) — libgloss example.  |
| `bin/*.rs`          | Example programs (see below).                                 |

## Build

```bash
cargo build --release          # all example bins
cargo build --release --bin hello
cargo clippy --release         # lint
```

Artifacts land in `target/riscv32i-unknown-none-elf/release/<name>` as RV32
ELF executables with entry `0x8000_0000`. Disassemble / make a Verilog hex:

```bash
riscv64-unknown-elf-objdump -d target/riscv32i-unknown-none-elf/release/hello
riscv64-unknown-elf-objcopy -O verilog <elf> hello.hex
```

> RV32I only — `*` / `/` lower to compiler-builtins (`__mulsi3` / `__divsi3`),
> so no `M`-extension instruction is ever emitted. Verified across all bins.

## Example programs

| Bin           | Exercises                                                       |
|---------------|-----------------------------------------------------------------|
| `hello`       | `println!` → UART; `return` → `terminate(0)`.                   |
| `msort`       | Recursive merge sort over `Vec` (heap, recursion, formatting).  |
| `matmul`      | 8×8 matmul; `*` → software `__mulsi3`.                          |
| `malloc_test` | Bump-allocator contract: alignment, OOM-null, base reuse.       |
| `echo_timer`  | UART RX/TX echo + CLINT `mtime` readout (Lab 3 Problem 3).      |

## Writing a program

```rust
#![no_std]
#![no_main]
use runtime::println;

#[no_mangle]
fn main() {
    println!("Hello, Snake SoC!");
}
```

The runtime provides `_start`, the panic handler, and the global allocator; the
program supplies `fn main`. Return a non-zero status with `runtime::halt(code)`
or `runtime::syscall::exit(code)`.

## Notes

- `mtvec` is armed in Direct mode; Lab 3 delivers only the synchronous `ECALL`
  trap (`mcause = 11`). Async interrupt plumbing (`mie`/`mstatus.MIE`) is in
  place but not fired until Lab 5.
- The bump allocator aligns to each request's `Layout` (no `(size+3)&~3` hack)
  and reclaims the whole heap once the last live allocation is freed.
- Single-threaded: the allocator's `Cell` state and its `Sync` impl are sound
  under that invariant alone (CLAUDE.md §8).
