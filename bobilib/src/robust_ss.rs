//! Robust subset sum: can a leader with a budget delete items so that no
//! subset of the remaining items sums into a target band `[lo, hi]`?
//!
//! This decides the subset-sum classes of knapsack interdiction (profit equal
//! to the follower's weight), where the follower's best response is simply
//! the largest reachable sum not above its capacity. Asking whether the
//! leader can push that value below `lo` is a yes/no question we answer by
//! exhaustive search over a much smaller space than the items themselves:
//!
//! * **Count formulation.** Items with the same follower weight are
//!   interchangeable for the follower, so all that matters is how many copies
//!   of each weight survive, and a leader removing `r` copies of a weight
//!   should remove the `r` cheapest. The search branches on these counts.
//! * **Pruning, from the follower's side.** A node is *safe* (no blocking
//!   leader choice below it) when the band is already reachable from decided
//!   survivors plus copies the leader cannot afford to delete, or when the
//!   reachable sums contain a contiguous window that later weights can only
//!   extend (see `Search::window_safe`).
//! * **Robust online tables** ([`RobustTable`]): in a relaxation where the
//!   leader deletes class by class and reacts to the follower's earlier
//!   choices, the follower can still guarantee some sums. The tables are
//!   antitone in the budget, so they are stored as one budget threshold per
//!   (class, sum), which makes them cheap enough to build in any *play
//!   order* of the undecided classes, and at every node that would branch,
//!   with that node's decided sums as the set the game must end in
//!   (`Search::node_table_threshold`). Playing the classes that are cheapest
//!   to delete per unit of mass first (`ratioasc`) is strongest.
//!
//! When every node is safe the follower always reaches the band, which proves
//! the leader cannot do better than `lo`.

use std::collections::HashMap;

/// One follower weight class: `value` is the follower weight (and profit);
/// `costs` are the leader's costs of deleting each copy, ascending.
#[derive(Clone, Debug)]
pub struct Class {
    pub value: usize,
    pub costs: Vec<i64>,
}

/// Restrict the follower to at most `k` copies of each class. The leader then
/// lowers a class's usable copies one at a time; the first step costs the
/// cheapest `n - k + 1` copies together, later steps one more copy each.
/// The follower only gets weaker, so `Safe` in the restricted game proves
/// `Safe` in the original; a `Blocking` answer proves nothing.
pub fn restrict_copies(classes: &mut [Class], k: usize) {
    for c in classes.iter_mut() {
        let n = c.costs.len();
        if k == 0 || n <= k {
            continue;
        }
        let first: i64 = c.costs[..n - k + 1].iter().sum();
        let mut v = vec![first];
        v.extend_from_slice(&c.costs[n - k + 1..]);
        c.costs = v;
    }
}

#[derive(Clone, Debug)]
pub enum Outcome {
    /// No leader choice within budget avoids the band.
    Safe,
    /// Copies to delete per class (same order as the input) that keep every
    /// subset sum out of the band.
    Blocking(Vec<usize>),
    /// The node limit was reached.
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub nodes: u64,
    pub safe_direct: u64,
    pub safe_guaranteed: u64,
    pub safe_window: u64,
    pub safe_online: u64,
    pub safe_band: u64,
    /// Nodes certified by a robust table built for the node itself (base S).
    pub safe_node_table: u64,
    /// Nodes where that table was built.
    pub node_tables: u64,
    pub cache_hits: u64,
    pub leaves: u64,
    /// Prunes per table of `Search::tables`, by label.
    pub table_hits: Vec<(String, u64)>,
    /// Nodes per depth (class index).
    pub depth: Vec<u64>,
}

/// A bitset of reachable sums `0..=hi`.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Sums(Vec<u64>);

impl Sums {
    fn new(hi: usize) -> Self {
        let mut s = Sums(vec![0; hi / 64 + 1]);
        s.0[0] = 1;
        s
    }
    fn get(&self, i: usize) -> bool {
        self.0[i / 64] >> (i % 64) & 1 == 1
    }
    /// self |= self << v, truncated to `hi`.
    fn add(&mut self, v: usize, hi: usize) {
        let words = self.0.len();
        let (ws, bs) = (v / 64, v % 64);
        for i in (ws..words).rev() {
            let mut x = self.0[i - ws] << bs;
            if bs > 0 && i > ws {
                x |= self.0[i - ws - 1] >> (64 - bs);
            }
            self.0[i] |= x;
        }
        let extra = (words * 64 - 1) - hi;
        if extra > 0 {
            let last = words - 1;
            self.0[last] &= u64::MAX >> extra;
        }
    }
    fn any_in(&self, lo: usize, hi: usize) -> bool {
        (lo..=hi).any(|i| self.get(i))
    }
}

/// Robust online tables. For a *play order* of the undecided classes and a
/// band width `w >= 1`, `G_w(k, B)` holds the `t` for which the follower can
/// guarantee that the undecided classes contribute a sum in `[t - w + 1, t]`,
/// whatever the leader deletes with budget `B`, in the relaxation where the
/// leader decides class by class (in play order) and may react to the
/// follower's choices on earlier classes. With `p` the first class in play
/// order and `G'` the table of the classes after it:
///
/// t in G_w(B)  iff  for every r copies deleted from p (cost c_p(r) <= B)
///                   some j <= n_p - r has t - j v_p in G'_w(B - c_p(r)),
///
/// and `G_w(B) = [0, w - 1]` once no class is left. `H = G_1` is the robust
/// table of single targets. Reacting only helps the leader, so `G_w` is a
/// subset of what the follower can guarantee in the real game, for *any*
/// play order (`Bobi.G_sound` in `lean/Bobi/BandTable.lean`: the induction
/// never uses which class comes first; `Bobi.GA_sound_perm` in
/// `lean/Bobi/EndTable.lean` states it for a play order that permutes the
/// search order).
///
/// **Threshold form.** `G_w(B)` is antitone in `B`: by induction, for
/// `B' <= B` every deletion affordable with `B'` is affordable with `B` and
/// `G'_w(B' - c) ⊇ G'_w(B - c)`; the base case does not depend on `B`. So
/// one number per `t` describes the table for every budget:
/// `thr(t) = max { B in 0..=cap : t in G_w(B) }` (or -1 if there is none),
/// and `t in G_w(B)` iff `B <= thr(t)` for every `B <= cap`. The recursion
/// becomes a min-max: with `P(r)` the cost of the `r` cheapest copies of `p`
/// and `M_r(t) = max_{j <= n_p - r, j v_p <= t} thr'(t - j v_p)`, deletion `r`
/// defeats the follower exactly for the budgets `B >= P(r)` with
/// `B - P(r) > M_r(t)`, that is `B > P(r) + M_r(t)` (as `M_r(t) >= -1`), so
///
/// thr(t) = min(cap, min_r (P(r) + M_r(t))).
///
/// (Capping `thr'` at `cap` does not change the result, since
/// `min(cap, P + min(cap, x)) = min(cap, P + x)` for `P >= 0`.) This stores
/// `(hi + 1)` numbers per row instead of `(cap + 1)` bitsets of `hi + 1`
/// bits, and builds in `O(hi * n_p^2)` per class instead of
/// `O(cap * hi / 64 * n_p^2)`; `tests::threshold_matches_bitset` checks it
/// against the bitset recursion. In Lean: `Bobi.G_antitone` and
/// `Bobi.GA_antitone` (`lean/Bobi/EndTable.lean`); `Bobi.thr` is this
/// recursion and `Bobi.thr_iff`, `Bobi.G_iff_thr` the equivalence
/// (`lean/Bobi/Threshold.lean`).
#[derive(Clone)]
pub struct RobustTable {
    /// Band width `w` (1 for `H`).
    pub width: usize,
    /// The play order: `asc` for the search order, else a name of
    /// [`play_order`].
    pub kind: String,
    /// `kind` and width, for statistics.
    pub label: String,
    stride: usize,
    blocks: usize,
    cap: i32,
    /// `thr[k * stride + t]`: the threshold of `t` for the classes `k..` of
    /// the search order (in this table's play order).
    thr: Vec<i32>,
    /// `blockmax[k * blocks + t / 64]`: the largest threshold in the block.
    blockmax: Vec<i32>,
    /// `rowmax[k]`: the largest threshold in row `k`.
    rowmax: Vec<i32>,
}

