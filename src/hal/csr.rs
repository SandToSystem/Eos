//! Control & Status Register access (Zicsr).
//!
//! The macros below expand to a single CSR instruction each. The CSR name has
//! to be a compile-time string baked into the instruction mnemonic, so they
//! take a string literal (`csr_read!("mcause")`) and `concat!` it into the
//! asm template. They are the Rust counterpart of the C runtime's
//! `csr_read(name)` / `csr_write(name, val)` macro family.
//!
//! Bit-position constants for the CSRs the runtime touches live in the
//! [`mstatus`] / [`mie`] submodules; [`enable_global_interrupts`] and friends
//! wrap the common edits.

/// Read a CSR into a `usize`. `csr_read!("mcause")`.
#[macro_export]
macro_rules! csr_read {
    ($csr:literal) => {{
        let value: usize;
        // SAFETY: a CSR read has no memory effects and cannot fail for the
        // M-mode CSRs this runtime names.
        unsafe {
            core::arch::asm!(
                concat!("csrr {0}, ", $csr),
                out(reg) value,
                options(nomem, nostack),
            );
        }
        value
    }};
}

/// Write a `usize` into a CSR. `csr_write!("mtvec", addr)`.
#[macro_export]
macro_rules! csr_write {
    ($csr:literal, $value:expr) => {{
        let value: usize = $value;
        // SAFETY: writing an M-mode CSR is permitted in M-mode; we keep
        // `nostack` but allow memory ordering since e.g. mstatus.MIE acts as a
        // synchronisation point.
        unsafe {
            core::arch::asm!(
                concat!("csrw ", $csr, ", {0}"),
                in(reg) value,
                options(nostack),
            );
        }
    }};
}

/// Atomically set the masked bits of a CSR (`csrrs`). `csr_set!("mstatus", m)`.
#[macro_export]
macro_rules! csr_set {
    ($csr:literal, $mask:expr) => {{
        let mask: usize = $mask;
        // SAFETY: see `csr_write!`.
        unsafe {
            core::arch::asm!(
                concat!("csrs ", $csr, ", {0}"),
                in(reg) mask,
                options(nostack),
            );
        }
    }};
}

/// Atomically clear the masked bits of a CSR (`csrrc`). `csr_clear!("mie", m)`.
#[macro_export]
macro_rules! csr_clear {
    ($csr:literal, $mask:expr) => {{
        let mask: usize = $mask;
        // SAFETY: see `csr_write!`.
        unsafe {
            core::arch::asm!(
                concat!("csrc ", $csr, ", {0}"),
                in(reg) mask,
                options(nostack),
            );
        }
    }};
}

/// `mstatus` bit positions used by the runtime.
pub mod mstatus {
    /// Machine Interrupt Enable — the global async-interrupt gate.
    pub const MIE: usize = 1 << 3;
}

/// `mie` bit positions (machine interrupt-enable mask).
pub mod mie {
    /// Machine Software Interrupt Enable.
    pub const MSIE: usize = 1 << 3;
    /// Machine Timer Interrupt Enable.
    pub const MTIE: usize = 1 << 7;
    /// Machine External Interrupt Enable.
    pub const MEIE: usize = 1 << 11;
}

/// Unmask async interrupts globally (`mstatus.MIE = 1`).
///
/// Lab 3 wires this interface up but does not fire any async interrupt — the
/// ISS/RTL only deliver them from Lab 5 onward. The trap handler still works
/// for the single synchronous trap (ECALL) regardless.
#[inline]
pub fn enable_global_interrupts() {
    csr_set!("mstatus", mstatus::MIE);
}

/// Mask async interrupts globally (`mstatus.MIE = 0`).
#[inline]
pub fn disable_global_interrupts() {
    csr_clear!("mstatus", mstatus::MIE);
}
