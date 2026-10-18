//! The bilevel knapsack problem with interdiction constraints (KIP):
//! one leader budget row and one follower knapsack row.

use crate::kp;
use crate::robust_ss::{self, Class};
use crate::structure::PackingInterdiction;

#[derive(Clone, Debug)]
pub struct Kip {
    pub n: usize,
    pub p: Vec<i64>,
    /// Follower weights.
    pub wl: Vec<i64>,
    /// Leader (interdiction) costs.
    pub wu: Vec<i64>,
    pub cl: i64,
    pub cu: i64,
}

impl Kip {
    pub fn from_packing(pi: &PackingInterdiction) -> Option<Kip> {
        if pi.lead_w.len() != 1 || pi.foll_w.len() != 1 {
            return None;
        }
        Some(Kip {
            n: pi.n,
            p: pi.profit.clone(),
            wl: pi.foll_w[0].clone(),
            wu: pi.lead_w[0].clone(),
            cl: pi.foll_cap[0],
            cu: pi.lead_cap[0],
        })
    }

    /// Items the leader blocks for free; blocking them never hurts the leader.
    pub fn free_items(&self) -> Vec<usize> {
        (0..self.n).filter(|&j| self.wu[j] == 0).collect()
    }

    /// The follower's best response to `x`.
    pub fn follower(&self, x: &[bool]) -> kp::KpResult {
        let items: Vec<usize> = (0..self.n).filter(|&j| !x[j]).collect();
        kp::solve(&self.p, &self.wl, self.cl, &items)
    }

    /// Weninger and Fukasawa's greedy heuristic: the leader blocks the most
    /// profit its budget allows, then the follower responds.
    pub fn greedy(&self) -> (Vec<bool>, i64) {
        let all: Vec<usize> = (0..self.n).collect();
        let r = kp::solve(&self.p, &self.wu, self.cu, &all);
        let mut x = vec![false; self.n];
        for j in r.take {
            x[j] = true;
        }
        for j in self.free_items() {
            x[j] = true;
        }
        let v = self.follower(&x).value;
        (x, v)
    }

    pub fn is_subset_sum(&self) -> bool {
        (0..self.n).all(|j| self.p[j] == self.wl[j])
    }

    /// For subset-sum instances: group the items the leader does not get for
    /// free by follower weight, dividing all weights by their gcd `g`.
    /// Returns the classes (ascending value) and `g`.
    pub fn subset_sum_classes(&self) -> (Vec<Class>, i64) {
        let live: Vec<usize> = (0..self.n)
            .filter(|&j| self.wu[j] > 0 && self.wl[j] > 0 && self.wl[j] <= self.cl)
            .collect();
        let g = live.iter().fold(0i64, |g, &j| gcd(g, self.wl[j])).max(1);
        let mut by: std::collections::BTreeMap<i64, Vec<i64>> = Default::default();
        for &j in &live {
            by.entry(self.wl[j] / g).or_default().push(self.wu[j]);
        }
        let classes = by
            .into_iter()
            .map(|(v, mut costs)| {
                costs.sort();
                Class {
                    value: v as usize,
                    costs,
                }
            })
            .collect();
        (classes, g)
    }

    /// Decide whether the leader can hold a subset-sum follower below `lo`.
    /// `Safe` means the follower always gets at least `lo`.
    pub fn subset_sum_threshold(
        &self,
        lo: i64,
        node_limit: u64,
        cfg: &SsConfig,
        order: SsOrder,
        max_copies: Option<usize>,
    ) -> (robust_ss::Outcome, robust_ss::Stats, Vec<Class>, i64) {
        let online = cfg.online;
        assert!(self.is_subset_sum());
        let (mut classes, g) = self.subset_sum_classes();
        if let Some(k) = max_copies {
            robust_ss::restrict_copies(&mut classes, k);
        }
        order_classes(&mut classes, order);
        let hi = (self.cl / g) as usize;
        let lo_s = ((lo + g - 1) / g) as usize;
        let mut s = robust_ss::Search::new(&classes, lo_s.min(hi + 1), hi);
        s.node_limit = node_limit;
        s.cache_bytes = cfg.cache_mb << 20;
        s.progress_every = cfg.progress_every;
        s.node_order = cfg.node_order.clone();
        if online && lo_s <= hi {
            let t = std::time::Instant::now();
            s.tables = robust_ss::build_tables(&classes, hi, self.cu, &cfg.tables);
            // The narrowest table of each kind that guarantees the target band
            // at the root with the full budget.
            let mut root: Vec<String> = Vec::new();
            for tb in &s.tables {
                if root.iter().any(|r| r.starts_with(&tb.kind)) {
                    continue;
                }
                if (lo_s..=hi).any(|v| tb.contains(0, self.cu, v)) {
                    root.push(format!("{}: width {}", tb.kind, tb.width));
                }
            }
            eprintln!(
                "{} online tables built in {:.2}s; target [{lo_s}, {hi}]; root reached by [{}]",
                s.tables.len(),
                t.elapsed().as_secs_f64(),
                root.join(", ")
            );
        }
        let out = if lo_s > hi {
            robust_ss::Outcome::Safe
        } else {
            s.run(self.cu)
        };
        (out, s.stats.clone(), classes, g)
    }
}

