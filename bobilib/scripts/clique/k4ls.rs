//! Local search for a small edge K4-transversal: a set D of edges meeting
//! every K4 of G. G - D is then K4-free, so a transversal with |D| <= k is an
//! upper-bound certificate of value <= 3 for edge clique interdiction. Unlike
//! rcut_tabu it is not restricted to inside edges of a 3-partition, so it can
//! find K4-free remainders that are not 3-partite.
//!
//! The search is NuMVC-style weighted hitting set (Cai et al., JAIR 2013),
//! with K4s as the sets to hit: at a fixed |D| it swaps one edge out (least
//! weighted loss) and one edge of a random unhit K4 in (most weighted gain),
//! then raises the weight of every unhit K4. When every K4 is hit it records
//! D and drops one more edge. Weights decay now and then so that old
//! conflicts are forgotten. Tabu: an edge just put in cannot leave for TT
//! steps, and an edge just taken out cannot come back at once. Weights are
//! scaled by 0.3 whenever their mean exceeds GAMMA (NuMVC's forgetting rule).
//!
//! Standalone (no crates): `rustc -O k4ls.rs`. Input: a text file "n m" then
//! m lines "u v" (n <= 64), and an rcut_tabu output whose "best" line gives
//! the start partition (D starts as its inside edges). Output: progress
//! lines, "found <|D|>" for every new best transversal, and the edges of the
//! first one with |D| <= TARGET as "transversal <|D|> u-v ...".
//!
//! usage: k4ls GRAPH START_PARTITION TARGET SECONDS SEED GAMMA

use std::env;
use std::fs;
use std::time::Instant;

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

const TT: u64 = 7;
const REPORT_EVERY: u64 = 1_000_000;

struct Search {
    quads: Vec<[u32; 6]>,
    inc: Vec<Vec<u32>>,
    in_d: Vec<bool>,
    /// cnt[q]: edges of K4 q that are in D.
    cnt: Vec<u8>,
    w: Vec<u64>,
    /// loss[e] for e in D: weight of the K4s that only e hits.
    /// gain[e] for e not in D: weight of the unhit K4s through e.
    score: Vec<u64>,
    unhit: Vec<u32>,
    pos: Vec<u32>,
    d: Vec<u32>,
    dpos: Vec<u32>,
    stamp: Vec<u64>,
}

impl Search {
    fn hit_mark(&mut self, q: u32) {
        let p = self.pos[q as usize] as usize;
        let last = *self.unhit.last().unwrap();
        self.unhit[p] = last;
        self.pos[last as usize] = p as u32;
        self.unhit.pop();
        self.pos[q as usize] = u32::MAX;
    }

    fn unhit_mark(&mut self, q: u32) {
        self.pos[q as usize] = self.unhit.len() as u32;
        self.unhit.push(q);
    }

    fn add(&mut self, e: u32, step: u64) {
        debug_assert!(!self.in_d[e as usize]);
        self.in_d[e as usize] = true;
        self.dpos[e as usize] = self.d.len() as u32;
        self.d.push(e);
        self.score[e as usize] = 0;
        for i in 0..self.inc[e as usize].len() {
            let q = self.inc[e as usize][i];
            let wq = self.w[q as usize];
            self.cnt[q as usize] += 1;
            match self.cnt[q as usize] {
                1 => {
                    self.hit_mark(q);
                    for &f in &self.quads[q as usize] {
                        if f != e {
                            self.score[f as usize] -= wq;
                        }
                    }
                    self.score[e as usize] += wq;
                }
                2 => {
                    let g = self.quads[q as usize]
                        .iter()
                        .copied()
                        .find(|&f| f != e && self.in_d[f as usize])
                        .unwrap();
                    self.score[g as usize] -= wq;
                }
                _ => {}
            }
        }
        self.stamp[e as usize] = step;
    }

