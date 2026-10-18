//! Attack-protect games on graphs: BOBILib's `inter-fire` set (Baggio,
//! Carvalho, Lodi and Tramontani 2021, the lower two levels of their
//! multilevel critical node problem, with the vaccinated nodes already
//! removed from the graph).
//!
//! The model, as read from the MPS/AUX pair (every variable binary; node
//! labels are the name suffixes, `y_v`, `a_v`, `x_v`):
//!
//! ```text
//! leader (attacker):   min  sum_v w_v a_v
//!                      s.t. sum_v y_v <= Phi                (row Phi)
//! follower (defender): max  sum_v w_v a_v                   (AUX: a_v has -w_v)
//!                      s.t. sum_v x_v <= Lambda             (row Lambda)
//!                           a_v + y_v <= 1         each v   (rows V<v>)
//!                           a_v - a_u - x_v <= 0   each arc (rows A<u>-<v>)
//! ```
//!
//! `y_v = 1` attacks (infects) v, `x_v = 1` protects v and `a_v = 1` marks
//! v saved: an attacked node is lost, and an unprotected node is saved only
//! if every neighbour is. Both arcs of every edge are present. The leader's
//! objective is the negative of the follower's, so every follower optimum
//! gives the leader the same value and the optimistic tie-break is moot.
//!
//! [`Fire::from_model`] checks every row, bound and objective coefficient
//! against this pattern and refuses anything else. When the graph is a
//! forest, the follower's MILP is solved exactly by dynamic programming over
//! the follower's own variables (per node, the four values of (x_v, a_v);
//! the arc rows only couple adjacent nodes and the budget row is a knapsack
//! dimension). The leader's feasible set is every subset of at most Phi
//! nodes, which [`Fire::solve`] enumerates in full.
//!
//! The result is a certificate that does not rest on the dynamic program
//! being optimal: for every attack the program's value is backed by an
//! explicit follower point that passes [`Bilevel::max_violation`] on the
//! raw model (so the follower's optimum there is at least that value), and
//! at the minimising attack the follower is re-solved by HiGHS. The minimum
//! over attacks is then a lower bound on every attack's follower optimum and
//! is attained.

use crate::general::check_point;
use crate::model::Bilevel;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};

/// The largest protection budget the fixed-size tables support.
pub const MAX_LAMBDA: usize = 15;
const W: usize = MAX_LAMBDA + 1;
const NEG: i64 = i64::MIN / 4;
/// Follower state of a node: s = 2 x + a.
const A_OF: [i64; 4] = [0, 1, 0, 1];
const X_OF: [usize; 4] = [0, 0, 1, 1];

/// Both arc rows of edge {p, c}: a_c <= a_p + x_c and a_p <= a_c + x_p.
const fn edge_ok(sp: usize, sc: usize) -> bool {
    let (ap, xp) = (A_OF[sp], X_OF[sp] as i64);
    let (ac, xc) = (A_OF[sc], X_OF[sc] as i64);
    ac <= ap + xc && ap <= ac + xp
}

const COMPAT: [[bool; 4]; 4] = {
    let mut t = [[false; 4]; 4];
    let mut sp = 0;
    while sp < 4 {
        let mut sc = 0;
        while sc < 4 {
            t[sp][sc] = edge_ok(sp, sc);
            sc += 1;
        }
        sp += 1;
    }
    t
};

/// An attack-protect instance in node form.
#[derive(Clone, Debug)]
pub struct Fire {
    pub name: String,
    /// Node labels (the variable name suffixes), in model order of `y_`.
    pub labels: Vec<String>,
    /// Model column of y_v (leader), a_v and x_v (follower) per node.
    pub y: Vec<usize>,
    pub a: Vec<usize>,
    pub x: Vec<usize>,
    /// Objective weight of a_v (leader obj; the follower's is its negative).
    pub w: Vec<i64>,
    pub phi: usize,
    pub lambda: usize,
    /// Undirected edges (both arc rows present), u < v.
    pub edges: Vec<(usize, usize)>,
    pub nvars: usize,
}

fn binary(m: &Bilevel, j: usize) -> bool {
    let v = &m.vars[j];
    v.integer && v.lb == 0.0 && v.ub == 1.0
}

fn int_of(x: f64, what: &str) -> Result<i64, String> {
    if x.is_finite() && x == x.round() {
        Ok(x as i64)
    } else {
        Err(format!("{what}: {x} is not an integer"))
    }
}

