//! Exact min(w3(G[S]), cap) for many vertex sets S of one graph G, where
//! w3 is the fewest edges inside the parts of a 3-partition (parts may be
//! empty). k4lp.py uses it for the right-hand side of its local rows.
//!
//! Depth-first search over part assignments with the bound
//! inside + sum over unplaced u of min_i (placed neighbours of u in part i):
//! the edges from u to placed vertices are disjoint across u, and wherever u
//! goes it keeps the ones in its own part. Parts open in index order, which
//! removes the 3! relabellings. The search starts with `cap` as the bound to
//! beat, so it proves "w3 >= cap" without finding a partition.
//!
//! Standalone: `rustc -O w3_batch.rs`. Input: "n m", m lines "u v", then "q",
//! then q lines "s cap v_1 ... v_s". Output: one line per set, "value nodes".

use std::env;
use std::fs;

struct Search {
    adj: Vec<Vec<usize>>,
    order: Vec<usize>,
    cnt: Vec<[u32; 3]>,
    best: u32,
    nodes: u64,
}

impl Search {
    fn bound(&self, from: usize) -> u32 {
        self.order[from..]
            .iter()
            .map(|&u| *self.cnt[u].iter().min().unwrap())
            .sum()
    }

    fn dfs(&mut self, depth: usize, inside: u32, opened: usize) {
        self.nodes += 1;
        if inside >= self.best {
            return;
        }
        if depth == self.order.len() {
            self.best = inside;
            return;
        }
        if inside + self.bound(depth) >= self.best {
            return;
        }
        let v = self.order[depth];
        let mut choices: Vec<(u32, usize)> = (0..(opened + 1).min(3))
            .map(|i| (self.cnt[v][i], i))
            .collect();
        choices.sort();
        for (add, i) in choices {
            for k in 0..self.adj[v].len() {
                let u = self.adj[v][k];
                self.cnt[u][i] += 1;
            }
            self.dfs(depth + 1, inside + add, opened.max(i + 1));
            for k in 0..self.adj[v].len() {
                let u = self.adj[v][k];
                self.cnt[u][i] -= 1;
            }
        }
    }
}

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() != 2 {
        eprintln!("usage: w3_batch FILE");
        std::process::exit(2);
    }
    let text = fs::read_to_string(&a[1]).expect("read input");
    let mut it = text.split_whitespace().map(|t| t.parse::<usize>().unwrap());
    let n = it.next().unwrap();
    let m = it.next().unwrap();
    let mut adj = vec![vec![false; n]; n];
    for _ in 0..m {
        let u = it.next().unwrap();
        let v = it.next().unwrap();
        adj[u][v] = true;
        adj[v][u] = true;
    }
    let q = it.next().unwrap();
    for _ in 0..q {
        let s = it.next().unwrap();
        let cap = it.next().unwrap() as u32;
        let vs: Vec<usize> = (0..s).map(|_| it.next().unwrap()).collect();
        // Local indices; place high-degree vertices first.
        let mut local: Vec<Vec<usize>> = vec![Vec::new(); s];
        for i in 0..s {
            for j in 0..s {
                if i != j && adj[vs[i]][vs[j]] {
                    local[i].push(j);
                }
            }
        }
        let mut order: Vec<usize> = (0..s).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(local[i].len()));
        let mut search = Search {
            adj: local,
            order,
            cnt: vec![[0; 3]; s],
            best: cap,
            nodes: 0,
        };
        search.dfs(0, 0, 0);
        println!("{} {}", search.best, search.nodes);
    }
}
