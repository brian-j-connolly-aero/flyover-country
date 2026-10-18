//! Tabu search for a map of V(G) onto the vertices of a small K4-free
//! pattern graph F with the fewest "bad" edges: edges of G whose ends land on
//! the same pattern vertex or on two non-adjacent ones. Deleting the bad edges
//! leaves a graph that maps homomorphically into F, so it is K4-free like F.
//! With F = K3 this is rcut_tabu's max 3-cut. A 4-chromatic F (the wheels W5
//! and W7, the complement of C7, the Moser spindle) also reaches K4-free
//! remainders that are not 3-partite, which a 3-partition cannot.
//!
//! The moves relocate one vertex; tabu tenure, aspiration and perturbed
//! restarts are as in rcut_tabu. Every F listed contains a triangle, so the
//! first run can start from a 3-partition (START) placed on one of its
//! triangles; the other runs start at random.
//!
//! Standalone (no crates): `rustc -O ftabu.rs`. Input: a text file "n m"
//! then m lines "u v". Output: one line per run and, at the end,
//! "best <bad edges> <pattern vertex of v_0> ... <of v_{n-1}>".
//!
//! usage: ftabu GRAPH PATTERN ITERS RUNS SEED TARGET [START]
//! PATTERN is one of k3, w5, w7, c7bar, moser.

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

/// The pattern's vertex count and edge list.
fn pattern(name: &str) -> (usize, Vec<(usize, usize)>) {
    let cycle = |k: usize| (0..k).map(move |i| (i, (i + 1) % k));
    match name {
        "k3" => (3, vec![(0, 1), (1, 2), (0, 2)]),
        // C5 or C7 on 0..k-1 plus an apex k joined to all of them.
        "w5" => (6, cycle(5).chain((0..5).map(|i| (i, 5))).collect()),
        "w7" => (8, cycle(7).chain((0..7).map(|i| (i, 7))).collect()),
        // i ~ j iff j - i is 2 or 3 mod 7: the complement of C7.
        "c7bar" => (
            7,
            (0..7)
                .flat_map(|i| [(i, (i + 2) % 7), (i, (i + 3) % 7)])
                .collect(),
        ),
        // Two rhombi (two triangles sharing an edge) from 0, tips 3 and 6 joined.
        "moser" => (
            7,
            vec![
                (0, 1),
                (0, 2),
                (1, 2),
                (1, 3),
                (2, 3),
                (0, 4),
                (0, 5),
                (4, 5),
                (4, 6),
                (5, 6),
                (3, 6),
            ],
        ),
        _ => panic!("unknown pattern {name}"),
    }
}

struct State {
    part: Vec<usize>,
    /// cnt[v * r + j]: neighbours of v on pattern vertex j.
    cnt: Vec<i64>,
    bad: i64,
}

fn init(adj: &[Vec<usize>], cost: &[i64], r: usize, part: Vec<usize>) -> State {
    let n = adj.len();
    let mut cnt = vec![0i64; n * r];
    for v in 0..n {
        for &u in &adj[v] {
            cnt[v * r + part[u]] += 1;
        }
    }
    let bad = (0..n)
        .map(|v| {
            (0..r)
                .map(|j| cnt[v * r + j] * cost[part[v] * r + j])
                .sum::<i64>()
        })
        .sum::<i64>()
        / 2;
    State { part, cnt, bad }
}

fn delta(s: &State, cost: &[i64], r: usize, v: usize, j: usize) -> i64 {
    let cur = s.part[v];
    (0..r)
        .map(|c| s.cnt[v * r + c] * (cost[j * r + c] - cost[cur * r + c]))
        .sum()
}

fn relocate(s: &mut State, adj: &[Vec<usize>], cost: &[i64], r: usize, v: usize, j: usize) {
    let old = s.part[v];
    s.bad += delta(s, cost, r, v, j);
    for &u in &adj[v] {
        s.cnt[u * r + old] -= 1;
        s.cnt[u * r + j] += 1;
    }
    s.part[v] = j;
}

