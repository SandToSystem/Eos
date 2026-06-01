//! `snake` — a turn-based Snake game driven entirely by UART RX.
//!
//! This is the input demo: every move is one byte read with `getchar()`
//! (blocking UART RX), so it exercises the RX path the other demos never touch.
//! `w`/`a`/`s`/`d` steer, `q` quits; the board is redrawn to UART (TX) after
//! each move. The game is *turn-based* — one input byte advances the snake one
//! cell — which keeps it fully deterministic: a given input script always
//! yields the same playthrough, so a host harness can feed a fixed sequence and
//! check the result.
//!
//! Food appears at a fixed sequence of cells (no RNG, for reproducibility). On
//! `q` or a wall/self collision the game ends, prints a summary line, checks its
//! own invariants (length, distinct in-bounds cells), and `halt(0)`s; a broken
//! invariant `panic!`s (→ `halt(1)`).

#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::VecDeque;

use runtime::io::getchar;
use runtime::{print, println};

/// Interior width: playable columns are `1..=W` (column 0 and `W+1` are walls).
const W: i32 = 12;
/// Interior height: playable rows are `1..=H`.
const H: i32 = 8;
/// Safety bound on turns, in case no `q`/collision ever arrives.
const MAX_TURNS: u32 = 1000;

/// Quit / interrupt keys.
const QUIT: u8 = b'q';
const CTRL_C: u8 = 0x03;

type Cell = (i32, i32);

/// Food positions, eaten in this order. After the last one the board has no
/// more food (head can no longer score).
const FOODS: &[Cell] = &[(8, 4), (8, 2), (4, 2)];

/// Map a key to a direction, ignoring a 180° reversal (which would run the
/// snake straight back into its own neck). Unknown keys keep the current dir.
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

/// Draw the board: `#` walls, `@` head, `o` body, `*` food, `.` empty.
fn render(snake: &VecDeque<Cell>, food: Cell, score: u32) {
    println!("score={score}");
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
    println!("snake: w/a/s/d to move, q to quit");

    // Head first. Start length 3, laid out horizontally, moving right.
    let mut snake: VecDeque<Cell> = VecDeque::from([(5, 4), (4, 4), (3, 4)]);
    let mut dir: Cell = (1, 0);
    let mut score: u32 = 0;
    let mut next_food = 0usize;
    let mut food = FOODS[next_food];

    render(&snake, food, score);

    let mut status = "quit";
    let mut turn = 0u32;
    loop {
        turn += 1;
        if turn > MAX_TURNS {
            status = "timeout";
            break;
        }

        let key = getchar();
        if key == QUIT || key == CTRL_C {
            break; // status stays "quit"
        }

        dir = steer(key, dir);
        let head = *snake.front().unwrap();
        let new_head = (head.0 + dir.0, head.1 + dir.1);

        // Wall collision.
        if new_head.0 < 1 || new_head.0 > W || new_head.1 < 1 || new_head.1 > H {
            status = "dead";
            break;
        }

        // Eating grows the snake (tail stays); otherwise the tail follows.
        let eating = new_head == food;
        let tail = *snake.back().unwrap();

        // Self collision: hitting any body cell, except the tail when it is
        // about to vacate (i.e. when not growing).
        let hit_self = snake
            .iter()
            .any(|&c| c == new_head && (eating || c != tail));
        if hit_self {
            status = "dead";
            break;
        }

        snake.push_front(new_head);
        if eating {
            score += 1;
            next_food += 1;
            food = if next_food < FOODS.len() {
                FOODS[next_food]
            } else {
                (-1, -1) // off-board: no more food
            };
        } else {
            snake.pop_back();
        }

        render(&snake, food, score);
    }

    println!("snake: score={score} status={status}");

    // --- Self-check the final state. ---
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

    println!("PASS snake");
}
