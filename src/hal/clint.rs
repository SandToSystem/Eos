//! CLINT driver — `mtime` / `mtimecmp` / `msip` (SiFive-compatible subset).
//!
//! Register block at `0x0200_0000` (64 KiB). The Snake SoC exposes a single
//! hart, so only index 0 of each per-hart register exists.
//!
//! | Address       | Name        | Width | Notes                          |
//! |---------------|-------------|-------|--------------------------------|
//! | `0x0200_0000` | msip[0]     | 32    | write 1 → pend machine SW IRQ  |
//! | `0x0200_4000` | mtimecmp[0] | 64    | timer compare                  |
//! | `0x0200_BFF8` | mtime       | 64    | free-running monotonic counter |

use super::mmio::Reg32;

const MSIP0: Reg32 = Reg32::new(0x0200_0000);
const MTIMECMP_LO: Reg32 = Reg32::new(0x0200_4000);
const MTIMECMP_HI: Reg32 = Reg32::new(0x0200_4004);
const MTIME_LO: Reg32 = Reg32::new(0x0200_BFF8);
const MTIME_HI: Reg32 = Reg32::new(0x0200_BFFC);

/// Read the 64-bit `mtime` counter.
///
/// `mtime` is two 32-bit MMIO words, so a naive read can tear when the low
/// word rolls over between the two loads. The standard re-read loop (read hi,
/// lo, hi again; retry if hi changed) makes the 64-bit snapshot atomic.
pub fn mtime() -> u64 {
    loop {
        let hi = MTIME_HI.read();
        let lo = MTIME_LO.read();
        let hi_again = MTIME_HI.read();
        if hi == hi_again {
            return (u64::from(hi) << 32) | u64::from(lo);
        }
    }
}

/// Program the timer-compare register. The machine timer interrupt pends
/// (`mip.MTIP`) while `mtime >= mtimecmp`.
///
/// Writing the 64-bit compare in two halves can transiently create a value
/// less than `mtime` and spuriously fire; the SiFive-recommended sequence
/// (set lo to all-ones first, write hi, then write lo) avoids that window.
pub fn set_mtimecmp(value: u64) {
    MTIMECMP_LO.write(u32::MAX);
    MTIMECMP_HI.write((value >> 32) as u32);
    MTIMECMP_LO.write(value as u32);
}

/// Pend (`pending = true`) or clear the machine software interrupt for hart 0.
pub fn set_software_interrupt(pending: bool) {
    MSIP0.write(pending as u32);
}
