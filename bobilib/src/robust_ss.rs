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
//!   extend (see [`window_safe`]).
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
    pub cache_hits: u64,
    pub leaves: u64,
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
    /// self = self << v, truncated to `hi`.
    fn shift_left(&mut self, v: usize, hi: usize) {
        let words = self.0.len();
        let (ws, bs) = (v / 64, v % 64);
        for i in (0..words).rev() {
            let mut x = if i >= ws { self.0[i - ws] << bs } else { 0 };
            if bs > 0 && i > ws {
                x |= self.0[i - ws - 1] >> (64 - bs);
            }
            self.0[i] = x;
        }
        let extra = (words * 64 - 1) - hi;
        if extra > 0 {
            self.0[words - 1] &= u64::MAX >> extra;
        }
    }

    fn any_in(&self, lo: usize, hi: usize) -> bool {
        (lo..=hi).any(|i| self.get(i))
    }
}

/// Robust online sums: `H(k, B)` holds targets the follower can hit from
/// classes `k..` whatever the leader deletes with budget `B`, in the
/// relaxation where the leader decides class by class and may react to the
/// follower's choices on earlier classes. Since reacting only helps the
/// leader, `H(k, B)` is a subset of the true robust sums:
///
/// t in H(k, B)  iff  for every r copies deleted from class k (cost c_k(r) <= B)
///                    some j <= n_k - r has t - j v_k in H(k + 1, B - c_k(r)),
///
/// with H(m, B) = {0}.
pub struct RobustTable {
    words: usize,
    budget: usize,
    data: Vec<u64>,
}

impl RobustTable {
    pub fn build(classes: &[Class], hi: usize, budget: i64) -> Self {
        let words = hi / 64 + 1;
        let nb = budget.max(0) as usize + 1;
        let m = classes.len();
        let mut data = vec![0u64; (m + 1) * nb * words];
        for b in 0..nb {
            data[(m * nb + b) * words] = 1;
        }
        let mut acc = Sums(vec![0; words]);
        let mut uni = Sums(vec![0; words]);
        for k in (0..m).rev() {
            let c = &classes[k];
            let n = c.costs.len();
            let mut prefix = vec![0i64; n + 1];
            for (i, &x) in c.costs.iter().enumerate() {
                prefix[i + 1] = prefix[i] + x;
            }
            for b in 0..nb {
                acc.0.iter_mut().for_each(|w| *w = u64::MAX);
                for r in 0..=n {
                    if prefix[r] > b as i64 {
                        break;
                    }
                    let nb2 = b - prefix[r] as usize;
                    let base = ((k + 1) * nb + nb2) * words;
                    uni.0.copy_from_slice(&data[base..base + words]);
                    let mut shifted = uni.clone();
                    for _ in 0..(n - r) {
                        shifted.shift_left(c.value, hi);
                        for (u, s) in uni.0.iter_mut().zip(&shifted.0) {
                            *u |= *s;
                        }
                    }
                    for (a, u) in acc.0.iter_mut().zip(&uni.0) {
                        *a &= *u;
                    }
                }
                let base = (k * nb + b) * words;
                data[base..base + words].copy_from_slice(&acc.0);
            }
        }
        RobustTable {
            words,
            budget: nb,
            data,
        }
    }

    fn get(&self, k: usize, b: i64) -> &[u64] {
        let b = (b.max(0) as usize).min(self.budget - 1);
        let base = (k * self.budget + b) * self.words;
        &self.data[base..base + self.words]
    }

    /// Is some s in `sums` with t = target - s in H(k, b), for a target in [lo, hi]?
    fn covers(&self, k: usize, b: i64, sums: &Sums, lo: usize, hi: usize) -> bool {
        let h = self.get(k, b);
        let bit = |v: &[u64], i: usize| v[i / 64] >> (i % 64) & 1 == 1;
        for target in lo..=hi {
            for s in 0..=target {
                if sums.get(s) && bit(h, target - s) {
                    return true;
                }
            }
        }
        false
    }

    pub fn contains(&self, k: usize, b: i64, t: usize) -> bool {
        let h = self.get(k, b);
        h[t / 64] >> (t % 64) & 1 == 1
    }
}

pub struct Search<'a> {
    classes: &'a [Class],
    lo: usize,
    hi: usize,
    pub stats: Stats,
    pub node_limit: u64,
    /// Safe states: (depth, reachable sums) -> largest budget proven safe.
    cache: HashMap<(usize, Sums), i64>,
    choice: Vec<usize>,
    /// cover[k][t - k - 1]: copies of classes k..t sorted by cost per value,
    /// as cumulative (mass, cost), for fractional deletion bounds.
    cover: Vec<Vec<Cumulative>>,
    /// mass[k] = total value of classes k.. (all copies).
    mass: Vec<f64>,
    pub table: Option<RobustTable>,
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
    /// `classes` should be sorted by ascending value.
    pub fn new(classes: &'a [Class], lo: usize, hi: usize) -> Self {
        let m = classes.len();
        let mut cover = Vec::with_capacity(m);
        for k in 0..m {
            let mut copies: Vec<(f64, f64)> = Vec::new();
            let mut row = Vec::with_capacity(m - k);
            for c in &classes[k..] {
                copies.extend(c.costs.iter().map(|&x| (x as f64, c.value as f64)));
                row.push(Cumulative::build(&mut copies.clone()));
            }
            cover.push(row);
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
            cache: HashMap::new(),
            choice: vec![0; classes.len()],
            cover,
            mass,
            table: None,
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
    /// makes [a, b + v] reachable. Taking later classes in ascending order,
    /// the window keeps growing unless it *stalls*: reaches a class whose
    /// value exceeds the current width. Stalling at class u needs the leader
    /// to delete at least D_u = mass(k..u) - v_u + w + 1 of the mass before
    /// u; if even the fractional (cheapest per unit) cost of that exceeds the
    /// budget for every u, no stall is possible and the window reaches
    /// b + (future mass - most mass the leader can delete).
    fn window_safe(&self, k: usize, budget: i64, sums: &Sums) -> bool {
        let m = self.classes.len();
        let vk = self.classes[k].value;
        let b_budget = budget as f64;
        let future = self.mass[k] - self.max_del[k][budget.max(0) as usize] as f64;
        // Maximal runs [a, b] of width >= v_k, scanned from the top down.
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
            for u in k..m {
                let before = self.mass[k] - self.mass[u];
                let d = before - self.classes[u].value as f64 + w + 1.0;
                if d <= 0.0 {
                    stallable = true;
                    break;
                }
                if u == k {
                    continue; // nothing before class k to delete
                }
                if self.cover[k][u - k - 1].min_cost(d) <= b_budget {
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
        if let Some(t) = &self.table {
            if t.covers(k, budget, sums, self.lo, self.hi) {
                self.stats.safe_online += 1;
                return Some(true);
            }
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

    fn remember(&mut self, k: usize, sums: &Sums, budget: i64) {
        let e = self.cache.entry((k, sums.clone())).or_insert(i64::MIN);
        *e = (*e).max(budget);
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
        for round in 0..3000 {
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
            let lo = hi.saturating_sub(rnd(3) as usize);
            let budget = rnd(40) as i64;
            let expect = brute(&classes, lo, hi, budget);
            let mut s = Search::new(&classes, lo, hi);
            if round % 2 == 1 {
                s.table = Some(RobustTable::build(&classes, hi, budget));
            }
            let got = s.run(budget);
            windows += s.stats.safe_window;
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
    }
}
