//! `hanoi` — Towers of Hanoi, solved recursively and then validated.
//!
//! Solves the 10-disk puzzle, recording every `(from, to)` move in a heap
//! `Vec`. Two invariants are then checked: the move count equals 2^10 − 1 =
//! 1023, and *replaying* the moves on three peg stacks is legal at every step
//! (never place a larger disk on a smaller one) and ends with all disks stacked
//! in order on the target peg. A violation `panic!`s (→ `halt(1)`); success
//! prints the move count and falls through to `halt(0)`.
//!
//! Exercises: recursion (depth 10), heap allocation of the move list, and
//! `Vec`-as-stack manipulation.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use runtime::println;

/// Number of disks.
const DISKS: u32 = 10;

/// A move records the source and destination peg indices (0, 1, 2).
type Move = (usize, usize);

fn solve(n: u32, from: usize, to: usize, via: usize, moves: &mut Vec<Move>) {
    if n == 0 {
        return;
    }
    solve(n - 1, from, via, to, moves);
    moves.push((from, to));
    solve(n - 1, via, to, from, moves);
}

/// Replay `moves` on three peg stacks, asserting every move is legal, and
/// return the final peg configuration.
fn replay(disks: u32, moves: &[Move]) -> [Vec<u32>; 3] {
    // Peg 0 starts with disks largest..=1 on top being smallest (top of stack).
    let mut pegs: [Vec<u32>; 3] = [
        (1..=disks).rev().collect(), // largest at bottom, smallest on top
        Vec::new(),
        Vec::new(),
    ];

    for &(from, to) in moves {
        let disk = pegs[from]
            .pop()
            .unwrap_or_else(|| panic!("illegal move: source peg {from} is empty"));
        if let Some(&top) = pegs[to].last() {
            assert!(
                disk < top,
                "illegal move: disk {disk} onto smaller disk {top} (peg {to})"
            );
        }
        pegs[to].push(disk);
    }
    pegs
}

#[no_mangle]
fn main() {
    let mut moves = Vec::new();
    solve(DISKS, 0, 2, 1, &mut moves);

    let expected = (1u64 << DISKS) - 1;
    assert_eq!(
        moves.len() as u64,
        expected,
        "move count: got {}, expected {expected}",
        moves.len()
    );

    let pegs = replay(DISKS, &moves);
    assert!(pegs[0].is_empty(), "source peg not empty at end");
    assert!(pegs[1].is_empty(), "spare peg not empty at end");

    // Target peg must hold all disks, largest at the bottom.
    let target: Vec<u32> = (1..=DISKS).rev().collect();
    assert!(pegs[2] == target, "target peg not correctly stacked");

    println!("hanoi: {DISKS} disks solved in {} moves", moves.len());
    println!("PASS hanoi");
}