impl Fire {
    /// Recognise the attack-protect pattern; every row, bound and objective
    /// coefficient is checked, and anything else is an error.
    pub fn from_model(m: &Bilevel) -> Result<Fire, String> {
        // Pair variables by suffix.
        let mut by_label: BTreeMap<String, [Option<usize>; 3]> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        for (j, v) in m.vars.iter().enumerate() {
            let (kind, label) = match v.name.split_once('_') {
                Some(("y", l)) => (0, l),
                Some(("a", l)) => (1, l),
                Some(("x", l)) => (2, l),
                _ => return Err(format!("unexpected variable {}", v.name)),
            };
            let e = by_label.entry(label.to_string()).or_insert_with(|| {
                order.push(label.to_string());
                [None; 3]
            });
            if e[kind].replace(j).is_some() {
                return Err(format!("duplicate variable {}", v.name));
            }
            if !binary(m, j) {
                return Err(format!("{} is not binary", v.name));
            }
            let want_lower = kind != 0;
            if v.lower != want_lower {
                return Err(format!("{} is on the wrong level", v.name));
            }
        }
        let n = order.len();
        let (mut y, mut a, mut x, mut w) = (vec![0; n], vec![0; n], vec![0; n], vec![0; n]);
        let mut node_of = vec![usize::MAX; m.vars.len()];
        let mut kind_of = vec![usize::MAX; m.vars.len()];
        for (i, l) in order.iter().enumerate() {
            let e = by_label[l];
            let [Some(jy), Some(ja), Some(jx)] = e else {
                return Err(format!("node {l} lacks one of y_, a_, x_"));
            };
            let (vy, va, vx) = (&m.vars[jy], &m.vars[ja], &m.vars[jx]);
            if vy.obj != 0.0 || vy.ll_obj != 0.0 || vx.obj != 0.0 || vx.ll_obj != 0.0 {
                return Err(format!("y_{l} or x_{l} has an objective coefficient"));
            }
            w[i] = int_of(va.obj, &format!("objective of a_{l}"))?;
            if va.ll_obj != -va.obj || w[i] < 0 {
                return Err(format!(
                    "a_{l}: follower coefficient {} is not minus the leader's {} (or negative weight)",
                    va.ll_obj, va.obj
                ));
            }
            (y[i], a[i], x[i]) = (jy, ja, jx);
            for (k, j) in [jy, ja, jx].into_iter().enumerate() {
                node_of[j] = i;
                kind_of[j] = k;
            }
        }
        if m.obj_offset != 0.0 {
            return Err("objective offset".into());
        }

        let (mut phi, mut lambda) = (None, None);
        let mut node_row = vec![false; n];
        let mut arcs: HashMap<(usize, usize), usize> = HashMap::new();
        for r in &m.rows {
            if r.lo != f64::NEG_INFINITY {
                return Err(format!("row {} has a lower bound", r.name));
            }
            let hi = int_of(r.hi, &format!("right-hand side of {}", r.name))?;
            let mut coefs: Vec<(usize, usize, f64)> = r
                .coefs
                .iter()
                .map(|&(j, c)| (kind_of[j], node_of[j], c))
                .collect();
            coefs.sort_by_key(|p| (p.0, p.1));
            let all_one = |k: usize| {
                coefs.len() == n
                    && coefs
                        .iter()
                        .enumerate()
                        .all(|(i, &(kk, node, c))| kk == k && node == i && c == 1.0)
            };
            if !r.lower {
                // The only leader row: the attack budget.
                if !all_one(0) || phi.is_some() || hi < 0 {
                    return Err(format!("unexpected leader row {}", r.name));
                }
                phi = Some(hi as usize);
                continue;
            }
            if all_one(2) {
                if lambda.is_some() || hi < 0 {
                    return Err(format!("second protection budget row {}", r.name));
                }
                lambda = Some(hi as usize);
                continue;
            }
            match coefs.as_slice() {
                // a_v + y_v <= 1
                [(0, v1, c1), (1, v2, c2)] if v1 == v2 && *c1 == 1.0 && *c2 == 1.0 && hi == 1 => {
                    if std::mem::replace(&mut node_row[*v1], true) {
                        return Err(format!("second node row for {}", order[*v1]));
                    }
                }
                // a_v - a_u - x_v <= 0 (both a_ entries sort before x_)
                [(1, p, cp), (1, q, cq), (2, xv, cx)] if hi == 0 && *cx == -1.0 && p != q => {
                    let (u, v) = match (*cp, *cq) {
                        (c, d) if c == 1.0 && d == -1.0 => (*q, *p),
                        (c, d) if c == -1.0 && d == 1.0 => (*p, *q),
                        _ => return Err(format!("unexpected arc row {}", r.name)),
                    };
                    if *xv != v {
                        return Err(format!("arc row {} protects the wrong node", r.name));
                    }
                    *arcs.entry((u, v)).or_insert(0) += 1;
                }
                _ => return Err(format!("unrecognised follower row {}", r.name)),
            }
        }
        if let Some(v) = node_row.iter().position(|&b| !b) {
            return Err(format!("node {} has no row a + y <= 1", order[v]));
        }
        let mut edges = Vec::new();
        for &(u, v) in arcs.keys() {
            if !arcs.contains_key(&(v, u)) {
                return Err(format!("arc {}-{} without its reverse", order[u], order[v]));
            }
            if u < v {
                edges.push((u, v));
            }
        }
        edges.sort();
        let lambda = lambda.ok_or("no protection budget row")?;
        if lambda > MAX_LAMBDA {
            return Err(format!("protection budget {lambda} above {MAX_LAMBDA}"));
        }
        Ok(Fire {
            name: m.name.clone(),
            labels: order,
            y,
            a,
            x,
            w,
            phi: phi.ok_or("no attack budget row")?,
            lambda,
            edges,
            nvars: m.vars.len(),
        })
    }

