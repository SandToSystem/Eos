//! Volatile access to a fixed memory-mapped register.
//!
//! Every MMIO read/write on the Snake SoC bus must be `volatile` so the
//! compiler never elides, reorders, or coalesces it — the device, not the
//! program, observes the side effects. [`Reg32`] is the single place that
//! `unsafe` raw-pointer volatile access lives; the device drivers above it
//! (`uart`, `clint`) stay safe by constructing `Reg32` only from addresses
//! the SoC memory map guarantees are real registers.

/// A 32-bit memory-mapped register at a fixed physical address.
///
/// Word-sized accesses only — the Snake SoC AXI4-Lite subordinates see full
/// 32-bit beats (`WSTRB = 4'b1111`), so byte/halfword stores never reach a
/// device register directly (CLAUDE.md §4).
#[derive(Clone, Copy)]
pub struct Reg32 {
    addr: usize,
}

impl Reg32 {
    /// Bind a register to its physical address.
    ///
    /// `const` so device drivers can name their registers as associated
    /// constants. The caller asserts `addr` is a valid, word-aligned MMIO
    /// register for the lifetime of the program.
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self { addr }
    }

    /// Read the current 32-bit value.
    #[inline]
    pub fn read(self) -> u32 {
        // SAFETY: `addr` was promised to point at a real MMIO register by
        // whoever called `Reg32::new`; volatile keeps the access observable.
        unsafe { (self.addr as *const u32).read_volatile() }
    }

    /// Write a 32-bit value.
    #[inline]
    pub fn write(self, value: u32) {
        // SAFETY: see `read` — valid register address, volatile access.
        unsafe { (self.addr as *mut u32).write_volatile(value) }
    }
}
