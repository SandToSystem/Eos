//! Hardware Abstraction Layer — the hardware-*dependent* surface of the
//! runtime (CLAUDE.md §5, the "HAL ⊂ Runtime" pillar). Everything here knows a
//! concrete MMIO address or a CSR name; the `stdlib`/`io` layers above depend
//! only on these interfaces, never on the addresses themselves.

pub mod clint;
pub mod csr;
pub mod halt;
pub mod irq;
pub mod mmio;
pub mod trap;
pub mod uart;

pub use halt::{halt, terminate};
pub use uart::{Status, Uart};