    pub fn n(&self) -> usize {
        self.labels.len()
    }

    /// True when the graph has no cycle.
    pub fn is_forest(&self) -> bool {
        let mut parent: Vec<usize> = (0..self.n()).collect();
        fn find(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        for &(u, v) in &self.edges {
            let (ru, rv) = (find(&mut parent, u), find(&mut parent, v));
            if ru == rv {
                return false;
            }
            parent[ru] = rv;
        }
        true
    }

    /// Model values for an attack and follower states (s = 2 x + a).
    pub fn values(&self, attack: &[usize], states: &[u8]) -> Vec<f64> {
        let mut val = vec![0.0; self.nvars];
        for &v in attack {
            val[self.y[v]] = 1.0;
        }
        for (v, &s) in states.iter().enumerate() {
            val[self.a[v]] = A_OF[s as usize] as f64;
            val[self.x[v]] = X_OF[s as usize] as f64;
        }
        val
    }

    /// The attack in a model point (nodes with y_v = 1).
    pub fn attack_of(&self, val: &[f64]) -> Vec<usize> {
        (0..self.n()).filter(|&v| val[self.y[v]] > 0.5).collect()
    }
}

/// The follower's problem on a forest, rooted at a virtual node `n` whose
/// edges to the components' roots carry no constraint.
#[derive(Clone, Debug)]
pub struct TreeDp {
    n: usize,
    lambda: usize,
    w: Vec<i64>,
    /// Post-order (children before parents); the virtual root is last.
    order: Vec<usize>,
    children: Vec<Vec<usize>>,
    /// Offset (in tables) of node v's first table; it has children+1 of them.
    off: Vec<usize>,
    ntables: usize,
}

/// Scratch space for one thread.
#[derive(Clone, Debug)]
pub struct DpScratch {
    tab: Vec<i64>,
    /// Allowed follower states per node (bit s), set from the attack.
    pub mask: Vec<u8>,
    pub states: Vec<u8>,
}

impl TreeDp {
    pub fn new(f: &Fire) -> Option<TreeDp> {
        if !f.is_forest() {
            return None;
        }
        let n = f.n();
        let mut adj = vec![Vec::new(); n];
        for &(u, v) in &f.edges {
            adj[u].push(v);
            adj[v].push(u);
        }
        let mut children = vec![Vec::new(); n + 1];
        let mut seen = vec![false; n];
        let mut bfs = vec![n];
        for r in 0..n {
            if seen[r] {
                continue;
            }
            seen[r] = true;
            children[n].push(r);
            let mut queue = vec![r];
            while let Some(u) = queue.pop() {
                bfs.push(u);
                for &v in &adj[u] {
                    if !seen[v] {
                        seen[v] = true;
                        children[u].push(v);
                        queue.push(v);
                    }
                }
            }
        }
        // `bfs` lists every node after its parent; reversed, it is a post-order.
        let order: Vec<usize> = bfs.into_iter().rev().collect();
        let mut off = vec![0; n + 1];
        let mut t = 0;
        for v in 0..=n {
            off[v] = t;
            t += children[v].len() + 1;
        }
        let mut w = f.w.clone();
        w.push(0);
        Some(TreeDp {
            n,
            lambda: f.lambda,
            w,
            order,
            children,
            off,
            ntables: t,
        })
    }

