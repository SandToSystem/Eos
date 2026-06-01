//! IRQ aggregator driver + machine-interrupt setup for interrupt-driven RX.
//!
//! The Snake SoC routes the UART RX-valid edge through a tiny aggregator (NOT a
//! PLIC) at `0x1000_4000`, which drives the single external-interrupt wire
//! (`mip.MEIP`). [`init`] enables that source and unmasks `mie.MEIE` +
//! `mstatus.MIE`, after which a received byte traps into the M-mode handler
//! ([`crate::hal::trap`]) instead of being polled out of the UART STATUS bit.
//!
//! ```text
//!   0x1000_4000  IRQ_PENDING (R/W1C) ; bit 0 = UART RX-valid edge (latched)
//!   0x1000_4004  IRQ_ENABLE  (R/W)   ; bit 0 = route UART RX to MEIP
//! ```

use super::mmio::Reg32;
use crate::hal::csr::{mie, mstatus};
use crate::{csr_clear, csr_read, csr_set};

const PENDING: Reg32 = Reg32::new(0x1000_4000);
const ENABLE: Reg32 = Reg32::new(0x1000_4004);

/// Bit 0 — the UART RX source.
const BIT_UART_RX: u32 = 1 << 0;

/// Route the UART RX edge to the external-interrupt wire and unmask machine
/// external interrupts globally. Call once, from `crt0`, after `trap::init`.
pub fn init() {
    ENABLE.write(BIT_UART_RX); // aggregator: enable the UART RX source
    csr_set!("mie", mie::MEIE); // CPU: enable the machine-external interrupt
    csr_set!("mstatus", mstatus::MIE); // CPU: global interrupt enable
}

/// Clear (write-1-to-clear) the latched UART RX edge, deasserting `MEIP` once
/// the buffered byte has been drained.
#[inline]
pub fn clear_uart_rx() {
    PENDING.write(BIT_UART_RX);
}

/// Run `f` with machine interrupts masked, restoring the previous `mstatus.MIE`
/// afterwards. Used to make the RX ring's consumer side safe against the ISR
/// (the producer) preempting it mid-update.
#[inline]
pub fn without_interrupts<R>(f: impl FnOnce() -> R) -> R {
    let was_enabled = csr_read!("mstatus") & mstatus::MIE != 0;
    csr_clear!("mstatus", mstatus::MIE);
    let result = f();
    if was_enabled {
        csr_set!("mstatus", mstatus::MIE);
    }
    result
}

/// Wait for an interrupt. `wfi` is a NOP on this ISS, so this is effectively a
/// spin — but correctness comes from the RX interrupt filling the ring between
/// iterations of the caller's loop, not from the core actually sleeping.
#[inline]
pub fn wait_for_interrupt() {
    // SAFETY: `wfi` has no memory or register effects (a hint instruction).
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack));
    }
}