    fn remove(&mut self, e: u32, step: u64) {
        debug_assert!(self.in_d[e as usize]);
        self.in_d[e as usize] = false;
        let p = self.dpos[e as usize] as usize;
        let last = *self.d.last().unwrap();
        self.d[p] = last;
        self.dpos[last as usize] = p as u32;
        self.d.pop();
        self.score[e as usize] = 0;
        for i in 0..self.inc[e as usize].len() {
            let q = self.inc[e as usize][i];
            let wq = self.w[q as usize];
            self.cnt[q as usize] -= 1;
            match self.cnt[q as usize] {
                0 => {
                    self.unhit_mark(q);
                    for &f in &self.quads[q as usize] {
                        self.score[f as usize] += wq;
                    }
                }
                1 => {
                    let g = self.quads[q as usize]
                        .iter()
                        .copied()
                        .find(|&f| self.in_d[f as usize])
                        .unwrap();
                    self.score[g as usize] += wq;
                }
                _ => {}
            }
        }
        self.stamp[e as usize] = step;
    }

    /// Recomputes every score from scratch (after a weight decay).
    fn rescore(&mut self) {
        self.score.iter_mut().for_each(|s| *s = 0);
        for (q, quad) in self.quads.iter().enumerate() {
            match self.cnt[q] {
                0 => quad
                    .iter()
                    .for_each(|&f| self.score[f as usize] += self.w[q]),
                1 => {
                    let g = quad
                        .iter()
                        .copied()
                        .find(|&f| self.in_d[f as usize])
                        .unwrap();
                    self.score[g as usize] += self.w[q];
                }
                _ => {}
            }
        }
    }