    pub fn scratch(&self) -> DpScratch {
        let mut mask = vec![0b1111u8; self.n + 1];
        mask[self.n] = 0b0001; // the virtual root: x = a = 0
        DpScratch {
            tab: vec![NEG; self.ntables * 4 * W],
            mask,
            states: vec![0; self.n],
        }
    }

    #[inline]
    fn final_table(&self, v: usize) -> usize {
        (self.off[v] + self.children[v].len()) * 4 * W
    }

    /// The follower's optimum (max sum w_v a_v) for the allowed states in
    /// `sc.mask`, or None if no follower point exists.
    pub fn solve(&self, sc: &mut DpScratch) -> Option<i64> {
        let l = self.lambda;
        let tab = &mut sc.tab;
        for &v in &self.order {
            let base = self.off[v] * 4 * W;
            for s in 0..4 {
                let row = base + s * W;
                tab[row..row + W].fill(NEG);
                if (sc.mask[v] >> s) & 1 == 1 && X_OF[s] <= l {
                    tab[row + X_OF[s]] = A_OF[s] * self.w[v];
                }
            }
            let free = v == self.n;
            for (i, &c) in self.children[v].iter().enumerate() {
                let cf = self.final_table(c);
                let prev = base + i * 4 * W;
                let next = prev + 4 * W;
                for sp in 0..4 {
                    let mut g = [NEG; W];
                    for (sc_, &ok) in COMPAT[sp].iter().enumerate() {
                        if free || ok {
                            for k in 0..=l {
                                g[k] = g[k].max(tab[cf + sc_ * W + k]);
                            }
                        }
                    }
                    for k in 0..=l {
                        let mut best = NEG;
                        for k2 in 0..=k {
                            let p = tab[prev + sp * W + k - k2];
                            if p > NEG && g[k2] > NEG {
                                best = best.max(p + g[k2]);
                            }
                        }
                        tab[next + sp * W + k] = best;
                    }
                }
            }
        }
        let root = self.final_table(self.n);
        let best = (0..=l).map(|k| tab[root + k]).max().unwrap();
        (best > NEG).then_some(best)
    }

    /// After [`TreeDp::solve`] returned `Some`, recover an optimal follower
    /// point into `sc.states` (s = 2 x_v + a_v per node).
    pub fn trace(&self, sc: &mut DpScratch) {
        let l = self.lambda;
        let tab = &sc.tab;
        let root = self.final_table(self.n);
        let k0 = (0..=l)
            .max_by_key(|&k| (tab[root + k], usize::MAX - k))
            .unwrap();
        let mut stack = vec![(self.n, 0usize, k0)];
        while let Some((v, s, mut k)) = stack.pop() {
            if v < self.n {
                sc.states[v] = s as u8;
            }
            let base = self.off[v] * 4 * W;
            let free = v == self.n;
            for (i, &c) in self.children[v].iter().enumerate().rev() {
                let cf = self.final_table(c);
                let prev = base + i * 4 * W;
                let target = tab[prev + 4 * W + s * W + k];
                let mut found = None;
                'search: for k2 in 0..=k {
                    let p = tab[prev + s * W + k - k2];
                    if p <= NEG {
                        continue;
                    }
                    for (s2, &ok) in COMPAT[s].iter().enumerate() {
                        let q = tab[cf + s2 * W + k2];
                        if (free || ok) && q > NEG && p + q == target {
                            found = Some((s2, k2));
                            break 'search;
                        }
                    }
                }
                let (s2, k2) = found.expect("trace: no split reproduces the table");
                stack.push((c, s2, k2));
                k -= k2;
            }
            assert_eq!(k, X_OF[s], "trace: budget left over at node {v}");
        }
    }
}