/// One tabu run from `start`; returns the best map seen.
fn tabu(
    adj: &[Vec<usize>],
    cost: &[i64],
    r: usize,
    start: Vec<usize>,
    iters: u64,
    rng: &mut Rng,
    target: i64,
) -> (i64, Vec<usize>) {
    let n = adj.len();
    let mut s = init(adj, cost, r, start);
    let mut best = (s.bad, s.part.clone());
    let mut tabu_until = vec![0u64; n * r];
    let mut cand: Vec<(usize, usize)> = Vec::new();
    let mut since_best = 0u64;
    for it in 1..=iters {
        let mut bd = i64::MAX;
        cand.clear();
        for v in 0..n {
            for j in 0..r {
                if j == s.part[v] {
                    continue;
                }
                let d = delta(&s, cost, r, v, j);
                // Aspiration: a tabu move is allowed when it beats the best.
                if tabu_until[v * r + j] > it && s.bad + d >= best.0 {
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
        relocate(&mut s, adj, cost, r, v, j);
        tabu_until[v * r + cur] = it + 10 + rng.below(n / 2) as u64;
        if s.bad < best.0 {
            best = (s.bad, s.part.clone());
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
            s = init(adj, cost, r, p);
            since_best = 0;
        }
    }
    best
}

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() != 7 && a.len() != 8 {
        eprintln!("usage: ftabu GRAPH PATTERN ITERS RUNS SEED TARGET [START]");
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
    let (r, fe) = pattern(&a[2]);
    let mut fadj = vec![false; r * r];
    for &(i, j) in &fe {
        assert!(i != j && !fadj[i * r + j], "pattern edge list");
        fadj[i * r + j] = true;
        fadj[j * r + i] = true;
    }
    // F must be K4-free, or the remainder could keep a K4.
    for p in 0..r {
        for q in p + 1..r {
            for x in q + 1..r {
                for y in x + 1..r {
                    let s = [p, q, x, y];
                    let all = (0..4).all(|i| (i + 1..4).all(|j| fadj[s[i] * r + s[j]]));
                    assert!(!all, "pattern {} has a K4", a[2]);
                }
            }
        }
    }
    let cost: Vec<i64> = (0..r * r).map(|ij| i64::from(!fadj[ij])).collect();
    let tri = (0..r)
        .flat_map(|p| (p + 1..r).flat_map(move |q| (q + 1..r).map(move |x| [p, q, x])))
        .find(|t| fadj[t[0] * r + t[1]] && fadj[t[0] * r + t[2]] && fadj[t[1] * r + t[2]])
        .expect("pattern has a triangle");
    let iters: u64 = a[3].parse().unwrap();
    let runs: usize = a[4].parse().unwrap();
    let mut rng = Rng(a[5]
        .parse::<u64>()
        .unwrap()
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        | 1);
    let target: i64 = a[6].parse().unwrap();
    let start3: Option<Vec<usize>> = a.get(7).map(|path| {
        let t = fs::read_to_string(path).expect("read start");
        let line = t
            .lines()
            .find(|l| l.starts_with("best "))
            .expect("best line");
        let p: Vec<usize> = line
            .split_whitespace()
            .skip(2)
            .map(|x| tri[x.parse::<usize>().unwrap()])
            .collect();
        assert_eq!(p.len(), n);
        p
    });
    let mut best: Option<(i64, Vec<usize>)> = None;
    for run in 0..runs {
        let start = match (&start3, run) {
            (Some(p), 0) => p.clone(),
            _ => (0..n).map(|_| rng.below(r)).collect(),
        };
        let b = tabu(&adj, &cost, r, start, iters, &mut rng, target);
        let used = (0..r).filter(|j| b.1.contains(j)).count();
        println!("run {run} bad {} pattern vertices used {used}", b.0);
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