/// Result of solving a subset-sum KIP exactly.
#[derive(Clone, Debug)]
pub struct SsSolution {
    pub value: i64,
    pub x: Vec<bool>,
    pub proven: bool,
    pub nodes: u64,
    pub rounds: usize,
}

impl Kip {
    /// Turn per-class deletion counts back into a leader solution: delete the
    /// cheapest copies of each class, plus every free item.
    pub fn x_from_deletions(&self, classes: &[Class], g: i64, del: &[usize]) -> Vec<bool> {
        let mut x = vec![false; self.n];
        for j in self.free_items() {
            x[j] = true;
        }
        for (c, &r) in classes.iter().zip(del) {
            let mut cands: Vec<usize> = (0..self.n)
                .filter(|&j| !x[j] && self.wl[j] == c.value as i64 * g && self.wu[j] > 0)
                .collect();
            cands.sort_by_key(|&j| (self.wu[j], j));
            for &j in cands.iter().take(r) {
                x[j] = true;
            }
        }
        x
    }

    /// Exact solve of a subset-sum KIP: prove the incumbent or find a leader
    /// choice that beats it, until the threshold search proves optimality.
    pub fn solve_subset_sum(
        &self,
        node_limit: u64,
        cfg: &SsConfig,
        order: SsOrder,
        max_copies: Option<usize>,
    ) -> SsSolution {
        let (mut x, mut ub) = self.greedy();
        let mut nodes = 0;
        let mut rounds = 0;
        // A restricted follower (at most k copies per class) can only prove
        // safety; try it first since its search is much smaller.
        if let Some(k) = max_copies {
            let (out, st, _, _) = self.subset_sum_threshold(ub, node_limit, cfg, order, Some(k));
            nodes += st.nodes;
            rounds += 1;
            if matches!(out, robust_ss::Outcome::Safe) {
                return SsSolution {
                    value: ub,
                    x,
                    proven: true,
                    nodes,
                    rounds,
                };
            }
        }
        loop {
            rounds += 1;
            let (out, st, classes, g) =
                self.subset_sum_threshold(ub, node_limit, cfg, order, None);
            nodes += st.nodes;
            match out {
                robust_ss::Outcome::Safe => {
                    return SsSolution {
                        value: ub,
                        x,
                        proven: true,
                        nodes,
                        rounds,
                    }
                }
                robust_ss::Outcome::Unknown => {
                    return SsSolution {
                        value: ub,
                        x,
                        proven: false,
                        nodes,
                        rounds,
                    }
                }
                robust_ss::Outcome::Blocking(del) => {
                    let nx = self.x_from_deletions(&classes, g, &del);
                    let used: i64 = (0..self.n).filter(|&j| nx[j]).map(|j| self.wu[j]).sum();
                    assert!(used <= self.cu, "blocking choice exceeds the budget");
                    let v = self.follower(&nx).value;
                    assert!(v < ub, "blocking choice does not improve: {v} >= {ub}");
                    ub = v;
                    x = nx;
                }
            }
        }
    }
}

/// Options of the subset-sum search.
#[derive(Clone, Debug)]
pub struct SsConfig {
    /// Build and use the robust online tables.
    pub online: bool,
    pub tables: robust_ss::TableConfig,
    /// Memory allowance of the safe-state cache, in MiB.
    pub cache_mb: usize,
    /// Progress line every this many nodes (0: none).
    pub progress_every: u64,
    /// See `robust_ss::Search::node_order`.
    pub node_order: Option<String>,
}

impl Default for SsConfig {
    fn default() -> Self {
        SsConfig {
            online: true,
            tables: robust_ss::TableConfig::default(),
            cache_mb: 1024,
            progress_every: 0,
            node_order: Some("ratioasc".into()),
        }
    }
}

/// Order in which the subset-sum search decides classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum SsOrder {
    /// Smallest follower weight first.
    Ascending,
    /// Largest follower weight first.
    Descending,
    /// Classes that cost the leader most to delete entirely first.
    Cost,
}

pub fn order_classes(classes: &mut [Class], order: SsOrder) {
    match order {
        SsOrder::Ascending => classes.sort_by_key(|c| c.value),
        SsOrder::Descending => classes.sort_by_key(|c| std::cmp::Reverse(c.value)),
        SsOrder::Cost => {
            classes.sort_by_key(|c| (std::cmp::Reverse(c.costs.iter().sum::<i64>()), c.value))
        }
    }
}

pub fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}
