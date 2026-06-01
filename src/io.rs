//! Console I/O on top of the UART HAL.
//!
//! The §3 "printf HAL pattern": formatting is hardware-*independent* (it's just
//! [`core::fmt`]) while the byte sink, [`Stdout`], is hardware-*dependent* — it
//! funnels every byte through [`Uart::put_byte`]. Swapping the UART for any
//! other sink would not touch a line of formatting code.
//!
//! Idiomatically, the C runtime's bespoke `printf("%d", x)` becomes Rust's
//! type-checked [`println!`]`("{x}")`: the format string is validated at
//! compile time and the conversions come from `core`, so there is no custom
//! format parser to carry.

use core::cell::{Cell, UnsafeCell};
use core::fmt::{self, Write};

use crate::hal::irq;
use crate::hal::uart::Uart;

/// The console sink: writes UTF-8 bytes straight to the UART.
pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let uart = Uart::new();
        for &byte in s.as_bytes() {
            uart.put_byte(byte);
        }
        Ok(())
    }
}

/// Backing function for [`print!`]/[`println!`]. Public because the macros
/// expand to a call here, but not meant to be called directly.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    // Writing to the UART is infallible; the `Result` only exists to satisfy
    // the `fmt::Write` contract.
    let _ = Stdout.write_fmt(args);
}

// --- Interrupt-driven RX ---------------------------------------------------
//
// Received bytes are no longer polled out of the UART STATUS bit. Instead the
// UART RX edge raises a machine-external interrupt; the trap handler calls
// [`rx_isr`], which drains the UART into this single-producer/single-consumer
// ring. `getchar`/`try_getchar` are the consumer side.

/// RX ring capacity (power of two so the wrap is a mask).
const RX_CAP: usize = 64;

/// A lock-free SPSC byte ring. The ISR is the sole producer (writes `tail` and
/// `buf[tail]`); `getchar`/`try_getchar` are the sole consumer (write `head`,
/// read `buf[head]`) and run with interrupts masked, so the producer never
/// preempts a half-finished consume.
struct RxRing {
    buf: UnsafeCell<[u8; RX_CAP]>,
    head: Cell<usize>,
    tail: Cell<usize>,
}

// SAFETY: the Snake SoC is a single hart. The only "concurrency" is the RX
// interrupt preempting foreground code; the consumer masks interrupts while it
// touches the ring, and the producer (the ISR) cannot itself be preempted
// (interrupts are globally disabled on trap entry). So there is never truly
// concurrent access to any field.
unsafe impl Sync for RxRing {}

impl RxRing {
    const fn new() -> Self {
        Self {
            buf: UnsafeCell::new([0; RX_CAP]),
            head: Cell::new(0),
            tail: Cell::new(0),
        }
    }

    /// Producer (ISR context): enqueue a byte, dropping it if the ring is full.
    fn push(&self, byte: u8) {
        let tail = self.tail.get();
        let next = (tail + 1) % RX_CAP;
        if next == self.head.get() {
            return; // full — drop the byte rather than overwrite unread input
        }
        // SAFETY: the consumer never touches `buf[tail]`; this slot is ours.
        unsafe {
            (*self.buf.get())[tail] = byte;
        }
        self.tail.set(next);
    }

    /// Consumer: dequeue a byte if one is available.
    fn pop(&self) -> Option<u8> {
        let head = self.head.get();
        if head == self.tail.get() {
            return None; // empty
        }
        // SAFETY: the producer never overwrites an occupied slot (it stops one
        // short of `head`), so `buf[head]` is stable while we read it.
        let byte = unsafe { (*self.buf.get())[head] };
        self.head.set((head + 1) % RX_CAP);
        Some(byte)
    }
}

static RX: RxRing = RxRing::new();

/// UART RX interrupt service routine. Called from the trap handler on a machine
/// external interrupt: clear the latched edge first (so a byte arriving mid-
/// drain re-latches it rather than being stranded), then drain every buffered
/// byte into the ring.
pub(crate) fn rx_isr() {
    irq::clear_uart_rx();
    let uart = Uart::new();
    while let Some(byte) = uart.try_get_byte() {
        RX.push(byte);
    }
}

/// Block until a byte arrives on the UART and return it.
///
/// Bytes are delivered by [`rx_isr`] via the RX interrupt; this spins on `wfi`
/// (a NOP on this ISS) re-checking the ring between interrupts.
pub fn getchar() -> u8 {
    loop {
        if let Some(byte) = try_getchar() {
            return byte;
        }
        irq::wait_for_interrupt();
    }
}

/// Return a received byte without blocking, or `None` if none is buffered.
#[inline]
pub fn try_getchar() -> Option<u8> {
    irq::without_interrupts(|| RX.pop())
}

/// Print to the UART console. Same surface as `core`'s `print!`.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::io::_print(core::format_args!($($arg)*))
    };
}

/// Print to the UART console with a trailing newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::io::_print(core::format_args!("{}\n", core::format_args!($($arg)*)))
    };
}
