//! Tabu search for the r-partition of a graph with the fewest edges inside
//! its parts (max r-cut). Deleting those inside edges leaves an r-partite
//! graph, whose clique number is at most r, so a partition with at most k
//! inside edges is an upper-bound certificate for edge clique interdiction.
//!
//! Standalone (no crates), so it builds with `rustc -O rcut_tabu.rs`.
//! Input: a text file "n m" then m lines "u v". Output: one line per run
//! and, at the end, the best partition as "best <inside> <p_0> ... <p_{n-1}>".
//!
//! usage: rcut_tabu FILE R ITERS RUNS SEED TARGET

use std::env;
use std::fs;

/// xorshift64*: deterministic and dependency free.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

struct State {
    part: Vec<usize>,
    /// cnt[v * r + j]: neighbours of v in part j.
    cnt: Vec<i64>,
    inside: i64,
}

fn init(adj: &[Vec<usize>], r: usize, part: Vec<usize>) -> State {
    let n = adj.len();
    let mut cnt = vec![0i64; n * r];
    for v in 0..n {
        for &u in &adj[v] {
            cnt[v * r + part[u]] += 1;
        }
    }
    let inside = (0..n).map(|v| cnt[v * r + part[v]]).sum::<i64>() / 2;
    State { part, cnt, inside }
}

fn relocate(s: &mut State, adj: &[Vec<usize>], r: usize, v: usize, j: usize) {
    let old = s.part[v];
    s.inside += s.cnt[v * r + j] - s.cnt[v * r + old];
    for &u in &adj[v] {
        s.cnt[u * r + old] -= 1;
        s.cnt[u * r + j] += 1;
    }
    s.part[v] = j;
}

/// One tabu run from a random start; returns the best partition seen.
fn tabu(adj: &[Vec<usize>], r: usize, iters: u64, rng: &mut Rng, target: i64) -> (i64, Vec<usize>) {
    let n = adj.len();
    let start: Vec<usize> = (0..n).map(|_| rng.below(r)).collect();
    let mut s = init(adj, r, start);
    let mut best = (s.inside, s.part.clone());
    let mut tabu_until = vec![0u64; n * r];
    let mut cand: Vec<(usize, usize)> = Vec::new();
    let mut since_best = 0u64;
    for it in 1..=iters {
        let mut bd = i64::MAX;
        cand.clear();
        for v in 0..n {
            let cur = s.part[v];
            for j in 0..r {
                if j == cur {
                    continue;
                }
                let d = s.cnt[v * r + j] - s.cnt[v * r + cur];
                // Aspiration: a tabu move is allowed when it beats the best.
                if tabu_until[v * r + j] > it && s.inside + d >= best.0 {
                    continue;
                }
                if d < bd {
                    bd = d;
                    cand.clear();
                }
                if d == bd {
                    cand.push((v, j));
                }
            }
        }
        if cand.is_empty() {
            continue;
        }
        let (v, j) = cand[rng.below(cand.len())];
        let cur = s.part[v];
        relocate(&mut s, adj, r, v, j);
        tabu_until[v * r + cur] = it + 10 + rng.below(n / 2) as u64;
        if s.inside < best.0 {
            best = (s.inside, s.part.clone());
            since_best = 0;
            if best.0 <= target {
                break;
            }
        } else {
            since_best += 1;
        }
        // A long stall: restart from a perturbed copy of the best.
        if since_best > 20 * n as u64 {
            let mut p = best.1.clone();
            for _ in 0..n / 6 {
                let v = rng.below(n);
                p[v] = rng.below(r);
            }
            s = init(adj, r, p);
            since_best = 0;
        }
    }
    best
}

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() != 7 {
        eprintln!("usage: rcut_tabu FILE R ITERS RUNS SEED TARGET");
        std::process::exit(2);
    }
    let text = fs::read_to_string(&a[1]).expect("read graph");
    let mut it = text.split_whitespace().map(|t| t.parse::<usize>().unwrap());
    let n = it.next().unwrap();
    let m = it.next().unwrap();
    let mut adj = vec![Vec::new(); n];
    for _ in 0..m {
        let u = it.next().unwrap();
        let v = it.next().unwrap();
        adj[u].push(v);
        adj[v].push(u);
    }
    let r: usize = a[2].parse().unwrap();
    let iters: u64 = a[3].parse().unwrap();
    let runs: usize = a[4].parse().unwrap();
    let mut rng = Rng(a[5]
        .parse::<u64>()
        .unwrap()
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        | 1);
    let target: i64 = a[6].parse().unwrap();
    let mut best: Option<(i64, Vec<usize>)> = None;
    for run in 0..runs {
        let b = tabu(&adj, r, iters, &mut rng, target);
        println!("run {run} inside {}", b.0);
        if best.as_ref().is_none_or(|x| b.0 < x.0) {
            best = Some(b);
        }
        if best.as_ref().unwrap().0 <= target {
            break;
        }
    }
    let (v, p) = best.unwrap();
    let ps: Vec<String> = p.iter().map(|x| x.to_string()).collect();
    println!("best {v} {}", ps.join(" "));
}
