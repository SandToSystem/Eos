//! `echo_timer` — bare-metal UART echo with a CLINT `mtime` readout.
//!
//! Lab 3 Problem 3: practises every I/O primitive the Lab 8 Snake game needs
//! — blocking UART RX with W1C pop, UART TX, and `mtime` sampling — with far
//! simpler logic. Reads bytes, echoes them, prints `[mtime=N]` every few
//! bytes, and quits on `q` or Ctrl-C.

#![no_std]
#![no_main]

use runtime::hal::clint;
use runtime::io::getchar;
use runtime::{halt, print, println};

/// How many echoed bytes between `mtime` readouts.
const MTIME_EVERY: u32 = 4;
const QUIT: u8 = b'q';
const CTRL_C: u8 = 0x03;

#[no_mangle]
fn main() {
    println!("echo loop — q or Ctrl-C to quit");

    let mut echoed: u32 = 0;
    loop {
        let byte = getchar();
        if byte == QUIT || byte == CTRL_C {
            break;
        }

        // Echo the raw byte back out the UART.
        print!("{}", byte as char);
        echoed += 1;

        if echoed.is_multiple_of(MTIME_EVERY) {
            println!("[mtime={}]", clint::mtime());
        }
    }

    println!();
    println!("bye, total bytes echoed = {echoed}");
    halt(0);
}
