//! `crc32` — CRC-32 (IEEE 802.3), self-checked against canonical vectors.
//!
//! Computes the reflected CRC-32 (polynomial `0xEDB88320`) two independent
//! ways — a bit-by-bit shift-register and a 256-entry table-driven loop — and
//! requires them to agree. Results are checked against well-known vectors,
//! including the standard check value `CRC32("123456789") == 0xCBF43926`. A
//! mismatch `panic!`s (→ `halt(1)`); success prints the check value and falls
//! through to `halt(0)`.
//!
//! Exercises: bitwise shifts/masks/xor and a stack-resident lookup table — no
//! heap, no multiply.

#![no_std]
#![no_main]

use runtime::println;

/// Reflected CRC-32 polynomial.
const POLY: u32 = 0xEDB8_8320;

/// Bit-by-bit CRC-32 shift register.
fn crc32_bitwise(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ POLY
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Build the 256-entry CRC-32 lookup table at runtime.
fn make_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut bit = 0;
        while bit < 8 {
            c = if c & 1 != 0 { (c >> 1) ^ POLY } else { c >> 1 };
            bit += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

/// Table-driven CRC-32.
fn crc32_table(table: &[u32; 256], data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = table[idx] ^ (crc >> 8);
    }
    !crc
}

#[no_mangle]
fn main() {
    let table = make_table();

    // (input, expected CRC-32) — canonical, widely published vectors.
    let vectors: &[(&[u8], u32)] = &[
        (b"", 0x0000_0000),
        (b"123456789", 0xCBF4_3926),
        (b"The quick brown fox jumps over the lazy dog", 0x414F_A339),
        (b"a", 0xE8B7_BE43),
    ];

    for &(data, expected) in vectors {
        let bitwise = crc32_bitwise(data);
        let tabled = crc32_table(&table, data);
        assert_eq!(
            bitwise, tabled,
            "bitwise {bitwise:#010x} != table {tabled:#010x}"
        );
        assert_eq!(
            bitwise, expected,
            "CRC-32 mismatch: got {bitwise:#010x}, expected {expected:#010x}"
        );
    }

    println!(
        "crc32: check value = 0x{:08X}",
        crc32_table(&table, b"123456789")
    );
    println!("PASS crc32");
}