impl RobustTable {
    pub fn build(classes: &[Class], hi: usize, budget: i64) -> Self {
        Self::build_band(classes, hi, budget, 1)
    }

    fn base(hi: usize, w: usize, cap: i32) -> Vec<i32> {
        (0..=hi).map(|t| if t < w { cap } else { -1 }).collect()
    }

    /// One step of the recursion: `next` holds the thresholds of the classes
    /// played after `c`, `out` receives those of `c` followed by them.
    fn step(next: &[i32], c: &Class, cap: i32, out: &mut [i32]) {
        let n = c.costs.len();
        let v = c.value;
        let mut prefix = vec![0i32];
        for &x in &c.costs {
            let p = prefix.last().unwrap() + x as i32;
            if p > cap {
                break;
            }
            prefix.push(p);
        }
        let rmax = prefix.len() - 1;
        let len = next.len();
        // mx_j[t] = max over i <= j (i v <= t) of next[t - i v], built in
        // place for j = 0..=n; deletion r = n - j leaves j copies.
        let mut mx = next.to_vec();
        out[..len].iter_mut().for_each(|o| *o = cap);
        for j in 0..=n {
            if j > 0 {
                let d = j * v;
                if d < len {
                    for (m, &x) in mx[d..].iter_mut().zip(&next[..len - d]) {
                        *m = (*m).max(x);
                    }
                }
            }
            let r = n - j;
            if r <= rmax {
                let p = prefix[r];
                for (o, &m) in out[..len].iter_mut().zip(&mx) {
                    *o = (*o).min(p + m);
                }
            }
        }
    }

    /// [`RobustTable::step`] on the residuals `lo_t..=hi_t` only: `next`
    /// must hold correct values at `lo_t - n_c v_c..=hi_t` (indices below 0
    /// are skipped as in `step`). Returns the largest value written (-1 if
    /// the range is empty). `mx` is scratch space.
    fn step_range(
        next: &[i32],
        c: &Class,
        cap: i32,
        out: &mut [i32],
        lo_t: usize,
        hi_t: usize,
        mx: &mut Vec<i32>,
    ) -> i32 {
        if lo_t > hi_t {
            return -1;
        }
        let n = c.costs.len();
        let v = c.value;
        let mut prefix = [0i32; 64];
        let mut rmax = 0;
        for &x in c.costs.iter().take(63) {
            let p = prefix[rmax] + x as i32;
            if p > cap {
                break;
            }
            rmax += 1;
            prefix[rmax] = p;
        }
        if n > 63 {
            // Very large classes: fall back to the plain step.
            let mut full = vec![0i32; next.len()];
            Self::step(next, c, cap, &mut full);
            out[lo_t..=hi_t].copy_from_slice(&full[lo_t..=hi_t]);
            return out[lo_t..=hi_t].iter().copied().max().unwrap_or(-1);
        }
        let len = hi_t - lo_t + 1;
        mx.clear();
        mx.extend_from_slice(&next[lo_t..=hi_t]);
        let o = &mut out[lo_t..=hi_t];
        o.iter_mut().for_each(|x| *x = cap);
        for j in 0..=n {
            if j > 0 {
                let d = j * v;
                // mx[idx] covers t = lo_t + idx; next[t - d] needs t >= d.
                let idx0 = d.saturating_sub(lo_t);
                if idx0 < len {
                    let src = &next[lo_t + idx0 - d..lo_t + len - d];
                    for (m, &x) in mx[idx0..].iter_mut().zip(src) {
                        *m = (*m).max(x);
                    }
                }
            }
            let r = n - j;
            if r <= rmax {
                let p = prefix[r];
                for (x, &m) in o.iter_mut().zip(mx.iter()) {
                    *x = (*x).min(p + m);
                }
            }
        }
        o.iter().copied().max().unwrap_or(-1)
    }

    fn new(width: usize, kind: &str, hi: usize, cap: i32, rows: usize) -> Self {
        let stride = hi + 1;
        let blocks = stride.div_ceil(64);
        RobustTable {
            width,
            kind: kind.to_string(),
            label: format!("{kind}{width}"),
            stride,
            blocks,
            cap,
            thr: vec![-1; rows * stride],
            blockmax: vec![-1; rows * blocks],
            rowmax: vec![-1; rows],
        }
    }

    fn finish(&mut self) {
        let rows = self.thr.len() / self.stride;
        for k in 0..rows {
            for blk in 0..self.blocks {
                let lo = k * self.stride + blk * 64;
                let hi = (k * self.stride + self.stride).min(lo + 64);
                self.blockmax[k * self.blocks + blk] =
                    self.thr[lo..hi].iter().copied().max().unwrap_or(-1);
            }
            self.rowmax[k] = self.blockmax[k * self.blocks..(k + 1) * self.blocks]
                .iter()
                .copied()
                .max()
                .unwrap_or(-1);
        }
    }

    /// The band table with the search order as play order (row `k` for the
    /// classes `k..` played in order `k, k + 1, ...`). All rows share one
    /// backward pass.
    pub fn build_band(classes: &[Class], hi: usize, budget: i64, w: usize) -> Self {
        let m = classes.len();
        let cap = budget.max(0) as i32;
        let mut t = Self::new(w, "asc", hi, cap, m + 1);
        let s = t.stride;
        t.thr[m * s..].copy_from_slice(&Self::base(hi, w, cap));
        for k in (0..m).rev() {
            let (cur, next) = t.thr.split_at_mut((k + 1) * s);
            Self::step(&next[..s], &classes[k], cap, &mut cur[k * s..]);
        }
        t.finish();
        t
    }

    /// A band table whose row `k` uses its own play order of the classes
    /// `k..`, given by `order(k)` (a permutation of `k..m`, first played
    /// first). Each row needs its own backward pass, `m (m + 1) / 2` steps in
    /// all, but only the final row is kept.
    pub fn build_suffix(
        classes: &[Class],
        hi: usize,
        budget: i64,
        w: usize,
        kind: &str,
        order: impl Fn(usize) -> Vec<usize>,
    ) -> Self {
        let m = classes.len();
        let cap = budget.max(0) as i32;
        let mut t = Self::new(w, kind, hi, cap, m + 1);
        let s = t.stride;
        let base = Self::base(hi, w, cap);
        t.thr[m * s..].copy_from_slice(&base);
        let mut a = base.clone();
        let mut b = base.clone();
        for k in 0..m {
            let ord = order(k);
            debug_assert_eq!(ord.len(), m - k);
            a.copy_from_slice(&base);
            for &i in ord.iter().rev() {
                Self::step(&a, &classes[i], cap, &mut b);
                std::mem::swap(&mut a, &mut b);
            }
            t.thr[k * s..(k + 1) * s].copy_from_slice(&a);
        }
        t.finish();
        t
    }

