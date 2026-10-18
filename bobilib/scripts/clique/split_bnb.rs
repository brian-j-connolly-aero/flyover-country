//! Exact depth-first search: can a graph H be split into three labelled parts
//! of sizes s1, s2, s3 cutting at most B of its edges?
//!
//! This is the third, solver-free check of the density-0.9 lower bound: H is
//! the complement of G, and a 3-partition of G with at most k inside edges
//! exists only if H splits with sizes s cutting at most
//! b(s) = k + e(H) - sum C(s_i, 2) edges (see lb.py).
//!
//! Pruning: every unplaced vertex u will add at least
//! deg_placed(u) - max_i placed_nbrs(u, i) cut edges (maximum over parts with
//! room left), and those edge sets are disjoint across u. Parts of equal size
//! are interchangeable, so an empty part may only be opened after the equal
//! part before it. Vertices are placed in an order that keeps each new
//! vertex's placed neighbourhood large.
//!
//! Standalone: `rustc -O split_bnb.rs`. Input file "n m" then m lines "u v".
//! usage: split_bnb FILE S1 S2 S3 B   -> prints FOUND <parts> or NONE, and the node count

use std::env;
use std::fs;

struct Search {
    n: usize,
    adj: Vec<Vec<usize>>,
    order: Vec<usize>,
    cap: [usize; 3],
    size: [usize; 3],
    part: Vec<usize>,
    /// cnt[v][i]: placed neighbours of v in part i.
    cnt: Vec<[usize; 3]>,
    placed_deg: Vec<usize>,
    budget: usize,
    nodes: u64,
}

impl Search {
    fn bound(&self, from: usize, cut: usize) -> usize {
        let mut lb = cut;
        for &u in &self.order[from..] {
            let best = (0..3)
                .filter(|&i| self.size[i] < self.cap[i])
                .map(|i| self.cnt[u][i])
                .max()
                .unwrap_or(0);
            lb += self.placed_deg[u] - best;
            if lb > self.budget {
                return lb;
            }
        }
        lb
    }

    fn place(&mut self, v: usize, i: usize) {
        self.part[v] = i;
        self.size[i] += 1;
        for k in 0..self.adj[v].len() {
            let u = self.adj[v][k];
            self.cnt[u][i] += 1;
            self.placed_deg[u] += 1;
        }
    }

    fn unplace(&mut self, v: usize, i: usize) {
        self.part[v] = usize::MAX;
        self.size[i] -= 1;
        for k in 0..self.adj[v].len() {
            let u = self.adj[v][k];
            self.cnt[u][i] -= 1;
            self.placed_deg[u] -= 1;
        }
    }

    fn dfs(&mut self, depth: usize, cut: usize) -> bool {
        self.nodes += 1;
        if depth == self.n {
            return true;
        }
        let v = self.order[depth];
        let mut choices: Vec<(usize, usize)> = Vec::with_capacity(3);
        for i in 0..3 {
            if self.size[i] == self.cap[i] {
                continue;
            }
            // Equal sizes: part i may open only after the earlier part of that size.
            if self.size[i] == 0 && (0..i).any(|j| self.cap[j] == self.cap[i] && self.size[j] == 0)
            {
                continue;
            }
            let add = self.placed_deg[v] - self.cnt[v][i];
            if cut + add <= self.budget {
                choices.push((add, i));
            }
        }
        choices.sort();
        for (add, i) in choices {
            self.place(v, i);
            if self.bound(depth + 1, cut + add) <= self.budget && self.dfs(depth + 1, cut + add) {
                return true;
            }
            self.unplace(v, i);
        }
        false
    }
}

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() != 6 {
        eprintln!("usage: split_bnb FILE S1 S2 S3 B");
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
    let cap = [
        a[2].parse().unwrap(),
        a[3].parse().unwrap(),
        a[4].parse().unwrap(),
    ];
    assert_eq!(cap[0] + cap[1] + cap[2], n, "sizes must add up to n");
    let budget: usize = a[5].parse().unwrap();
    // Order: repeatedly take the vertex with most already-ordered neighbours
    // (ties: higher degree, then lower index).
    let mut order = Vec::with_capacity(n);
    let mut inside = vec![false; n];
    let mut links = vec![0usize; n];
    for _ in 0..n {
        let v = (0..n)
            .filter(|&v| !inside[v])
            .max_by_key(|&v| (links[v], adj[v].len(), usize::MAX - v))
            .unwrap();
        inside[v] = true;
        order.push(v);
        for &u in &adj[v] {
            links[u] += 1;
        }
    }
    let mut s = Search {
        n,
        adj,
        order,
        cap,
        size: [0; 3],
        part: vec![usize::MAX; n],
        cnt: vec![[0; 3]; n],
        placed_deg: vec![0; n],
        budget,
        nodes: 0,
    };
    if s.dfs(0, 0) {
        let p: Vec<String> = s.part.iter().map(|x| x.to_string()).collect();
        println!("FOUND {}", p.join(" "));
    } else {
        println!("NONE");
    }
    println!("nodes {}", s.nodes);
}
