//! Voluntary halt — terminate the simulation via `EBREAK`.
//!
//! The Snake SoC has no Halt MMIO register (removed 2026-05-24). Instead,
//! `EBREAK` is the voluntary-halt instruction: the simulator observes it at
//! the WB commit boundary and exits with `a0 & 0xFF`
//! (`plans/privileged-arch-plan.md` §2.1).

use core::arch::asm;

/// Halt the simulator with `code` (only the low byte is observed).
///
/// On real hardware with no sim backend the `ebreak` would trap; the trailing
/// spin loop pins the CPU so control never falls through into stray bytes.
#[inline(never)]
pub fn halt(code: i32) -> ! {
    // SAFETY: `ebreak` has no memory effects; we place `code` in a0 and never
    // expect to return. `nomem`/`nostack` let the optimiser around it freely.
    unsafe {
        asm!("ebreak", in("a0") code, options(nomem, nostack));
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Terminate wrapper used by `crt0` after `main` returns. Semantically
/// identical to [`halt`]; kept as a distinct entry point so the startup path
/// reads cleanly.
#[inline]
pub fn terminate(code: i32) -> ! {
    halt(code)
}