    /// The threshold of `t` for the classes `k..`: `t in G_w(k, B)` iff
    /// `B <= threshold(k, t)` (for `B <= cap`).
    pub fn threshold(&self, k: usize, t: usize) -> i32 {
        self.thr[k * self.stride + t]
    }

    pub fn contains(&self, k: usize, b: i64, t: usize) -> bool {
        b <= self.cap as i64 && self.threshold(k, t) as i64 >= b
    }

    /// Largest threshold in row `k` (the follower guarantees nothing in this
    /// table above this budget).
    pub fn row_max(&self, k: usize) -> i32 {
        self.rowmax[k]
    }

    /// The band rule for a run `[a, rb]` of decided sums (every sum in it
    /// reachable, `rb - a + 1 >= w`): some `t` in
    /// `[target + w - 1 - rb, target - a]` lies in `G_w(k, b)`. Then the
    /// undecided classes contribute some `c` in `[t - w + 1, t]` whatever
    /// the leader deletes, and `target - c` lies in `[a, rb]`
    /// (`Bobi.band_covers_sound`).
    fn covers_run(&self, k: usize, b: i64, a: usize, rb: usize, lo: usize, hi: usize) -> bool {
        if b > self.cap as i64 {
            return false;
        }
        let w = self.width;
        for target in lo..=hi {
            if target + w < rb + 1 || target < a {
                continue;
            }
            let t_lo = target + w - 1 - rb;
            let t_hi = (target - a).min(self.stride - 1);
            if (t_lo..=t_hi).any(|t| self.threshold(k, t) as i64 >= b) {
                return true;
            }
        }
        false
    }