/// Saved weight for an attack and protection set, by the fixpoint of the
/// arc rows (an independent evaluation used by the tests and the sampler).
pub fn saved_by_fixpoint(f: &Fire, attacked: &[bool], protected: &[bool]) -> i64 {
    let n = f.n();
    let mut a: Vec<bool> = (0..n).map(|v| !attacked[v]).collect();
    loop {
        let mut changed = false;
        for &(u, v) in &f.edges {
            for (p, q) in [(u, v), (v, u)] {
                // a_q <= a_p + x_q
                if a[q] && !a[p] && !protected[q] {
                    a[q] = false;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    (0..n).filter(|&v| a[v]).map(|v| f.w[v]).sum()
}

fn binom(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let mut r: u128 = 1;
    for i in 0..k {
        r = r * (n - i) as u128 / (i + 1) as u128;
    }
    r as u64
}

/// Per attack size: the least follower optimum, how many attacks reach it,
/// the lexicographically first of them, and a histogram of values.
#[derive(Clone, Debug, Default)]
pub struct SizeStats {
    pub attacks: u64,
    pub min: Option<i64>,
    pub count_min: u64,
    pub argmin: Vec<usize>,
    pub hist: BTreeMap<i64, u64>,
    /// Follower points that failed the check against the raw model.
    pub witness_failures: u64,
    /// Attacks without a follower optimum (none found, or the solver failed).
    pub unsolved: u64,
}

impl SizeStats {
    fn add(&mut self, val: Option<i64>, attack: &[usize]) {
        self.attacks += 1;
        let Some(v) = val else {
            self.unsolved += 1;
            return;
        };
        *self.hist.entry(v).or_insert(0) += 1;
        match self.min {
            Some(m) if v > m => {}
            Some(m) if v == m => {
                self.count_min += 1;
                if attack < self.argmin.as_slice() {
                    self.argmin = attack.to_vec();
                }
            }
            _ => {
                self.min = Some(v);
                self.count_min = 1;
                self.argmin = attack.to_vec();
            }
        }
    }

    fn merge(mut self, o: SizeStats) -> SizeStats {
        self.attacks += o.attacks;
        self.witness_failures += o.witness_failures;
        self.unsolved += o.unsolved;
        for (k, c) in o.hist {
            *self.hist.entry(k).or_insert(0) += c;
        }
        match (self.min, o.min) {
            (_, None) => {}
            (None, Some(_)) => {
                self.min = o.min;
                self.count_min = o.count_min;
                self.argmin = o.argmin;
            }
            (Some(a), Some(b)) if b < a => {
                self.min = o.min;
                self.count_min = o.count_min;
                self.argmin = o.argmin;
            }
            (Some(a), Some(b)) if a == b => {
                self.count_min += o.count_min;
                if o.argmin < self.argmin {
                    self.argmin = o.argmin;
                }
            }
            _ => {}
        }
        self
    }
}

#[derive(Clone, Debug)]
pub struct Enumeration {
    /// Index k: attacks with exactly k nodes.
    pub by_size: Vec<SizeStats>,
    /// Each follower value backed by a point checked against the raw model.
    pub witnessed: bool,
    /// How the follower was solved: "tree-dp" or "highs".
    pub engine: &'static str,
}

impl Enumeration {
    pub fn attacks(&self) -> u64 {
        self.by_size.iter().map(|s| s.attacks).sum()
    }
    pub fn min(&self) -> Option<i64> {
        self.by_size.iter().filter_map(|s| s.min).min()
    }
    /// The lexicographically first minimising attack of the smallest size.
    pub fn argmin(&self) -> Option<Vec<usize>> {
        let m = self.min()?;
        self.by_size
            .iter()
            .find(|s| s.min == Some(m))
            .map(|s| s.argmin.clone())
    }
    pub fn count_min(&self) -> u64 {
        let m = self.min();
        self.by_size
            .iter()
            .filter(|s| s.min.is_some() && s.min == m)
            .map(|s| s.count_min)
            .sum()
    }
    pub fn witness_failures(&self) -> u64 {
        self.by_size.iter().map(|s| s.witness_failures).sum()
    }
    pub fn unsolved(&self) -> u64 {
        self.by_size.iter().map(|s| s.unsolved).sum()
    }
}

/// Visit every k-subset of `lo..n` in lexicographic order, prefixed by `head`.
fn for_each_combo(head: &[usize], lo: usize, n: usize, k: usize, mut f: impl FnMut(&[usize])) {
    if lo + k > n {
        return;
    }
    let h = head.len();
    let mut c: Vec<usize> = head.to_vec();
    c.extend(lo..lo + k);
    loop {
        f(&c);
        // The rightmost tail position below its largest value n - k + i.
        let Some(i) = (0..k).rev().find(|&i| c[h + i] < n - k + i) else {
            return;
        };
        c[h + i] += 1;
        for j in i + 1..k {
            c[h + j] = c[h + j - 1] + 1;
        }
    }
}

/// Run `eval` (the follower's optimum, or None) on every leader-feasible
/// attack: each subset of at most Phi nodes, which is the whole leader
/// region (binary y and the single row sum y <= Phi). Tasks are (size,
/// first node) pairs spread over the current rayon pool; the counts per
/// size are checked against the binomial coefficients.
fn over_attacks<T, I, E>(f: &Fire, init: I, eval: E) -> Vec<SizeStats>
where
    I: Fn() -> T + Sync + Send,
    E: Fn(&mut T, &[usize], &mut SizeStats) -> Option<i64> + Sync + Send,
{
    let n = f.n();
    let kmax = f.phi.min(n);
    let mut tasks: Vec<(usize, Option<usize>)> = vec![(0, None)];
    for k in 1..=kmax {
        for i0 in 0..n {
            tasks.push((k, Some(i0)));
        }
    }
    let parts: Vec<(usize, SizeStats)> = tasks
        .par_iter()
        .map_init(init, |t, &(k, first)| {
            let mut st = SizeStats::default();
            let mut visit = |attack: &[usize]| {
                let r = eval(t, attack, &mut st);
                st.add(r, attack);
            };
            match first {
                None => visit(&[]),
                Some(i0) => for_each_combo(&[i0], i0 + 1, n, k - 1, &mut visit),
            }
            (k, st)
        })
        .collect();
    let mut by_size = vec![SizeStats::default(); kmax + 1];
    for (k, st) in parts {
        by_size[k] = std::mem::take(&mut by_size[k]).merge(st);
    }
    for (k, s) in by_size.iter().enumerate() {
        assert_eq!(
            s.attacks,
            binom(n, k),
            "enumeration missed attacks of size {k}"
        );
    }
    by_size
}

/// Every leader-feasible attack, the follower solved exactly on the forest
/// by [`TreeDp`]. With `witness`, each attack's DP follower point is checked
/// against the raw model (every row, bound and integrality by
/// [`Bilevel::max_violation`], and its objective by [`Bilevel::upper_obj`]).
pub fn enumerate(m: &Bilevel, f: &Fire, dp: &TreeDp, witness: bool) -> Enumeration {
    let n = f.n();
    let by_size = over_attacks(
        f,
        || (dp.scratch(), vec![0.0f64; f.nvars]),
        |(sc, val), attack, st| {
            for &v in attack {
                sc.mask[v] = 0b0101; // a_v + y_v <= 1 forces a_v = 0
            }
            let r = dp.solve(sc);
            if let (true, Some(value)) = (witness, r) {
                dp.trace(sc);
                val.fill(0.0);
                for &v in attack {
                    val[f.y[v]] = 1.0;
                }
                for v in 0..n {
                    let s = sc.states[v] as usize;
                    val[f.a[v]] = A_OF[s] as f64;
                    val[f.x[v]] = X_OF[s] as f64;
                }
                let (viol, _) = m.max_violation(&val[..]);
                if viol > 0.0 || m.upper_obj(&val[..]) != value as f64 {
                    st.witness_failures += 1;
                }
            }
            for &v in attack {
                sc.mask[v] = 0b1111;
            }
            r
        },
    );
    Enumeration {
        by_size,
        witnessed: witness,
        engine: "tree-dp",
    }
}

/// Every leader-feasible attack, the follower's MILP solved by HiGHS (any
/// graph; for those with cycles, where [`TreeDp`] does not apply).
pub fn enumerate_highs(m: &Bilevel, f: &Fire, time: f64) -> Enumeration {
    let by_size = over_attacks(
        f,
        || (),
        |_, attack, _| match follower_highs(m, f, attack, time) {
            Some(h) if (h - h.round()).abs() <= 1e-6 => Some(h.round() as i64),
            _ => None,
        },
    );
    Enumeration {
        by_size,
        witnessed: false,
        engine: "highs",
    }
}

/// Follower optimum at an attack by the dynamic program, with the optimal
/// follower point (model values).
pub fn follower_at(f: &Fire, dp: &TreeDp, attack: &[usize]) -> Option<(i64, Vec<f64>)> {
    let mut sc = dp.scratch();
    for &v in attack {
        sc.mask[v] = 0b0101;
    }
    let value = dp.solve(&mut sc)?;
    dp.trace(&mut sc);
    Some((value, f.values(attack, &sc.states)))
}

/// The follower's optimum at an attack by HiGHS (the follower MILP built
/// from the raw model), as the saved weight.
pub fn follower_highs(m: &Bilevel, f: &Fire, attack: &[usize], time: f64) -> Option<f64> {
    let mut val = vec![0.0; f.nvars];
    for &v in attack {
        val[f.y[v]] = 1.0;
    }
    let (_, phi, _) = check_point(m, &val, time);
    phi.map(|p| -p)
}

/// A deterministic pseudo-random attack of `k` nodes (SplitMix64).
pub fn random_attack(n: usize, k: usize, seed: &mut u64) -> Vec<usize> {
    let mut next = || {
        *seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    let mut pool: Vec<usize> = (0..n).collect();
    for i in 0..k.min(n) {
        let j = i + (next() % (n - i) as u64) as usize;
        pool.swap(i, j);
    }
    let mut a = pool[..k.min(n)].to_vec();
    a.sort();
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Row, Var};

    /// A BOBILib-style attack-protect model on the given edges.
    fn model(n: usize, edges: &[(usize, usize)], phi: usize, lambda: usize) -> Bilevel {
        let mut m = Bilevel {
            name: "test".into(),
            ..Default::default()
        };
        let var = |name: String, obj: f64, lower: bool, ll: f64| Var {
            name,
            lb: 0.0,
            ub: 1.0,
            integer: true,
            obj,
            lower,
            ll_obj: ll,
        };
        for v in 0..n {
            m.vars.push(var(format!("a_{v}"), 1.0, true, -1.0));
        }
        for v in 0..n {
            m.vars.push(var(format!("x_{v}"), 0.0, true, 0.0));
        }
        for v in 0..n {
            m.vars.push(var(format!("y_{v}"), 0.0, false, 0.0));
        }
        let (a, x, y) = (|v: usize| v, |v: usize| n + v, |v: usize| 2 * n + v);
        let row = |name: String, lower: bool, coefs: Vec<(usize, f64)>, hi: f64| Row {
            name,
            lo: f64::NEG_INFINITY,
            hi,
            lower,
            coefs,
        };
        m.rows.push(row(
            "Phi".into(),
            false,
            (0..n).map(|v| (y(v), 1.0)).collect(),
            phi as f64,
        ));
        m.rows.push(row(
            "Lambda".into(),
            true,
            (0..n).map(|v| (x(v), 1.0)).collect(),
            lambda as f64,
        ));
        for v in 0..n {
            m.rows.push(row(
                format!("V{v}"),
                true,
                vec![(a(v), 1.0), (y(v), 1.0)],
                1.0,
            ));
        }
        for &(u, v) in edges {
            for (p, q) in [(u, v), (v, u)] {
                m.rows.push(row(
                    format!("A{p}-{q}"),
                    true,
                    vec![(a(q), 1.0), (a(p), -1.0), (x(q), -1.0)],
                    0.0,
                ));
            }
        }
        m
    }

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *seed >> 33
    }

    fn random_forest(n: usize, seed: &mut u64) -> Vec<(usize, usize)> {
        // Each node links to an earlier one, or to nothing (a new component).
        (1..n)
            .filter_map(|v| {
                let r = lcg(seed) as usize % (v + 1);
                (r < v).then_some((r, v))
            })
            .collect()
    }

    /// Follower optimum by enumerating every (a, x) and checking every row.
    fn brute_follower(m: &Bilevel, f: &Fire, attack: &[usize]) -> i64 {
        let n = f.n();
        let mut best = i64::MIN;
        for bits in 0u64..(1 << (2 * n)) {
            let states: Vec<u8> = (0..n).map(|v| ((bits >> (2 * v)) & 3) as u8).collect();
            let val = f.values(attack, &states);
            if m.max_violation(&val).0 == 0.0 {
                best = best.max(m.upper_obj(&val) as i64);
            }
        }
        best
    }

    #[test]
    fn dp_matches_brute_force_on_random_forests() {
        let mut seed = 7u64;
        for trial in 0..300 {
            let n = 1 + (lcg(&mut seed) as usize % 7);
            let edges = random_forest(n, &mut seed);
            let lambda = lcg(&mut seed) as usize % 3;
            let m = model(n, &edges, n, lambda);
            let f = Fire::from_model(&m).unwrap();
            assert_eq!(f.edges.len(), edges.len());
            let dp = TreeDp::new(&f).unwrap();
            let k = lcg(&mut seed) as usize % (n + 1);
            let attack = random_attack(n, k, &mut seed);
            let (v, val) = follower_at(&f, &dp, &attack).unwrap();
            assert_eq!(m.max_violation(&val).0, 0.0, "trial {trial}");
            assert_eq!(m.upper_obj(&val) as i64, v, "trial {trial}");
            assert_eq!(
                v,
                brute_follower(&m, &f, &attack),
                "trial {trial}: {edges:?} {attack:?} L={lambda}"
            );
            let attacked: Vec<bool> = (0..n).map(|u| attack.contains(&u)).collect();
            let protected: Vec<bool> = (0..n).map(|u| val[f.x[u]] > 0.5).collect();
            assert_eq!(saved_by_fixpoint(&f, &attacked, &protected), v);
        }
    }

    #[test]
    fn enumeration_matches_brute_force_bilevel() {
        let mut seed = 11u64;
        for _ in 0..40 {
            let n = 2 + (lcg(&mut seed) as usize % 5);
            let edges = random_forest(n, &mut seed);
            let phi = lcg(&mut seed) as usize % 3;
            let lambda = lcg(&mut seed) as usize % 3;
            let m = model(n, &edges, phi, lambda);
            let f = Fire::from_model(&m).unwrap();
            let dp = TreeDp::new(&f).unwrap();
            let e = enumerate(&m, &f, &dp, true);
            assert_eq!(e.witness_failures(), 0);
            let mut best = i64::MAX;
            for bits in 0u64..(1 << n) {
                let attack: Vec<usize> = (0..n).filter(|&v| bits >> v & 1 == 1).collect();
                if attack.len() <= phi {
                    best = best.min(brute_follower(&m, &f, &attack));
                }
            }
            assert_eq!(e.min(), Some(best));
            assert_eq!(
                e.attacks(),
                (0..=phi.min(n)).map(|k| binom(n, k)).sum::<u64>()
            );
        }
    }

    #[test]
    fn rejects_other_models() {
        let mut m = model(3, &[(0, 1), (1, 2)], 1, 1);
        assert!(Fire::from_model(&m).is_ok());
        m.rows.last_mut().unwrap().hi = 1.0;
        assert!(Fire::from_model(&m).is_err());
        let mut m = model(3, &[(0, 1), (1, 2)], 1, 1);
        m.rows.pop(); // drop one arc of an edge
        assert!(Fire::from_model(&m).is_err());
        let m = model(3, &[(0, 1), (1, 2), (0, 2)], 1, 1);
        let f = Fire::from_model(&m).unwrap();
        assert!(!f.is_forest() && TreeDp::new(&f).is_none());
    }

    #[test]
    fn combos_are_complete() {
        for n in 0..8 {
            for k in 0..=n {
                let mut seen = Vec::new();
                for_each_combo(&[], 0, n, k, |c| seen.push(c.to_vec()));
                assert_eq!(seen.len() as u64, binom(n, k), "n={n} k={k}");
                assert!(seen.windows(2).all(|w| w[0] < w[1]));
            }
        }
    }

    #[test]
    fn bobilib_tree_instance() {
        let p = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/collection/interdiction/network-interdiction/inter-fire/tree-25_1-3-3_001.mps.gz"
        ));
        if !p.exists() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        let m = crate::mps::read_instance(p).unwrap();
        let f = Fire::from_model(&m).unwrap();
        let dp = TreeDp::new(&f).unwrap();
        let mut seed = 3u64;
        for _ in 0..20 {
            let attack = random_attack(f.n(), f.phi, &mut seed);
            let (v, _) = follower_at(&f, &dp, &attack).unwrap();
            let h = follower_highs(&m, &f, &attack, 60.0).unwrap();
            assert!((h - v as f64).abs() < 1e-6, "{attack:?}: dp {v} highs {h}");
        }
    }
}
