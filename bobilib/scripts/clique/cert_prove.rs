//! Prover for the clique partition certificates (results/clique/certificates/).
//!
//! Question: can graph H (the complement of an instance graph G) be split into
//! three labelled parts of sizes s1 >= s2 >= s3 cutting at most B edges of H?
//! When the answer is no, this program prints a branch-and-bound tree that
//! proves it, in the format that scripts/check_clique_certificates.py checks:
//!
//! * a fixed vertex order v_0, ..., v_{n-1};
//! * the tree in preorder, one character per node: '1' = branch, '0' = leaf.
//!   A node at depth d is a placement of v_0..v_{d-1}. A branch node places
//!   v_d into every part p = 0, 1, 2 (in this order) that is *allowed*: the
//!   part has room (size < s_p), and, if it is empty and s_{p-1} = s_p, part
//!   p-1 is not empty (equal parts are opened in index order). A leaf is
//!   closed by the bound `cut(placed) + sum over unplaced u of min over
//!   parts p with room of (placed H-neighbours of u outside p) > B`.
//!
//! The search is complete (it never stops early), so the tree is the full
//! search tree of this bound and branching rule. Among `tries` vertex orders
//! (the first greedy, the others greedy with random tie-breaking) it keeps the
//! one with the smallest tree.
//!
//! Standalone: `rustc -O --edition 2021 cert_prove.rs`.
//! Input file: "n m" then m lines "u v" (0-based edges of H).
//! usage: cert_prove FILE S1 S2 S3 B TRIES SEED
//! output: "FOUND <part of v for v = 0..n-1>"  (a split exists; no certificate)
//!     or: "ORDER <v_0 ... v_{n-1}>", "NODES <nodes> <leaves>", "TREE <string>"

use std::env;
use std::fs;

struct Search {
    n: usize,
    adj: Vec<Vec<usize>>,
    order: Vec<usize>,
    cap: [usize; 3],
    size: [usize; 3],
    part: Vec<usize>,
    /// cnt[v][p]: placed H-neighbours of v in part p.
    cnt: Vec<[usize; 3]>,
    placed_deg: Vec<usize>,
    budget: usize,
    tree: Vec<u8>,
    leaves: u64,
    found: bool,
    /// Stop when the tree grows beyond this many nodes (used while comparing orders).
    limit: usize,
}

impl Search {
    fn bound(&self, depth: usize, cut: usize) -> usize {
        let mut lb = cut;
        for &u in &self.order[depth..] {
            let best = (0..3)
                .filter(|&p| self.size[p] < self.cap[p])
                .map(|p| self.placed_deg[u] - self.cnt[u][p])
                .min()
                .expect("some part has room");
            lb += best;
        }
        lb
    }

    fn allowed(&self, p: usize) -> bool {
        self.size[p] < self.cap[p]
            && !(self.size[p] == 0 && p > 0 && self.cap[p - 1] == self.cap[p] && self.size[p - 1] == 0)
    }

    fn set(&mut self, v: usize, p: usize, add: bool) {
        if add {
            self.part[v] = p;
            self.size[p] += 1;
        } else {
            self.part[v] = usize::MAX;
            self.size[p] -= 1;
        }
        for k in 0..self.adj[v].len() {
            let u = self.adj[v][k];
            if add {
                self.cnt[u][p] += 1;
                self.placed_deg[u] += 1;
            } else {
                self.cnt[u][p] -= 1;
                self.placed_deg[u] -= 1;
            }
        }
    }

    /// Full search below the current node. Returns false if the tree limit was hit.
    fn dfs(&mut self, depth: usize, cut: usize) -> bool {
        if self.tree.len() >= self.limit {
            return false;
        }
        if self.bound(depth, cut) > self.budget {
            self.tree.push(b'0');
            self.leaves += 1;
            return true;
        }
        if depth == self.n {
            self.found = true;
            return false;
        }
        self.tree.push(b'1');
        let v = self.order[depth];
        for p in 0..3 {
            if !self.allowed(p) {
                continue;
            }
            let add = self.placed_deg[v] - self.cnt[v][p];
            self.set(v, p, true);
            let ok = self.dfs(depth + 1, cut + add);
            if !ok {
                return false; // keep the placement in `part` when a split was found
            }
            self.set(v, p, false);
        }
        true
    }
}

fn greedy_order(n: usize, adj: &[Vec<usize>], rng: &mut u64, random_ties: bool) -> Vec<usize> {
    // Repeatedly take the unordered vertex with the most ordered neighbours;
    // ties by higher degree, then lower index (or a random key).
    let key: Vec<u64> = (0..n)
        .map(|v| if random_ties { next(rng) } else { (n - v) as u64 })
        .collect();
    let mut order = Vec::with_capacity(n);
    let mut inside = vec![false; n];
    let mut links = vec![0usize; n];
    for _ in 0..n {
        let v = (0..n)
            .filter(|&v| !inside[v])
            .max_by_key(|&v| (links[v], adj[v].len(), key[v]))
            .unwrap();
        inside[v] = true;
        order.push(v);
        for &u in &adj[v] {
            links[u] += 1;
        }
    }
    order
}

fn next(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() != 8 {
        eprintln!("usage: cert_prove FILE S1 S2 S3 B TRIES SEED");
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
        assert!(u != v && u < n && v < n);
        adj[u].push(v);
        adj[v].push(u);
    }
    let cap: [usize; 3] = [a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap()];
    assert_eq!(cap[0] + cap[1] + cap[2], n, "sizes must add up to n");
    assert!(cap[0] >= cap[1] && cap[1] >= cap[2], "sizes must be non-increasing");
    let budget: usize = a[5].parse().unwrap();
    let tries: usize = a[6].parse().unwrap();
    let mut rng: u64 = a[7].parse::<u64>().unwrap() * 2654435761 + 88172645463325252;
    let mut best: Option<(Vec<usize>, Vec<u8>, u64)> = None;
    for t in 0..tries.max(1) {
        let order = greedy_order(n, &adj, &mut rng, t > 0);
        let limit = best.as_ref().map_or(usize::MAX, |b| b.1.len());
        let mut s = Search {
            n,
            adj: adj.clone(),
            order: order.clone(),
            cap,
            size: [0; 3],
            part: vec![usize::MAX; n],
            cnt: vec![[0; 3]; n],
            placed_deg: vec![0; n],
            budget,
            tree: Vec::new(),
            leaves: 0,
            found: false,
            limit,
        };
        let complete = s.dfs(0, 0);
        if s.found {
            let p: Vec<String> = s.part.iter().map(|x| x.to_string()).collect();
            println!("FOUND {}", p.join(" "));
            return;
        }
        if complete {
            best = Some((order, s.tree, s.leaves));
        }
    }
    let (order, tree, leaves) = best.expect("the first order always completes");
    let o: Vec<String> = order.iter().map(|x| x.to_string()).collect();
    println!("ORDER {}", o.join(" "));
    println!("NODES {} {}", tree.len(), leaves);
    println!("TREE {}", String::from_utf8(tree).unwrap());
}
