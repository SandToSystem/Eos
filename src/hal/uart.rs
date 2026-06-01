//! Full-duplex UART driver — Snake SoC TUI I/O path.
//!
//! Register block at `0x1000_1000` (12 bytes), revised in the 2026-05-25 TUI
//! pivot to full-duplex:
//!
//! | Offset | Name     | Access | Notes                                      |
//! |--------|----------|--------|--------------------------------------------|
//! | `0x0`  | TX_DATA  | W      | low byte transmitted; sim does `putc`      |
//! | `0x4`  | STATUS   | R/W1C  | bit0 TX_BUSY (always 0 in sim), bit1 RX_VALID |
//! | `0x8`  | RX_DATA  | R      | pure peek — reading does **not** pop        |
//!
//! Popping the single-entry RX buffer is the explicit W1C write of
//! `RX_VALID` back to STATUS, mirroring the hardware contract exactly.

use super::mmio::Reg32;

/// Decoded view of the UART STATUS register.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Status(u32);

impl Status {
    /// Transmitter busy (always 0 in simulation — TX is instantaneous).
    pub const TX_BUSY: u32 = 1 << 0;
    /// A received byte is waiting in the single-entry RX buffer.
    pub const RX_VALID: u32 = 1 << 1;

    /// Raw register bits.
    #[inline]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Is the transmitter busy?
    #[inline]
    pub const fn is_tx_busy(self) -> bool {
        self.0 & Self::TX_BUSY != 0
    }

    /// Is a received byte available to pop?
    #[inline]
    pub const fn is_rx_valid(self) -> bool {
        self.0 & Self::RX_VALID != 0
    }
}

/// The Snake SoC UART. Zero-sized: it owns no state, only the knowledge of
/// where its registers live, so constructing one is free and any number of
/// handles refer to the same device.
#[derive(Clone, Copy, Default)]
pub struct Uart;

impl Uart {
    const TX_DATA: Reg32 = Reg32::new(0x1000_1000);
    const STATUS: Reg32 = Reg32::new(0x1000_1004);
    const RX_DATA: Reg32 = Reg32::new(0x1000_1008);

    /// Obtain a handle to the UART.
    #[inline]
    pub const fn new() -> Self {
        Uart
    }

    /// Transmit one byte (blocks only conceptually — TX is instant in sim).
    #[inline]
    pub fn put_byte(&self, byte: u8) {
        Self::TX_DATA.write(byte as u32);
    }

    /// Read the STATUS register.
    #[inline]
    pub fn status(&self) -> Status {
        Status(Self::STATUS.read())
    }

    /// Non-blocking receive: `Some(byte)` if one was waiting (and pop it),
    /// `None` otherwise.
    #[inline]
    pub fn try_get_byte(&self) -> Option<u8> {
        if !self.status().is_rx_valid() {
            return None;
        }
        // Pure peek, then W1C the RX_VALID bit to pop the buffer.
        let byte = (Self::RX_DATA.read() & 0xFF) as u8;
        Self::STATUS.write(Status::RX_VALID);
        Some(byte)
    }
}
