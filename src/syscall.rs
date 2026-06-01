//! Syscall layer — the libgloss-style boundary between programs and the
//! runtime (Lab 3 §10).
//!
//! A program issues `ecall` with the syscall number in `a7` and arguments in
//! `a0..a6`; the M-mode trap handler ([`crate::hal::trap`]) catches the
//! `ECALL` trap and routes it to [`dispatch`]. This mirrors how a hosted libc
//! reaches the kernel — except "the kernel" here is three functions.
//!
//! The runtime's own [`println!`](crate::println)/[`exit`] take the *direct*
//! MMIO/`EBREAK` path and never `ecall`; this layer exists so the trap
//! mechanism has a worked example and so a program *can* go through it.

use crate::allocator::ALLOCATOR;
use crate::hal::halt::halt;
use crate::hal::uart::Uart;

/// Terminate with an exit code.
pub const SYS_EXIT: usize = 0;
/// Write a byte buffer to a file descriptor (only the UART exists).
pub const SYS_WRITE: usize = 1;
/// Move the program break (Newlib `_sbrk`).
pub const SYS_SBRK: usize = 2;

/// Error return for an unknown syscall (`-1`).
const ENOSYS: usize = usize::MAX;

/// Service one syscall. Called by the trap handler with `number` from `a7` and
/// `args` from `a0..a6`. Returns the value to place back in `a0`.
pub fn dispatch(number: usize, args: [usize; 7]) -> usize {
    match number {
        SYS_EXIT => halt(args[0] as i32),
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_SBRK => ALLOCATOR.sbrk(args[0] as isize) as usize,
        _ => ENOSYS,
    }
}

/// `write(fd, buf, len)` — every fd maps to the single UART. Returns `len`.
fn sys_write(_fd: usize, buf: *const u8, len: usize) -> usize {
    let uart = Uart::new();
    // SAFETY: the `ecall` site guarantees `buf` points at `len` readable bytes
    // (it comes from a live `&[u8]` in `write` below).
    let bytes = unsafe { core::slice::from_raw_parts(buf, len) };
    for &byte in bytes {
        uart.put_byte(byte);
    }
    len
}

/// Raw `ecall` trampoline: number in `a7`, three args in `a0..a2`, result from
/// `a0`. The trap handler preserves every register except `a0`, so the only
/// out-operand is the return value.
#[inline]
fn ecall(number: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    // SAFETY: `ecall` traps into our own handler, which saves/restores all
    // caller-saved registers; only `a0` carries a result back.
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") number,
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            options(nostack),
        );
    }
    ret
}

/// `write(2)` via the syscall path. Returns the number of bytes accepted.
pub fn write(fd: usize, buf: &[u8]) -> usize {
    ecall(SYS_WRITE, fd, buf.as_ptr() as usize, buf.len())
}

/// `exit(2)` via the syscall path. Traps into [`dispatch`], which halts.
pub fn exit(code: i32) -> ! {
    ecall(SYS_EXIT, code as usize, 0, 0);
    // `SYS_EXIT` halts the simulator and never returns.
    unreachable!()
}

/// `sbrk(2)` via the syscall path. Returns the previous program break, or
/// `usize::MAX as *mut u8` on failure.
pub fn sbrk(increment: isize) -> *mut u8 {
    ecall(SYS_SBRK, increment as usize, 0, 0) as *mut u8
}
