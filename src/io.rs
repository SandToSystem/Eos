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

use core::fmt::{self, Write};

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

/// Block until one byte arrives on the UART and return it.
#[inline]
pub fn getchar() -> u8 {
    Uart::new().get_byte()
}

/// Return a pending UART byte without blocking, or `None` if none is ready.
#[inline]
pub fn try_getchar() -> Option<u8> {
    Uart::new().try_get_byte()
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
