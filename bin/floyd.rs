//! `floyd` — all-pairs shortest paths, cross-checked between two algorithms.
//!
//! On a fixed 6-node directed weighted graph, the Floyd–Warshall all-pairs
//! distance matrix is computed and then checked against an independent
//! array-based Dijkstra run from every source. Two famous algorithms must agree
//! on every entry. A handful of hand-verified distances (`dist[0] = [0, 7, 9,
//! 20, 20, 11]`) anchor the check so a shared bug can't pass silently. A
//! disagreement `panic!`s (→ `halt(1)`); success prints the matrix and falls
//! through to `halt(0)`.
//!
//! Exercises: 2D arrays, min/relaxation, and overflow-safe `INF` arithmetic
//! (done in `i64`, whose ops lower to compiler-builtins on RV32I).

#![no_std]
#![no_main]

use runtime::{print, println};

/// Number of vertices.
const N: usize = 6;
/// "No path" sentinel, small enough that `INF + INF` never overflows `i64`.
const INF: i64 = 1 << 40;

/// Build the directed weighted adjacency matrix. Diagonal is 0; missing edges
/// are `INF`.
fn graph() -> [[i64; N]; N] {
    // (from, to, weight)
    const EDGES: &[(usize, usize, i64)] = &[
        (0, 1, 7),
        (0, 2, 9),
        (0, 5, 14),
        (1, 2, 10),
        (1, 3, 15),
        (2, 3, 11),
        (2, 5, 2),
        (3, 4, 6),
        (4, 5, 9),
        (5, 4, 9),
    ];

    let mut w = [[INF; N]; N];
    for (i, row) in w.iter_mut().enumerate() {
        row[i] = 0;
    }
    for &(u, v, c) in EDGES {
        w[u][v] = c;
    }
    w
}

fn floyd_warshall(mut dist: [[i64; N]; N]) -> [[i64; N]; N] {
    for k in 0..N {
        for i in 0..N {
            for j in 0..N {
                let through = dist[i][k] + dist[k][j];
                if through < dist[i][j] {
                    dist[i][j] = through;
                }
            }
        }
    }
    dist
}

/// Classic O(V²) array-based Dijkstra (no priority queue) from `src`.
fn dijkstra(w: &[[i64; N]; N], src: usize) -> [i64; N] {
    let mut dist = [INF; N];
    let mut done = [false; N];
    dist[src] = 0;

    for _ in 0..N {
        // Pick the unfinished vertex with the smallest tentative distance.
        let mut u = None;
        let mut best = INF;
        for v in 0..N {
            if !done[v] && dist[v] < best {
                best = dist[v];
                u = Some(v);
            }
        }
        let Some(u) = u else { break };
        done[u] = true;

        for v in 0..N {
            if w[u][v] < INF {
                let cand = dist[u] + w[u][v];
                if cand < dist[v] {
                    dist[v] = cand;
                }
            }
        }
    }
    dist
}

#[no_mangle]
fn main() {
    let w = graph();
    let fw = floyd_warshall(w);

    // Cross-check: Floyd–Warshall vs Dijkstra from every source.
    for (src, fw_row) in fw.iter().enumerate() {
        let dj = dijkstra(&w, src);
        for j in 0..N {
            assert_eq!(
                fw_row[j], dj[j],
                "mismatch at dist[{src}][{j}]: floyd {} != dijkstra {}",
                fw_row[j], dj[j]
            );
        }
    }

    // Hand-verified anchor row from vertex 0.
    let expected_row0: [i64; N] = [0, 7, 9, 20, 20, 11];
    assert!(fw[0] == expected_row0, "row 0 mismatch: {:?}", fw[0]);

    // Print the full distance matrix for inspection.
    for row in &fw {
        for &d in row {
            if d >= INF {
                print!("   inf");
            } else {
                print!("{d:6}");
            }
        }
        println!();
    }
    println!("PASS floyd");
}
