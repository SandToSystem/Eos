//! `snake_rt` — real-time Snake, the playable counterpart to `snake`.
//!
//! Unlike the turn-based `snake` (one move per keystroke, used as the
//! deterministic RX test fixture), this version advances *on a timer*: the
//! board steps forward every `STEP_US` units of the CLINT `mtime` counter while
//! the keyboard only steers. Input is polled non-blocking with `try_getchar()`,
//! so the snake keeps moving whether or not you press a key — a real arcade
//! feel. `w`/`a`/`s`/`d` steer, `q` (or Ctrl-C) quits.
//!
//! Food is placed pseudo-randomly (a fixed-seed LCG) at a free cell. On a
//! wall/self collision or quit the game prints a summary, checks its invariants,
//! and `halt(0)`s.
//!
//! Pace it with the `run-eos` runner, which syncs `mtime` to the host wall
//! clock so `STEP_US` is real microseconds regardless of machine speed:
//!
//! ```text
//!   cargo run --bin run-eos -- Eos/target/riscv32i-unknown-none-elf/release/snake_rt
//! ```

#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::VecDeque;

use runtime::hal::clint;
use runtime::io::try_getchar;
use runtime::{print, println};

/// Interior width: playable columns are `1..=W`.
const W: i32 = 12;
/// Interior height: playable rows are `1..=H`.
const H: i32 = 8;
/// `mtime` units between auto-advances. Under `run-eos` (which ties `mtime` to
/// the wall clock) this is ~microseconds, so 120 000 ≈ 120 ms ≈ 8 moves/sec.
const STEP_US: u64 = 120_000;

const QUIT: u8 = b'q';
const CTRL_C: u8 = 0x03;

type Cell = (i32, i32);

/// Fixed-seed LCG (glibc constants) for food placement — reproducible, and the
/// `*` lowers to `__mulsi3` on RV32I.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }
}

/// Map a key to a direction, ignoring a 180° reversal. Unknown keys keep the
/// current direction.
fn steer(key: u8, cur: Cell) -> Cell {
    let d = match key {
        b'w' | b'W' => (0, -1),
        b's' | b'S' => (0, 1),
        b'a' | b'A' => (-1, 0),
        b'd' | b'D' => (1, 0),
        _ => return cur,
    };
    if d.0 == -cur.0 && d.1 == -cur.1 {
        cur
    } else {
        d
    }
}

/// Pick a free cell for food (re-rolling off the snake's body).
fn place_food(rng: &mut Lcg, snake: &VecDeque<Cell>) -> Cell {
    loop {
        let x = 1 + (rng.next() % W as u32) as i32;
        let y = 1 + (rng.next() % H as u32) as i32;
        if !snake.iter().any(|&c| c == (x, y)) {
            return (x, y);
        }
    }
}

/// Draw the board in place (cursor home; every cell rewritten — flicker-free).
fn render(snake: &VecDeque<Cell>, food: Cell, score: u32) {
    print!("\x1b[H");
    println!("snake-rt  w/a/s/d move, q quit   score={score}");
    let head = *snake.front().unwrap();
    for y in 0..=(H + 1) {
        for x in 0..=(W + 1) {
            let ch = if x == 0 || x == W + 1 || y == 0 || y == H + 1 {
                '#'
            } else if (x, y) == head {
                '@'
            } else if snake.iter().any(|&c| c == (x, y)) {
                'o'
            } else if (x, y) == food {
                '*'
            } else {
                '.'
            };
            print!("{ch}");
        }
        println!();
    }
}

#[no_mangle]
fn main() {
    print!("\x1b[2J");

    let mut snake: VecDeque<Cell> = VecDeque::from([(5, 4), (4, 4), (3, 4)]);
    let mut dir: Cell = (1, 0);
    let mut score: u32 = 0;
    let mut rng = Lcg(0x00C0_FFEE);
    let mut food = place_food(&mut rng, &snake);

    render(&snake, food, score);

    let mut last = clint::mtime();
    let mut status = "quit";
    loop {
        // Non-blocking input: steer on the latest key, quit on q/Ctrl-C.
        if let Some(key) = try_getchar() {
            if key == QUIT || key == CTRL_C {
                break;
            }
            dir = steer(key, dir);
        }

        // Advance only once enough simulated time has elapsed.
        let now = clint::mtime();
        if now.wrapping_sub(last) < STEP_US {
            continue;
        }
        last = now;

        let head = *snake.front().unwrap();
        let new_head = (head.0 + dir.0, head.1 + dir.1);

        if new_head.0 < 1 || new_head.0 > W || new_head.1 < 1 || new_head.1 > H {
            status = "dead";
            break;
        }
        let eating = new_head == food;
        let tail = *snake.back().unwrap();
        let hit_self = snake.iter().any(|&c| c == new_head && (eating || c != tail));
        if hit_self {
            status = "dead";
            break;
        }

        snake.push_front(new_head);
        if eating {
            score += 1;
            food = place_food(&mut rng, &snake);
        } else {
            snake.pop_back();
        }

        render(&snake, food, score);
    }

    println!("snake-rt: score={score} status={status}");

    // Self-check the final state.
    assert_eq!(
        snake.len(),
        3 + score as usize,
        "length {} inconsistent with score {score}",
        snake.len()
    );
    for (i, &a) in snake.iter().enumerate() {
        assert!(
            a.0 >= 1 && a.0 <= W && a.1 >= 1 && a.1 <= H,
            "body cell {a:?} out of bounds"
        );
        for &b in snake.iter().skip(i + 1) {
            assert!(a != b, "duplicate body cell {a:?}");
        }
    }

    println!("PASS snake-rt");
}
