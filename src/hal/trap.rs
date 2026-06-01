//! M-mode trap handling.
//!
//! `crt0` points `mtvec` at [`trap_entry`] in Direct mode before calling
//! `main`. On a trap the hardware jumps there with interrupts globally masked;
//! [`trap_entry`] is a naked function that spills the caller-saved registers
//! into a [`TrapFrame`] on the stack, hands a `&mut TrapFrame` to the Rust
//! [`dispatch`] routine, restores the (possibly edited) frame, and `mret`s.
//!
//! Two trap sources are handled: the synchronous `ECALL` from M-mode (`mcause`
//! = 11), and the machine *external* interrupt (`mcause` = interrupt-flag | 11),
//! which the IRQ aggregator raises on a UART RX byte — that is what makes RX
//! interrupt-driven (see [`crate::io::rx_isr`]). Any other cause is a contract
//! violation and halts loudly.

use crate::hal::halt::halt;
use crate::syscall;
use crate::{csr_read, csr_write};

/// The caller-saved register state spilled on a trap, in the exact word order
/// [`trap_entry`] stores it. `#[repr(C)]` pins the field offsets the assembly
/// hard-codes.
#[repr(C)]
pub struct TrapFrame {
    pub ra: usize, //  0
    pub t0: usize, //  4
    pub t1: usize, //  8
    pub t2: usize, // 12
    pub t3: usize, // 16
    pub t4: usize, // 20
    pub t5: usize, // 24
    pub t6: usize, // 28
    pub a0: usize, // 32
    pub a1: usize, // 36
    pub a2: usize, // 40
    pub a3: usize, // 44
    pub a4: usize, // 48
    pub a5: usize, // 52
    pub a6: usize, // 56
    pub a7: usize, // 60
}

/// Synchronous cause: environment call from M-mode.
const CAUSE_ECALL_FROM_M: usize = 11;
/// Asynchronous cause code: machine external interrupt (UART RX via the IRQ
/// aggregator). Distinguished from `ECALL` by the interrupt flag.
const CAUSE_MACHINE_EXTERNAL: usize = 11;
/// `mcause` interrupt flag (MSB) — set for async interrupts, clear for traps.
const CAUSE_INTERRUPT_FLAG: usize = 1 << (usize::BITS - 1);
/// Halt code for an unexpected trap (kind-coded, see CLAUDE.md §4).
const HALT_UNEXPECTED_TRAP: i32 = 130;

/// Install the trap vector. Must run before any `ecall`. `mtvec` low two bits
/// select the mode; 0 = Direct (all traps jump to `BASE`).
///
/// # Safety
///
/// Takes the address of [`trap_entry`], which the linker guarantees is
/// 4-byte aligned (so the mode bits stay clear). Call once, from `crt0`.
pub fn init() {
    let base = trap_entry as *const () as usize;
    csr_write!("mtvec", base); // Direct mode (mode bits already 0)
}

/// The Rust trap handler. Reached from [`trap_entry`] with a frame holding the
/// trapped context. Decodes `mcause`, services an `ECALL`, and writes the
/// syscall return value back into `frame.a0`.
#[no_mangle]
extern "C" fn rust_trap_handler(frame: &mut TrapFrame) {
    let cause = csr_read!("mcause");
    let is_interrupt = cause & CAUSE_INTERRUPT_FLAG != 0;
    let code = cause & !CAUSE_INTERRUPT_FLAG;

    if !is_interrupt && code == CAUSE_ECALL_FROM_M {
        // `mepc` points at the `ecall`; resume at the next instruction (no
        // compressed instructions on this ISA, so the stride is always 4).
        let mepc = csr_read!("mepc");
        csr_write!("mepc", mepc + 4);

        let args = [
            frame.a0, frame.a1, frame.a2, frame.a3, frame.a4, frame.a5, frame.a6,
        ];
        frame.a0 = syscall::dispatch(frame.a7, args);
        return;
    }

    if is_interrupt && code == CAUSE_MACHINE_EXTERNAL {
        // UART RX: drain received bytes into the console ring. `mepc` already
        // points at the interrupted instruction, so we just return and resume.
        crate::io::rx_isr();
        return;
    }

    // No other synchronous trap or async source is expected.
    halt(HALT_UNEXPECTED_TRAP);
}

/// Naked trap vector: spill caller-saved regs, call the Rust handler with a
/// pointer to the frame, reload, and return from the trap.
///
/// # Safety
///
/// Never call this from Rust — it is the hardware trap target installed in
/// `mtvec` by [`init`]. It assumes the architectural trap entry state (a valid
/// stack pointer, M-mode, `mepc`/`mcause` set by the trap) and ends in `mret`.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn trap_entry() -> ! {
    core::arch::naked_asm!(
        // Reserve a 16-word TrapFrame.
        "addi sp, sp, -64",
        "sw   ra,  0(sp)",
        "sw   t0,  4(sp)",
        "sw   t1,  8(sp)",
        "sw   t2, 12(sp)",
        "sw   t3, 16(sp)",
        "sw   t4, 20(sp)",
        "sw   t5, 24(sp)",
        "sw   t6, 28(sp)",
        "sw   a0, 32(sp)",
        "sw   a1, 36(sp)",
        "sw   a2, 40(sp)",
        "sw   a3, 44(sp)",
        "sw   a4, 48(sp)",
        "sw   a5, 52(sp)",
        "sw   a6, 56(sp)",
        "sw   a7, 60(sp)",
        // a0 = &mut TrapFrame (the stack slot we just filled).
        "mv   a0, sp",
        "call rust_trap_handler",
        // Reload — a0 may have been overwritten with the syscall return.
        "lw   ra,  0(sp)",
        "lw   t0,  4(sp)",
        "lw   t1,  8(sp)",
        "lw   t2, 12(sp)",
        "lw   t3, 16(sp)",
        "lw   t4, 20(sp)",
        "lw   t5, 24(sp)",
        "lw   t6, 28(sp)",
        "lw   a0, 32(sp)",
        "lw   a1, 36(sp)",
        "lw   a2, 40(sp)",
        "lw   a3, 44(sp)",
        "lw   a4, 48(sp)",
        "lw   a5, 52(sp)",
        "lw   a6, 56(sp)",
        "lw   a7, 60(sp)",
        "addi sp, sp, 64",
        "mret",
    )
}