    /// The edge of D with the least loss that was not put in during the last TT steps
    /// (ties go to the edge that has been in D longest).
    fn pick_out(&self, rng: &mut Rng, step: u64) -> u32 {
        let mut best = (u64::MAX, u64::MAX, 0u32);
        for &e in &self.d {
            let st = self.stamp[e as usize];
            if step.saturating_sub(st) < TT && step > TT {
                continue;
            }
            if (self.score[e as usize], st) < (best.0, best.1) {
                best = (self.score[e as usize], st, e);
            }
        }
        if best.0 == u64::MAX {
            self.d[rng.below(self.d.len())]
        } else {
            best.2
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(
        args.len() == 7,
        "usage: k4ls GRAPH START_PARTITION TARGET SECONDS SEED GAMMA"
    );
    let text = fs::read_to_string(&args[1]).expect("graph file");
    let mut it = text.split_whitespace().map(|t| t.parse::<usize>().unwrap());
    let (n, m) = (it.next().unwrap(), it.next().unwrap());
    assert!(n <= 64);
    let edges: Vec<(usize, usize)> = (0..m)
        .map(|_| (it.next().unwrap(), it.next().unwrap()))
        .collect();
    let start = fs::read_to_string(&args[2]).expect("start file");
    let best_line = start
        .lines()
        .find(|l| l.starts_with("best "))
        .expect("best line");
    let part: Vec<usize> = best_line
        .split_whitespace()
        .skip(2)
        .map(|t| t.parse().unwrap())
        .collect();
    assert_eq!(part.len(), n);
    let target: usize = args[3].parse().unwrap();
    let seconds: f64 = args[4].parse().unwrap();
    let gamma: u64 = args[6].parse().unwrap();
    let mut rng = Rng(args[5]
        .parse::<u64>()
        .unwrap()
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        | 1);

    let mut adj = vec![0u64; n];
    let mut eid = vec![u32::MAX; n * n];
    for (i, &(u, v)) in edges.iter().enumerate() {
        adj[u] |= 1 << v;
        adj[v] |= 1 << u;
        eid[u * n + v] = i as u32;
        eid[v * n + u] = i as u32;
    }
    let mut quads = Vec::new();
    for a in 0..n {
        let mut pa = adj[a] & !((2u64 << a) - 1);
        while pa != 0 {
            let b = pa.trailing_zeros() as usize;
            pa &= pa - 1;
            let mut pb = pa & adj[b];
            while pb != 0 {
                let c = pb.trailing_zeros() as usize;
                pb &= pb - 1;
                let mut pc = pb & adj[c];
                while pc != 0 {
                    let d = pc.trailing_zeros() as usize;
                    pc &= pc - 1;
                    let e = |x: usize, y: usize| eid[x * n + y];
                    quads.push([e(a, b), e(a, c), e(a, d), e(b, c), e(b, d), e(c, d)]);
                }
            }
        }
    }
    let nq = quads.len();
    let mut inc = vec![Vec::new(); m];
    for (q, quad) in quads.iter().enumerate() {
        for &f in quad {
            inc[f as usize].push(q as u32);
        }
    }
    let mut s = Search {
        quads,
        inc,
        in_d: vec![false; m],
        cnt: vec![0; nq],
        w: vec![1; nq],
        score: vec![0; m],
        unhit: (0..nq as u32).collect(),
        pos: (0..nq as u32).collect(),
        d: Vec::new(),
        dpos: vec![u32::MAX; m],
        stamp: vec![0; m],
    };
    s.rescore();
    for (i, &(u, v)) in edges.iter().enumerate() {
        if part[u] == part[v] {
            s.add(i as u32, 0);
        }
    }
    println!("k4 {nq} start {} unhit {}", s.d.len(), s.unhit.len());
    assert!(
        s.unhit.is_empty(),
        "start partition must leave a K4-free graph"
    );

    let clock = Instant::now();
    let mut best = s.d.len();
    let mut step: u64 = 1;
    let mut min_unhit = usize::MAX;
    let mut total_w = nq as u64;
    loop {
        if s.unhit.is_empty() {
            if s.d.len() < best || step == 1 {
                best = s.d.len();
                let sec = clock.elapsed().as_secs_f64();
                println!("found {best} step {step} sec {sec:.1}");
                if best <= target {
                    let mut list: Vec<String> =
                        s.d.iter()
                            .map(|&e| {
                                let (u, v) = edges[e as usize];
                                format!("{u}-{v}")
                            })
                            .collect();
                    list.sort();
                    println!("transversal {best} {}", list.join(" "));
                }
            }
            if s.d.len() <= target {
                break;
            }
            let e = s.pick_out(&mut rng, step);
            s.remove(e, step);
            min_unhit = usize::MAX;
            step += 1;
            continue;
        }
        let out = s.pick_out(&mut rng, step);
        s.remove(out, step);
        let q = s.unhit[rng.below(s.unhit.len())];
        let mut pick = (0u64, 0u64, u32::MAX);
        for &f in &s.quads[q as usize] {
            if f == out {
                continue;
            }
            let key = (s.score[f as usize], u64::MAX - s.stamp[f as usize]);
            if pick.2 == u32::MAX || key > (pick.0, pick.1) {
                pick = (key.0, key.1, f);
            }
        }
        s.add(pick.2, step);
        for i in 0..s.unhit.len() {
            let q = s.unhit[i];
            s.w[q as usize] += 1;
            for &f in &s.quads[q as usize] {
                s.score[f as usize] += 1;
            }
        }
        min_unhit = min_unhit.min(s.unhit.len());
        total_w += s.unhit.len() as u64;
        if total_w > gamma * nq as u64 {
            s.w.iter_mut().for_each(|w| *w = (*w * 3 / 10).max(1));
            total_w = s.w.iter().sum();
            s.rescore();
        }
        if step.is_multiple_of(REPORT_EVERY) {
            let sec = clock.elapsed().as_secs_f64();
            let mean_w = total_w as f64 / nq as f64;
            let (size, unhit) = (s.d.len(), s.unhit.len());
            println!("step {step} sec {sec:.1} size {size} unhit {unhit} min_unhit {min_unhit} best {best} mean_w {mean_w:.2}");
            if sec > seconds {
                break;
            }
        }
        step += 1;
    }
    println!(
        "done best {best} steps {step} sec {:.1}",
        clock.elapsed().as_secs_f64()
    );
}