    /// Is there a target in `[lo, hi]` and a decided sum `s` with
    /// `t = target - s in H(k, b)`? (For `w = 1`; `Bobi.covers_sound`: the
    /// undecided classes contribute exactly `t` whatever the leader deletes.)
    fn covers(&self, k: usize, b: i64, sums: &Sums, lo: usize, hi: usize) -> bool {
        debug_assert_eq!(self.width, 1);
        if b > self.cap as i64 {
            return false;
        }
        let b = b as i32;
        let row = &self.thr[k * self.stride..(k + 1) * self.stride];
        let bm = &self.blockmax[k * self.blocks..(k + 1) * self.blocks];
        for target in lo..=hi.min(self.stride - 1) {
            for (blk, &mx) in bm.iter().enumerate() {
                let t0 = blk * 64;
                if t0 > target {
                    break;
                }
                if mx < b {
                    continue;
                }
                for t in t0..(t0 + 64).min(target + 1) {
                    if row[t] >= b && sums.get(target - t) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Which robust tables the search builds.
#[derive(Clone, Debug)]
pub struct TableConfig {
    /// Widths of the tables in the search order as play order (1 = `H`).
    pub widths: Vec<usize>,
    /// Tables whose row `k` plays the classes `k..` in another order: the
    /// order's name (see [`play_order`]) and the widths.
    pub extra: Vec<(String, Vec<usize>)>,
}

impl Default for TableConfig {
    /// Only `H`: with per-node tables ([`Search::node_order`]) the root band
    /// tables certify almost nothing more (13,458 instead of 13,498 nodes on
    /// interKP-200-100-6-3 with twelve `ratioasc` widths) and cost time.
    fn default() -> Self {
        TableConfig {
            widths: vec![1],
            extra: vec![],
        }
    }
}

/// A play order of the classes `k..` by name: `desc` (largest value
/// first), `ascv` (smallest value first), `costdesc` / `costasc` (by the
/// cost of deleting every copy, most / least expensive first), `ratiodesc`
/// / `ratioasc` (by that cost per unit of mass).
pub fn play_order(classes: &[Class], k: usize, name: &str) -> Vec<usize> {
    let mut idx: Vec<usize> = (k..classes.len()).collect();
    let total = |i: usize| classes[i].costs.iter().sum::<i64>();
    let mass = |i: usize| (classes[i].value * classes[i].costs.len()) as i64;
    match name {
        "desc" => idx.sort_by_key(|&i| std::cmp::Reverse(classes[i].value)),
        "ascv" => idx.sort_by_key(|&i| classes[i].value),
        "costdesc" => idx.sort_by_key(|&i| (std::cmp::Reverse(total(i)), classes[i].value)),
        "costasc" => idx.sort_by_key(|&i| (total(i), classes[i].value)),
        // total(a) / mass(a) compared by cross-multiplication.
        "ratiodesc" => idx.sort_by(|&a, &b| (total(b) * mass(a)).cmp(&(total(a) * mass(b)))),
        "ratioasc" => idx.sort_by(|&a, &b| (total(a) * mass(b)).cmp(&(total(b) * mass(a)))),
        _ => panic!("unknown play order {name}"),
    }
    idx
}

pub fn build_tables(classes: &[Class], hi: usize, budget: i64, cfg: &TableConfig) -> Vec<RobustTable> {
    let mut out = Vec::new();
    for &w in &cfg.widths {
        out.push(RobustTable::build_band(classes, hi, budget, w));
    }
    for (name, widths) in &cfg.extra {
        for &w in widths {
            out.push(RobustTable::build_suffix(classes, hi, budget, w, name, |k| {
                play_order(classes, k, name)
            }));
        }
    }
    out
}

pub struct Search<'a> {
    classes: &'a [Class],
    lo: usize,
    hi: usize,
    pub stats: Stats,
    pub node_limit: u64,
    /// Print a progress line to stderr every this many nodes (0: never).
    pub progress_every: u64,
    started: std::time::Instant,
    /// Memory allowance for the safe-state cache, in bytes.
    pub cache_bytes: usize,
    /// Safe states: (depth, reachable sums) -> largest budget proven safe.
    cache: HashMap<(usize, Sums), i64>,
    choice: Vec<usize>,
    /// sorted[k]: the undecided classes k.. (search order) by ascending value.
    sorted: Vec<Vec<usize>>,
    /// before[k][i]: total value of the first i classes of sorted[k].
    before: Vec<Vec<f64>>,
    /// cover[k][i]: copies of the first i + 1 classes of sorted[k], sorted by
    /// cost per value, as cumulative (mass, cost) for fractional bounds.
    cover: Vec<Vec<Cumulative>>,
    /// mass[k] = total value of classes k.. (all copies).
    mass: Vec<f64>,
    /// Robust online tables (see [`RobustTable`]), checked at every node.
    pub tables: Vec<RobustTable>,
    /// Build a robust table with the node's own decided sums as base at
    /// every node that would branch, in this play order (see
    /// [`Search::node_table_threshold`]).
    pub node_order: Option<String>,
    /// node_play[k]: that play order of the classes k...
    node_play: Vec<Vec<usize>>,
    /// The budget the search started with (cap of the node tables).
    root_budget: i64,
    /// Scratch space of the node tables.
    nt_a: Vec<i32>,
    nt_b: Vec<i32>,
    nt_mx: Vec<i32>,
    /// by_kind[i]: indices into `tables` of one kind (same label prefix) of
    /// width > 1, by ascending width.
    by_kind: Vec<Vec<usize>>,
    /// max_del[k][b]: most value the leader can delete from classes k..
    /// with budget b (exact bounded knapsack, cheapest copies first).
    max_del: Vec<Vec<i64>>,
}

/// Cumulative (mass, cost) of copies sorted by cost per unit of value.
#[derive(Clone, Debug, Default)]
struct Cumulative {
    mass: Vec<f64>,
    cost: Vec<f64>,
}

impl Cumulative {
    fn build(copies: &mut [(f64, f64)]) -> Self {
        // (cost, value): cheapest per unit of value first.
        copies.sort_by(|a, b| (a.0 * b.1).partial_cmp(&(b.0 * a.1)).unwrap());
        let mut c = Cumulative {
            mass: vec![0.0],
            cost: vec![0.0],
        };
        for &(cost, value) in copies.iter() {
            c.mass.push(c.mass.last().unwrap() + value);
            c.cost.push(c.cost.last().unwrap() + cost);
        }
        c
    }

    /// Least (fractional) cost to delete at least `d` mass; infinite if
    /// there is not that much mass.
    fn min_cost(&self, d: f64) -> f64 {
        if d <= 0.0 {
            return 0.0;
        }
        let i = self.mass.partition_point(|&m| m < d);
        if i == self.mass.len() {
            return f64::INFINITY;
        }
        let (m0, c0) = (self.mass[i - 1], self.cost[i - 1]);
        c0 + (d - m0) * (self.cost[i] - c0) / (self.mass[i] - m0)
    }
}

impl<'a> Search<'a> {
    /// `classes` are taken in the given order (the search order): depth k
    /// decides class k. Any order is valid.
    pub fn new(classes: &'a [Class], lo: usize, hi: usize) -> Self {
        let m = classes.len();
        let mut cover = Vec::with_capacity(m);
        let mut sorted = Vec::with_capacity(m);
        let mut before = Vec::with_capacity(m);
        for k in 0..m {
            let mut idx: Vec<usize> = (k..m).collect();
            idx.sort_by_key(|&i| classes[i].value);
            let mut copies: Vec<(f64, f64)> = Vec::new();
            let mut row = Vec::with_capacity(m - k);
            let mut bm = vec![0.0];
            for &i in &idx {
                let c = &classes[i];
                copies.extend(c.costs.iter().map(|&x| (x as f64, c.value as f64)));
                row.push(Cumulative::build(&mut copies.clone()));
                bm.push(bm.last().unwrap() + (c.value * c.costs.len()) as f64);
            }
            cover.push(row);
            sorted.push(idx);
            before.push(bm);
        }
        let mut mass = vec![0.0; m + 1];
        for k in (0..m).rev() {
            mass[k] = mass[k + 1] + (classes[k].value * classes[k].costs.len()) as f64;
        }
        Search {
            classes,
            lo,
            hi,
            stats: Stats::default(),
            node_limit: u64::MAX,
            progress_every: 0,
            started: std::time::Instant::now(),
            cache_bytes: 3 << 30,
            cache: HashMap::new(),
            choice: vec![0; classes.len()],
            sorted,
            before,
            cover,
            mass,
            tables: Vec::new(),
            by_kind: Vec::new(),
            node_order: None,
            node_play: Vec::new(),
            root_budget: 0,
            nt_a: Vec::new(),
            nt_b: Vec::new(),
            nt_mx: Vec::new(),
            max_del: Vec::new(),
        }
    }

    fn build_max_del(&mut self, budget: i64) {
        let m = self.classes.len();
        let nb = budget.max(0) as usize + 1;
        let mut t = vec![vec![0i64; nb]; m + 1];
        for k in (0..m).rev() {
            let c = &self.classes[k];
            let mut prefix = vec![0i64; c.costs.len() + 1];
            for (i, &x) in c.costs.iter().enumerate() {
                prefix[i + 1] = prefix[i] + x;
            }
            for b in 0..nb {
                let mut best = 0;
                for r in 0..=c.costs.len() {
                    if prefix[r] > b as i64 {
                        break;
                    }
                    best = best.max(r as i64 * c.value as i64 + t[k + 1][b - prefix[r] as usize]);
                }
                t[k][b] = best;
            }
        }
        self.max_del = t;
    }

    pub fn run(&mut self, budget: i64) -> Outcome {
        self.build_max_del(budget);
        self.stats.table_hits = self.tables.iter().map(|t| (t.label.clone(), 0)).collect();
        self.by_kind.clear();
        let mut kinds: Vec<&str> = Vec::new();
        for (i, t) in self.tables.iter().enumerate() {
            if t.width == 1 {
                continue;
            }
            let pos = match kinds.iter().position(|&x| x == t.kind) {
                Some(p) => p,
                None => {
                    kinds.push(&t.kind);
                    self.by_kind.push(Vec::new());
                    kinds.len() - 1
                }
            };
            self.by_kind[pos].push(i);
        }
        for v in &mut self.by_kind {
            v.sort_by_key(|&i| self.tables[i].width);
        }
        self.root_budget = budget;
        self.node_play = match &self.node_order {
            Some(name) => (0..self.classes.len())
                .map(|k| play_order(self.classes, k, name))
                .collect(),
            None => Vec::new(),
        };
        let sums = Sums::new(self.hi);
        match self.node(0, budget, &sums) {
            Some(true) => Outcome::Safe,
            Some(false) => Outcome::Blocking(self.choice.clone()),
            None => Outcome::Unknown,
        }
    }

    /// Copies of class k the leader can delete with `budget` (cheapest first).
    fn deletable(&self, k: usize, budget: i64) -> usize {
        let mut spent = 0;
        let mut r = 0;
        for &c in &self.classes[k].costs {
            if spent + c > budget {
                break;
            }
            spent += c;
            r += 1;
        }
        r
    }

    /// Sufficient condition for safety from a contiguous window of reachable
    /// sums [a, b] of width w = b - a + 1. Adding a copy of value v <= w
    /// makes [a, b + v] reachable. Taking the undecided classes in ascending
    /// value order (whatever the search order),
    /// the window keeps growing unless it *stalls*: reaches a class whose
    /// value exceeds the current width. Stalling at class u needs the leader
    /// to delete at least D_u = (undecided mass below u) - v_u + w + 1 of the mass before
    /// u; if even the fractional (cheapest per unit) cost of that exceeds the
    /// budget for every u, no stall is possible and the window reaches
    /// b + (future mass - most mass the leader can delete).
    fn window_safe(&self, k: usize, budget: i64, sums: &Sums) -> bool {
        let order = &self.sorted[k];
        let vk = self.classes[order[0]].value;
        let b_budget = budget as f64;
        let future = self.mass[k] - self.max_del[k][budget.max(0) as usize] as f64;
        // Maximal runs [a, b] of width >= the smallest undecided value,
        // scanned from the top down.
        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i <= self.hi {
            if !sums.get(i) {
                i += 1;
                continue;
            }
            let a = i;
            while i <= self.hi && sums.get(i) {
                i += 1;
            }
            if i - a >= vk {
                runs.push((a, i - 1));
            }
        }
        for &(a, b) in runs.iter().rev() {
            if b as f64 + future < self.lo as f64 {
                return false; // lower runs reach even less
            }
            let w = (b - a + 1) as f64;
            let mut stallable = false;
            for (pos, &u) in order.iter().enumerate() {
                let before = self.before[k][pos];
                let d = before - self.classes[u].value as f64 + w + 1.0;
                if d <= 0.0 {
                    stallable = true;
                    break;
                }
                if pos == 0 {
                    continue; // nothing smaller to delete
                }
                if self.cover[k][pos - 1].min_cost(d) <= b_budget {
                    stallable = true;
                    break;
                }
            }
            if !stallable {
                return true;
            }
        }
        false
    }

    /// Returns Some(true) if safe, Some(false) if a blocking choice was found
    /// (left in `self.choice`), None on the node limit.
    fn node(&mut self, k: usize, budget: i64, sums: &Sums) -> Option<bool> {
        self.stats.nodes += 1;
        if self.stats.depth.len() <= k {
            self.stats.depth.resize(k + 1, 0);
        }
        self.stats.depth[k] += 1;
        if self.progress_every > 0 && self.stats.nodes % self.progress_every == 0 {
            let st = &self.stats;
            let top: Vec<usize> = self.choice[..k.min(6)].to_vec();
            eprintln!(
                "  {:.0}s nodes={} online={} band={} window={} cache={} ({} entries) depth={} choice[..6]={:?}",
                self.started.elapsed().as_secs_f64(),
                st.nodes,
                st.safe_online,
                st.safe_band,
                st.safe_window,
                st.cache_hits,
                self.cache.len(),
                k,
                top
            );
        }
        if self.stats.nodes > self.node_limit {
            return None;
        }
        if sums.any_in(self.lo, self.hi) {
            self.stats.safe_direct += 1;
            return Some(true);
        }
        if k == self.classes.len() {
            self.stats.leaves += 1;
            return Some(false);
        }
        if self.tables_cover(k, budget, sums) {
            return Some(true);
        }
        if let Some(&b) = self.cache.get(&(k, sums.clone())) {
            if b >= budget {
                self.stats.cache_hits += 1;
                return Some(true);
            }
        }
        // Copies the leader cannot afford to delete survive for sure.
        let mut guaranteed = sums.clone();
        for kk in k..self.classes.len() {
            let c = &self.classes[kk];
            let keep = c.costs.len() - self.deletable(kk, budget);
            for _ in 0..keep {
                guaranteed.add(c.value, self.hi);
            }
        }
        if guaranteed.any_in(self.lo, self.hi) {
            self.stats.safe_guaranteed += 1;
            self.remember(k, sums, budget);
            return Some(true);
        }
        if self.window_safe(k, budget, sums) {
            self.stats.safe_window += 1;
            self.remember(k, sums, budget);
            return Some(true);
        }
        if self.node_order.is_some() {
            self.stats.node_tables += 1;
            let thr = self.node_table_threshold(k, sums, budget);
            if thr >= budget {
                self.stats.safe_node_table += 1;
                self.remember(k, sums, thr);
                return Some(true);
            }
        }
        let c = &self.classes[k];
        let v = c.value;
        // Try deleting more first: a blocking leader removes a lot.
        let mut prefix = vec![0i64; c.costs.len() + 1];
        for (i, &x) in c.costs.iter().enumerate() {
            prefix[i + 1] = prefix[i] + x;
        }
        let n = c.costs.len();
        for r in (0..=n).rev() {
            if prefix[r] > budget {
                continue;
            }
            let mut s = sums.clone();
            for _ in 0..(n - r) {
                s.add(v, self.hi);
            }
            self.choice[k] = r;
            match self.node(k + 1, budget - prefix[r], &s) {
                Some(true) => {}
                other => return other,
            }
        }
        self.remember(k, sums, budget);
        Some(true)
    }

    /// The robust-table rules: `H` (a width-1 table) against every decided
    /// sum ([`RobustTable::covers`]), then for each maximal run of decided
    /// sums the widest band table of each kind that fits it
    /// ([`RobustTable::covers_run`]). (Trying every table against every
    /// start its width allows pruned no more nodes on interKP-100-100-6-3
    /// and was ten times slower.)
    fn tables_cover(&mut self, k: usize, budget: i64, sums: &Sums) -> bool {
        let b32 = budget.min(i32::MAX as i64) as i32;
        for (i, t) in self.tables.iter().enumerate() {
            if t.width == 1 && t.row_max(k) >= b32 && t.covers(k, budget, sums, self.lo, self.hi) {
                self.stats.table_hits[i].1 += 1;
                self.stats.safe_online += 1;
                return true;
            }
        }
        let mut i = 0;
        while i <= self.hi {
            if !sums.get(i) {
                i += 1;
                continue;
            }
            let a = i;
            while i <= self.hi && sums.get(i) {
                i += 1;
            }
            let rb = i - 1;
            let width = rb - a + 1;
            for kind in &self.by_kind {
                let Some(&ti) = kind.iter().rev().find(|&&ti| self.tables[ti].width <= width) else {
                    continue;
                };
                let t = &self.tables[ti];
                if t.row_max(k) >= b32 && t.covers_run(k, budget, a, rb, self.lo, self.hi) {
                    self.stats.table_hits[ti].1 += 1;
                    self.stats.safe_band += 1;
                    return true;
                }
            }
        }
        false
    }

    /// The robust table with the decided sums as base. With `A` any set of
    /// residuals, let `G_A` be the recursion of [`RobustTable`] with base
    /// case `G_A = A` once no class is left (the band table `G_w` is
    /// `A = [0, w - 1]`). The induction of `Bobi.G_sound` gives, word for
    /// word: if `t in G_A(B)`, then for every deletion within `B` the copies
    /// left have a subset sum `s` with `t - s in A`. (Leader deletes `r`
    /// from the first class `p` played; the recursion names `j <= n_p - r`
    /// with `t - j v_p in G'_A(B - c_p(r))`; by induction the later classes
    /// give `s'` with `t - j v_p - s' in A`; take `s = j v_p + s'`.) With
    /// `A = S`, the sums the decided survivors reach, `target in G_S(B)`
    /// means the follower hits `target` exactly: `s` from the undecided
    /// survivors plus `target - s` from the decided ones. (`Bobi.GA_sound` in
    /// `lean/Bobi/EndTable.lean`.)
    ///
    /// **A band of targets.** The follower wins with any total in `[lo, hi]`,
    /// so it need not fix the target in advance: take
    /// `A = {t : some s in S has t - (hi - lo) <= s <= t}` and ask whether
    /// `hi in G_A(B)`. If so, every deletion within `B` leaves undecided
    /// survivors with a sum `u` and some `s in S` with
    /// `hi - u - (hi - lo) <= s <= hi - u`, i.e. `lo <= u + s <= hi`. (This
    /// contains each single target `t in [lo, hi]` with base `S`: shifting
    /// the follower's strategy up by `hi - t` keeps every `j` feasible and
    /// ends in `S + (hi - t) ⊆ A`.) In threshold form this returns
    /// `thr_A(hi)` (capped at the root budget): the node is safe for every
    /// budget up to it (`Bobi.node_table_sound` and
    /// `Bobi.node_table_threshold_sound` in `lean/`; the range shortcuts and
    /// the early exit below are not formalised).
    ///
    /// `G_A` contains what the run rules of the tables of the same play
    /// order certify: `H` checks `target - s in G_{{0}}` for one `s in S`,
    /// and `G_{{0}} + s ⊆ G_S` (shift the recursion); a band table checks
    /// `G_{[0, w-1]}` shifted into a run of `S`, likewise contained.
    ///
    /// Only residuals that can matter are computed. Say the play order is
    /// `p_1, .., p_L` and the backward pass is at `p_i`: the follower has
    /// taken at most `Mb_i` (the mass of `p_1..p_{i-1}`) from `hi`, so
    /// residuals below `hi - Mb_i` are never read; and a residual above
    /// `max A + Ma_i` (`Ma_i` the mass of `p_i..p_L`) is -1, since `r = 0`
    /// bounds `thr(t)` by `max_j thr'(t - j v)` and the base is -1 above
    /// `max A`. The same bound shows the largest value never grows along the
    /// pass, so once it is below `budget` the node table fails and the pass
    /// stops (the value returned is then below `budget`, which is all the
    /// caller uses).
    fn node_table_threshold(&mut self, k: usize, sums: &Sums, budget: i64) -> i64 {
        if self.lo > self.hi {
            return -1; // no target at all
        }
        let cap = self.root_budget.clamp(0, i32::MAX as i64) as i32;
        let len = self.hi + 1;
        let mut a = std::mem::take(&mut self.nt_a);
        let mut b = std::mem::take(&mut self.nt_b);
        let mut mx = std::mem::take(&mut self.nt_mx);
        a.clear();
        a.resize(len, -1);
        b.clear();
        b.resize(len, -1);
        // Base A: some decided sum within hi - lo below t.
        let slack = self.hi - self.lo;
        let mut last: Option<usize> = None;
        let mut max_s = 0;
        for t in 0..len {
            if sums.get(t) {
                last = Some(t);
            }
            if matches!(last, Some(l) if l + slack >= t) {
                a[t] = cap;
                max_s = t;
            }
        }
        let play = &self.node_play[k];
        let mass = |i: usize| self.classes[i].value * self.classes[i].costs.len();
        let total: usize = play.iter().map(|&i| mass(i)).sum();
        let mut after = 0; // Ma_i: mass of the classes processed so far, this one included
        let mut result = None;
        for &i in play.iter().rev() {
            after += mass(i);
            let before = total - after;
            let lo_t = self.hi.saturating_sub(before);
            let hi_t = self.hi.min(max_s + after);
            let m = RobustTable::step_range(&a, &self.classes[i], cap, &mut b, lo_t, hi_t, &mut mx);
            std::mem::swap(&mut a, &mut b);
            if (m as i64) < budget {
                result = Some(m as i64);
                break;
            }
        }
        let result = result.unwrap_or(a[self.hi] as i64);
        self.nt_a = a;
        self.nt_b = b;
        self.nt_mx = mx;
        result
    }

    fn remember(&mut self, k: usize, sums: &Sums, budget: i64) {
        if let Some(e) = self.cache.get_mut(&(k, sums.clone())) {
            *e = (*e).max(budget);
            return;
        }
        // Keep the cache within its memory allowance; past that, new safe
        // states are simply not remembered (pruning stays sound).
        let entry_bytes = sums.0.len() * 8 + 64;
        if (self.cache.len() + 1) * entry_bytes <= self.cache_bytes {
            self.cache.insert((k, sums.clone()), budget);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute force over every deletion choice.
    fn brute(classes: &[Class], lo: usize, hi: usize, budget: i64) -> bool {
        fn rec(
            cl: &[Class],
            k: usize,
            budget: i64,
            items: &mut Vec<usize>,
            lo: usize,
            hi: usize,
        ) -> bool {
            if k == cl.len() {
                let mut s = vec![false; hi + 1];
                s[0] = true;
                for &v in items.iter() {
                    for x in (v..=hi).rev() {
                        if s[x - v] {
                            s[x] = true;
                        }
                    }
                }
                return (lo..=hi).any(|i| s[i]);
            }
            let n = cl[k].costs.len();
            let mut spent = 0;
            for r in 0..=n {
                if r > 0 {
                    spent += cl[k].costs[r - 1];
                }
                if spent > budget {
                    break;
                }
                let before = items.len();
                items.extend(std::iter::repeat(cl[k].value).take(n - r));
                let safe = rec(cl, k + 1, budget - spent, items, lo, hi);
                items.truncate(before);
                if !safe {
                    return false;
                }
            }
            true
        }
        rec(classes, 0, budget, &mut Vec::new(), lo, hi)
    }

    #[test]
    fn matches_brute_force() {
        let mut seed = 12345u64;
        let mut rnd = |m: u64| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) % m
        };
        let mut windows = 0;
        let mut bands = 0;
        let (mut desc_hits, mut asc_band_hits, mut online_hits) = (0, 0, 0);
        let mut node_table_hits = 0;
        for round in 0..5000 {
            let nclass = 1 + rnd(8) as usize;
            let mut classes: Vec<Class> = (0..nclass)
                .map(|_| {
                    let copies = 1 + rnd(4) as usize;
                    let mut costs: Vec<i64> = (0..copies).map(|_| rnd(10) as i64).collect();
                    costs.sort();
                    Class {
                        value: 1 + rnd(12) as usize,
                        costs,
                    }
                })
                .collect();
            classes.sort_by_key(|c| c.value);
            classes.dedup_by_key(|c| c.value);
            let total: usize = classes.iter().map(|c| c.value * c.costs.len()).sum();
            let hi = rnd(total as u64 + 1) as usize;
            // Mostly narrow target bands, sometimes wide ones.
            let spread = if round % 4 == 3 { hi as u64 / 2 + 1 } else { 3 };
            let lo = hi.saturating_sub(rnd(spread) as usize);
            let budget = rnd(40) as i64;
            let expect = brute(&classes, lo, hi, budget);
            // Any search order is valid: test a rotated one on some rounds.
            if round % 3 == 2 {
                let r = rnd(classes.len() as u64) as usize;
                classes.rotate_left(r);
            }
            let mut s = Search::new(&classes, lo, hi);
            // Table setups: none, H alone, ascending bands, descending
            // tables of every width, and both kinds.
            let cfg = match round % 5 {
                0 => None,
                1 => Some(TableConfig { widths: vec![1], extra: vec![] }),
                2 => Some(TableConfig { widths: vec![1, 2, 3, 5], extra: vec![] }),
                3 => Some(TableConfig { widths: vec![1], extra: vec![("desc".into(), (1..=6).collect())] }),
                _ => Some(TableConfig {
                    widths: vec![1, 2, 4],
                    extra: vec![
                        ("desc".into(), vec![1, 2, 3, 5]),
                        ("costdesc".into(), vec![1, 3]),
                        ("ratioasc".into(), vec![2]),
                    ],
                }),
            };
            if let Some(cfg) = &cfg {
                s.tables = build_tables(&classes, hi, budget, cfg);
            }
            // Per-node tables (base: the node's decided sums) in several
            // play orders, alone and with the root tables.
            s.node_order = match round % 7 {
                1 => Some("ratioasc".into()),
                3 => Some("desc".into()),
                5 => Some("ascv".into()),
                6 => Some("costdesc".into()),
                _ => None,
            };
            let got = s.run(budget);
            node_table_hits += s.stats.safe_node_table;
            windows += s.stats.safe_window;
            bands += s.stats.safe_band;
            for (label, hits) in &s.stats.table_hits {
                if label.starts_with("desc") {
                    desc_hits += hits;
                } else if label != "asc1" {
                    asc_band_hits += hits;
                } else {
                    online_hits += hits;
                }
            }
            match got {
                Outcome::Safe => assert!(expect, "round {round}: search says safe"),
                Outcome::Blocking(r) => {
                    assert!(
                        !expect,
                        "round {round}: search found a block brute force denies"
                    );
                    // The returned choice must really block.
                    let mut items = Vec::new();
                    let mut spent = 0;
                    for (c, &rr) in classes.iter().zip(&r) {
                        spent += c.costs[..rr].iter().sum::<i64>();
                        items.extend(std::iter::repeat(c.value).take(c.costs.len() - rr));
                    }
                    assert!(spent <= budget);
                    let mut sums = vec![false; hi + 1];
                    sums[0] = true;
                    for &v in &items {
                        for x in (v..=hi).rev() {
                            if sums[x - v] {
                                sums[x] = true;
                            }
                        }
                    }
                    assert!(
                        !(lo..=hi).any(|i| sums[i]),
                        "round {round}: returned choice does not block"
                    );
                }
                Outcome::Unknown => panic!("node limit in test"),
            }
        }
        assert!(windows > 0, "window rule never exercised");
        assert!(bands > 0, "band rule never exercised");
        assert!(desc_hits > 0, "descending tables never exercised");
        assert!(asc_band_hits > 0, "ascending band tables never exercised");
        assert!(online_hits > 0, "online table never exercised");
        assert!(node_table_hits > 0, "per-node tables never exercised");
    }

    /// Per-node tables on their own (no root tables), with wider values and
    /// budgets than `matches_brute_force` so that they decide most nodes;
    /// both answers must occur and blocks must be real.
    #[test]
    fn node_tables_match_brute_force() {
        let mut rnd = lcg(2026);
        let (mut safe, mut blocked, mut hits) = (0, 0, 0);
        for round in 0..5000 {
            let nclass = 2 + rnd(6) as usize;
            let classes = random_classes(&mut rnd, nclass, 3, 12, 30);
            let total: usize = classes.iter().map(|c| c.value * c.costs.len()).sum();
            let hi = rnd(total as u64 / 2 + 1) as usize;
            let spread = if round % 3 == 0 { hi as u64 / 3 + 1 } else { 1 };
            let lo = hi.saturating_sub(rnd(spread) as usize);
            let budget = rnd(30) as i64;
            let expect = brute(&classes, lo, hi, budget);
            let mut s = Search::new(&classes, lo, hi);
            s.node_order = Some(["ratioasc", "desc", "costasc"][round % 3].into());
            let got = s.run(budget);
            hits += s.stats.safe_node_table;
            match got {
                Outcome::Safe => {
                    assert!(expect, "round {round}: search says safe");
                    safe += 1;
                }
                Outcome::Blocking(r) => {
                    assert!(!expect, "round {round}: search found a block brute force denies");
                    let mut sums = vec![false; hi + 1];
                    sums[0] = true;
                    let mut spent = 0;
                    for (c, &rr) in classes.iter().zip(&r) {
                        spent += c.costs[..rr].iter().sum::<i64>();
                        for _ in 0..c.costs.len() - rr {
                            for x in (c.value..=hi).rev() {
                                if sums[x - c.value] {
                                    sums[x] = true;
                                }
                            }
                        }
                    }
                    assert!(spent <= budget && !(lo..=hi).any(|i| sums[i]), "round {round}: not a block");
                    blocked += 1;
                }
                Outcome::Unknown => panic!("node limit in test"),
            }
        }
        eprintln!("node_tables_match_brute_force: {safe} safe, {blocked} blocked, {hits} node-table prunes");
        assert!(safe > 500 && blocked > 500, "safe {safe}, blocked {blocked}");
        assert!(hits > 500, "node tables decided only {hits} nodes");
    }

    /// Larger instances than brute force can enumerate: the search with
    /// per-node tables must agree with the search that uses only the
    /// original rules (direct, guaranteed copies, windows, cache).
    #[test]
    fn node_tables_agree_with_plain_search() {
        let mut rnd = lcg(31337);
        let (mut safe, mut blocked) = (0, 0);
        // SS_TEST_ROUNDS=N runs a longer version.
        let rounds: usize = std::env::var("SS_TEST_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000);
        for round in 0..rounds {
            let nclass = 6 + rnd(10) as usize;
            let classes = random_classes(&mut rnd, nclass, 5, 15, 40);
            let total: usize = classes.iter().map(|c| c.value * c.costs.len()).sum();
            let hi = total / 3 + rnd(total as u64 / 3 + 1) as usize;
            let lo = if round % 2 == 0 { hi } else { hi.saturating_sub(rnd(hi as u64 / 4 + 1) as usize) };
            let budget = rnd(60) as i64;
            let mut plain = Search::new(&classes, lo, hi);
            let expect = plain.run(budget);
            let mut s = Search::new(&classes, lo, hi);
            s.node_order = Some("ratioasc".into());
            let got = s.run(budget);
            match (&expect, &got) {
                (Outcome::Safe, Outcome::Safe) => safe += 1,
                (Outcome::Blocking(_), Outcome::Blocking(_)) => blocked += 1,
                _ => panic!("round {round}: plain {expect:?}, with node tables {got:?}"),
            }
        }
        eprintln!("node_tables_agree_with_plain_search: {safe} safe, {blocked} blocked");
        assert!(safe > rounds / 10 && blocked > rounds / 10, "safe {safe}, blocked {blocked}");
    }

    /// The online game with base set `a`, straight from its definition: the
    /// classes `cl` are played in order; can the follower force the final
    /// residual into `a` from residual `t` with the leader's budget `b`?
    fn online_game(cl: &[Class], a: &[bool], b: i64, t: usize) -> bool {
        let Some((c, rest)) = cl.split_first() else {
            return a[t];
        };
        let n = c.costs.len();
        let mut spent = 0;
        for r in 0..=n {
            if r > 0 {
                spent += c.costs[r - 1];
            }
            if spent > b {
                break;
            }
            let ok = (0..=n - r).any(|j| j * c.value <= t && online_game(rest, a, b - spent, t - j * c.value));
            if !ok {
                return false;
            }
        }
        true
    }

    /// The per-node table: its threshold is exactly the online game with the
    /// decided sums as base set (checked against the definition), and what
    /// it certifies holds in the real game (checked by enumerating every
    /// deletion).
    #[test]
    fn node_table_matches_definition_and_is_sound() {
        let mut rnd = lcg(99);
        let mut certified = 0;
        for round in 0..400 {
            let nclass = 1 + rnd(6) as usize;
            let classes = random_classes(&mut rnd, nclass, 3, 7, 9);
            let total: usize = classes.iter().map(|c| c.value * c.costs.len()).sum();
            let hi = rnd(total as u64 + 6) as usize;
            let lo = hi.saturating_sub(rnd(4) as usize);
            let budget = rnd(20) as i64;
            let name = ["ratioasc", "desc", "ascv", "costasc"][round % 4];
            let mut s = Search::new(&classes, lo, hi);
            s.node_order = Some(name.into());
            s.root_budget = budget;
            s.node_play = (0..classes.len()).map(|k| play_order(&classes, k, name)).collect();
            for k in 0..=classes.len() {
                if k == classes.len() {
                    s.node_play.push(Vec::new());
                }
                // A random set of decided sums containing 0.
                let mut sums = Sums::new(hi);
                for x in 1..=hi {
                    if rnd(3) > 0 {
                        sums.0[x / 64] |= 1 << (x % 64);
                    }
                }
                let a: Vec<bool> = (0..=hi).map(|x| sums.get(x)).collect();
                // The base of the band game: a decided sum within hi - lo below.
                let dil: Vec<bool> = (0..=hi)
                    .map(|x| (x.saturating_sub(hi - lo)..=x).any(|y| a[y]))
                    .collect();
                let play: Vec<Class> = s.node_play[k].iter().map(|&i| classes[i].clone()).collect();
                // Without early exit (budget 0 never stops the pass) the
                // threshold is exact; with the node's budget the answer to
                // "at least b?" is the same.
                let thr = s.node_table_threshold(k, &sums, 0);
                for b in 0..=budget {
                    let game = online_game(&play, &dil, b, hi);
                    // Never weaker than fixing one target with base S.
                    assert!(game || !(lo..=hi).any(|t| online_game(&play, &a, b, t)));
                    assert_eq!(thr >= b, game, "round {round} k={k} b={b}: threshold {thr}");
                    let thr_b = s.node_table_threshold(k, &sums, b);
                    assert_eq!(thr_b >= b, game, "round {round} k={k} b={b}: early exit");
                    if game {
                        assert_eq!(thr_b, thr, "round {round} k={k} b={b}: value on success");
                    }
                    if thr < b {
                        continue;
                    }
                    certified += 1;
                    each_deletion(&classes[k..], b, hi, &mut |left| {
                        let hit = (lo..=hi).any(|t| (0..=t).any(|x| left[x] && a[t - x]));
                        assert!(hit, "round {round} k={k} b={b}: a deletion avoids every target");
                    });
                }
            }
        }
        assert!(certified > 500);
    }

    fn random_classes(rnd: &mut impl FnMut(u64) -> u64, nclass: usize, copies: u64, cost: u64, value: u64) -> Vec<Class> {
        let mut classes: Vec<Class> = (0..nclass)
            .map(|_| {
                let n = 1 + rnd(copies) as usize;
                let mut costs: Vec<i64> = (0..n).map(|_| rnd(cost) as i64).collect();
                costs.sort();
                Class {
                    value: 1 + rnd(value) as usize,
                    costs,
                }
            })
            .collect();
        classes.sort_by_key(|c| c.value);
        classes.dedup_by_key(|c| c.value);
        classes
    }

    fn lcg(seed: u64) -> impl FnMut(u64) -> u64 {
        let mut seed = seed;
        move |m: u64| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) % m
        }
    }

    /// The bitset recursion the tables used before the threshold form: one
    /// set of targets per budget `0..=budget`, rows `k = 0..=m`.
    fn bitset_band(classes: &[Class], hi: usize, budget: i64, w: usize) -> Vec<Vec<Vec<bool>>> {
        let m = classes.len();
        let nb = budget.max(0) as usize + 1;
        let mut g = vec![vec![vec![false; hi + 1]; nb]; m + 1];
        for b in 0..nb {
            for t in 0..w.min(hi + 1) {
                g[m][b][t] = true;
            }
        }
        for k in (0..m).rev() {
            let c = &classes[k];
            let n = c.costs.len();
            let mut prefix = vec![0i64; n + 1];
            for (i, &x) in c.costs.iter().enumerate() {
                prefix[i + 1] = prefix[i] + x;
            }
            for b in 0..nb {
                for t in 0..=hi {
                    g[k][b][t] = (0..=n).filter(|&r| prefix[r] <= b as i64).all(|r| {
                        let b2 = b - prefix[r] as usize;
                        (0..=n - r).any(|j| j * c.value <= t && g[k + 1][b2][t - j * c.value])
                    });
                }
            }
        }
        g
    }

    /// The threshold tables describe exactly the bitset recursion, for the
    /// search order and (row by row) for the descending play order.
    #[test]
    fn threshold_matches_bitset() {
        let mut rnd = lcg(777);
        for _ in 0..300 {
            let nclass = 1 + rnd(6) as usize;
            let classes = random_classes(&mut rnd, nclass, 4, 8, 10);
            let total: usize = classes.iter().map(|c| c.value * c.costs.len()).sum();
            let hi = rnd(total as u64 + 4) as usize;
            let budget = rnd(30) as i64;
            let w = 1 + rnd(5) as usize;
            let m = classes.len();
            let t = RobustTable::build_band(&classes, hi, budget, w);
            let g = bitset_band(&classes, hi, budget, w);
            let d = build_tables(&classes, hi, budget, &TableConfig { widths: vec![], extra: vec![("desc".into(), vec![w])] });
            for k in 0..=m {
                for b in 0..=budget {
                    for x in 0..=hi {
                        assert_eq!(t.contains(k, b, x), g[k][b as usize][x], "asc k={k} b={b} t={x}");
                    }
                }
                // Row k of the descending table is row 0 of the bitset table
                // of the classes k.. sorted by descending value.
                let mut rev: Vec<Class> = classes[k..].to_vec();
                rev.sort_by_key(|c| std::cmp::Reverse(c.value));
                let gr = bitset_band(&rev, hi, budget, w);
                for b in 0..=budget {
                    for x in 0..=hi {
                        assert_eq!(d[0].contains(k, b, x), gr[0][b as usize][x], "desc k={k} b={b} t={x}");
                    }
                }
            }
        }
    }

    /// Every subset sum of the copies left by some deletion of the classes
    /// `k..` with cost at most `b` (cheapest copies first): calls `f` with
    /// the reachable sums of each deletion.
    fn each_deletion(classes: &[Class], budget: i64, hi: usize, f: &mut impl FnMut(&[bool])) {
        fn rec(cl: &[Class], i: usize, budget: i64, sums: Vec<bool>, f: &mut impl FnMut(&[bool])) {
            if i == cl.len() {
                f(&sums);
                return;
            }
            let n = cl[i].costs.len();
            let mut spent = 0;
            for r in 0..=n {
                if r > 0 {
                    spent += cl[i].costs[r - 1];
                }
                if spent > budget {
                    break;
                }
                let mut s = sums.clone();
                for _ in 0..n - r {
                    for x in (cl[i].value..s.len()).rev() {
                        if s[x - cl[i].value] {
                            s[x] = true;
                        }
                    }
                }
                rec(cl, i + 1, budget - spent, s, f);
            }
        }
        let mut s = vec![false; hi + 1];
        s[0] = true;
        rec(classes, 0, budget, s, f);
    }

    /// What a table promises holds in the real game: if `thr(k, t) >= b`,
    /// every deletion within `b` leaves a subset sum in `[t - w + 1, t]`.
    #[test]
    fn tables_sound_by_enumeration() {
        let mut rnd = lcg(4242);
        let mut promises = 0;
        for _ in 0..200 {
            let nclass = 1 + rnd(5) as usize;
            let classes = random_classes(&mut rnd, nclass, 3, 6, 9);
            let total: usize = classes.iter().map(|c| c.value * c.costs.len()).sum();
            let hi = rnd(total as u64 + 4) as usize;
            let budget = rnd(16) as i64;
            let tables = build_tables(
                &classes,
                hi,
                budget,
                &TableConfig {
                    widths: vec![1, 2, 3],
                    extra: ["desc", "ascv", "costdesc", "costasc", "ratiodesc", "ratioasc"]
                        .iter()
                        .map(|n| (n.to_string(), vec![1, 2, 4]))
                        .collect(),
                },
            );
            for k in 0..=classes.len() {
                for b in 0..=budget {
                    // Sums each deletion within b leaves (as bitsets of 0..=hi).
                    let mut outcomes: Vec<Vec<bool>> = Vec::new();
                    each_deletion(&classes[k..], b, hi, &mut |s| outcomes.push(s.to_vec()));
                    for tb in &tables {
                        let w = tb.width;
                        for t in 0..=hi {
                            if !tb.contains(k, b, t) {
                                continue;
                            }
                            promises += 1;
                            for s in &outcomes {
                                assert!(
                                    ((t + 1).saturating_sub(w)..=t).any(|x| s[x]),
                                    "{} k={k} b={b} t={t}: a deletion leaves no sum in the band",
                                    tb.label
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(promises > 1000);
    }
}
