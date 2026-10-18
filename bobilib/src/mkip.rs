//! Packing interdiction with any number of leader budgets and follower
//! knapsacks (MKIP): the follower oracle, a greedy leader, and a
//! counterexample-guided cut loop (interdiction cuts in the leader's space,
//! as in Fischetti, Ljubic, Monaci and Sinnl 2019) with a HiGHS master.

use crate::kp;
use crate::milp::{Milp, MilpStatus};
use crate::structure::PackingInterdiction;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct FollowerResult {
    pub value: i64,
    pub y: Vec<bool>,
    /// True when `value` is proven optimal for this leader choice.
    pub proven: bool,
    /// Proven upper bound on the follower's optimum.
    pub bound: i64,
}

#[derive(Clone, Debug)]
pub struct CegarResult {
    pub best_x: Vec<bool>,
    pub best_y: Vec<bool>,
    /// Best verified bilevel value (follower optimum proven), if any.
    pub ub: Option<i64>,
    /// Proven lower bound on the bilevel optimum.
    pub lb: i64,
    pub iterations: usize,
    pub proven: bool,
    pub seconds: f64,
}

#[derive(Clone, Debug)]
pub struct LpFollowerBound {
    /// min over leader choices of the follower's LP value (as solved: the
    /// HiGHS dual bound, so a valid lower bound on that minimum).
    pub min_lp: f64,
    /// The follower's LP value at the leader choice found (an upper bound on OPT).
    pub lp_at_x: f64,
    /// The leader choice the MILP found and the follower's value there.
    pub x: Vec<bool>,
    pub y: Vec<bool>,
    pub value_at_x: i64,
    pub follower_proven: bool,
    pub follower_bound: i64,
    pub proven_min: bool,
}

impl PackingInterdiction {
    /// min over x of U(x), the follower's LP value, as one MILP. LP duality
    /// gives, with one multiplier per follower knapsack,
    /// U(x) = min_{l >= 0} sum_k l_k C_k + sum_j max(0, p_j - sum_k l_k w_kj) (1 - x_j),
    /// and with x binary the product linearises exactly:
    /// v_j >= p_j - sum_k l_k w_kj - p_j x_j, v_j >= 0.
    /// The follower's integer optimum is at least U(x) minus the profit of
    /// its (at most one per knapsack) fractional items, so
    /// OPT >= min_x U(x) - k * max_j p_j, and OPT <= U(x) at any x.
    pub fn lp_follower_bound(
        &self,
        time_limit: f64,
        follower_time: f64,
    ) -> Option<LpFollowerBound> {
        if self.foll_w.is_empty() {
            return None;
        }
        let n = self.n;
        let mut mp = Milp::new(false);
        let xs: Vec<usize> = (0..n).map(|_| mp.add_var(0.0, 0.0, 1.0, true)).collect();
        let pmax = self.profit.iter().copied().max().unwrap_or(0) as f64;
        let lams: Vec<usize> = self
            .foll_w
            .iter()
            .zip(&self.foll_cap)
            .map(|(w, &c)| {
                let wmin = (0..n)
                    .filter(|&j| w[j] > 0)
                    .map(|j| w[j])
                    .min()
                    .unwrap_or(1) as f64;
                mp.add_var(c as f64, 0.0, pmax / wmin, false)
            })
            .collect();
        for j in 0..n {
            if self.profit[j] == 0 {
                continue;
            }
            let v = mp.add_var(1.0, 0.0, f64::INFINITY, false);
            // v_j + sum_k l_k w_kj + p_j x_j >= p_j
            let mut coefs = vec![(v, 1.0), (xs[j], self.profit[j] as f64)];
            for (k, w) in self.foll_w.iter().enumerate() {
                if w[j] != 0 {
                    coefs.push((lams[k], w[j] as f64));
                }
            }
            mp.add_row(coefs, self.profit[j] as f64, f64::INFINITY);
        }
        for (lw, &lc) in self.lead_w.iter().zip(&self.lead_cap) {
            let coefs = (0..n)
                .filter(|&j| lw[j] != 0)
                .map(|j| (xs[j], lw[j] as f64))
                .collect();
            mp.add_row(coefs, f64::NEG_INFINITY, lc as f64);
        }
        let r = mp.solve(time_limit, 1);
        if !matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
            return None;
        }
        let x: Vec<bool> = xs.iter().map(|&v| r.x[v] > 0.5).collect();
        let f = self.follower(&x, follower_time);
        Some(LpFollowerBound {
            min_lp: if r.bound.is_finite() { r.bound } else { r.obj },
            lp_at_x: r.obj,
            x,
            value_at_x: f.value,
            follower_proven: f.proven,
            follower_bound: f.bound,
            y: f.y,
            proven_min: r.status == MilpStatus::Optimal,
        })
    }

    fn total_profit(&self) -> i64 {
        self.profit.iter().sum()
    }

    /// The follower's best response to `x`.
    pub fn follower(&self, x: &[bool], time_limit: f64) -> FollowerResult {
        let n = self.n;
        let items: Vec<usize> = (0..n).filter(|&j| !x[j] && self.profit[j] > 0).collect();
        let mut y = vec![false; n];
        if self.foll_w.is_empty() {
            for &j in &items {
                y[j] = true;
            }
            let v = items.iter().map(|&j| self.profit[j]).sum();
            return FollowerResult {
                value: v,
                y,
                proven: true,
                bound: v,
            };
        }
        if self.foll_w.len() == 1 {
            let r = kp::solve(&self.profit, &self.foll_w[0], self.foll_cap[0], &items);
            for &j in &r.take {
                y[j] = true;
            }
            return FollowerResult {
                value: r.value,
                y,
                proven: !r.truncated,
                bound: if r.truncated { i64::MAX } else { r.value },
            };
        }
        // Items that fit every knapsack; zero-weight items are free.
        let fits: Vec<usize> = items
            .iter()
            .copied()
            .filter(|&j| {
                self.foll_w
                    .iter()
                    .zip(&self.foll_cap)
                    .all(|(w, &c)| w[j] <= c)
            })
            .collect();
        let mut mp = Milp::new(true);
        let vars: Vec<usize> = fits
            .iter()
            .map(|&j| mp.add_var(self.profit[j] as f64, 0.0, 1.0, true))
            .collect();
        for (w, &c) in self.foll_w.iter().zip(&self.foll_cap) {
            let coefs: Vec<(usize, f64)> = fits
                .iter()
                .zip(&vars)
                .filter(|(&j, _)| w[j] != 0)
                .map(|(&j, &v)| (v, w[j] as f64))
                .collect();
            mp.add_row(coefs, f64::NEG_INFINITY, c as f64);
        }
        let r = mp.solve(time_limit, 1);
        if !matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
            return FollowerResult {
                value: 0,
                y,
                proven: false,
                bound: i64::MAX,
            };
        }
        for (&j, &v) in fits.iter().zip(&vars) {
            y[j] = r.x[v] > 0.5;
        }
        // Recompute exactly and make sure the point is feasible.
        assert!(
            self.follower_feasible(x, &y),
            "MKP solver returned an infeasible point"
        );
        let value: i64 = (0..n).filter(|&j| y[j]).map(|j| self.profit[j]).sum();
        let bound = if r.bound.is_finite() {
            (r.bound + 1e-6).floor() as i64
        } else {
            i64::MAX
        };
        FollowerResult {
            value,
            y,
            proven: r.status == MilpStatus::Optimal || bound <= value,
            bound: bound.max(value),
        }
    }

    /// First-improvement local search on the leader's choice: add an item
    /// the follower uses, or swap a blocked item for one the follower uses,
    /// keeping every budget satisfied. Needs a fast (single-knapsack)
    /// follower to be practical.
    pub fn local_search(
        &self,
        x: &[bool],
        time_limit: f64,
        follower_time: f64,
    ) -> (Vec<bool>, FollowerResult) {
        let start = Instant::now();
        let n = self.n;
        let mut x = x.to_vec();
        let mut cur = self.follower(&x, follower_time);
        let used = |x: &[bool]| -> Vec<i64> {
            self.lead_w
                .iter()
                .map(|w| (0..n).filter(|&j| x[j]).map(|j| w[j]).sum())
                .collect()
        };
        'outer: loop {
            if start.elapsed().as_secs_f64() > time_limit || !cur.proven {
                break;
            }
            let u = used(&x);
            let fits = |add: usize, drop: Option<usize>| {
                self.lead_w
                    .iter()
                    .zip(&self.lead_cap)
                    .zip(&u)
                    .all(|((w, &c), &uu)| uu + w[add] - drop.map_or(0, |d| w[d]) <= c)
            };
            // Items the follower takes, most profitable first.
            let mut targets: Vec<usize> = (0..n).filter(|&j| cur.y[j]).collect();
            targets.sort_by_key(|&j| std::cmp::Reverse(self.profit[j]));
            let mut blocked: Vec<usize> = (0..n).filter(|&j| x[j]).collect();
            blocked.sort_by_key(|&j| self.profit[j]);
            for &a in &targets {
                if fits(a, None) {
                    x[a] = true;
                    let f = self.follower(&x, follower_time);
                    if f.proven && f.value < cur.value {
                        cur = f;
                        continue 'outer;
                    }
                    x[a] = false;
                }
            }
            for &a in &targets {
                for &d in &blocked {
                    if start.elapsed().as_secs_f64() > time_limit {
                        break 'outer;
                    }
                    if !fits(a, Some(d)) {
                        continue;
                    }
                    x[a] = true;
                    x[d] = false;
                    let f = self.follower(&x, follower_time);
                    if f.proven && f.value < cur.value {
                        cur = f;
                        continue 'outer;
                    }
                    x[a] = false;
                    x[d] = true;
                }
            }
            break;
        }
        (x, cur)
    }

    /// The leader blocks as much profit as its budgets allow.
    pub fn greedy_leader(&self, time_limit: f64) -> Vec<bool> {
        let n = self.n;
        let mut x = vec![false; n];
        let free: Vec<usize> = (0..n)
            .filter(|&j| self.lead_w.iter().all(|w| w[j] == 0))
            .collect();
        for &j in &free {
            x[j] = true;
        }
        let rest: Vec<usize> = (0..n).filter(|&j| !x[j]).collect();
        if self.lead_w.len() == 1 {
            let r = kp::solve(&self.profit, &self.lead_w[0], self.lead_cap[0], &rest);
            for j in r.take {
                x[j] = true;
            }
            return x;
        }
        let mut mp = Milp::new(true);
        let vars: Vec<usize> = rest
            .iter()
            .map(|&j| mp.add_var(self.profit[j] as f64, 0.0, 1.0, true))
            .collect();
        for (w, &c) in self.lead_w.iter().zip(&self.lead_cap) {
            let coefs = rest
                .iter()
                .zip(&vars)
                .filter(|(&j, _)| w[j] != 0)
                .map(|(&j, &v)| (v, w[j] as f64))
                .collect();
            mp.add_row(coefs, f64::NEG_INFINITY, c as f64);
        }
        let r = mp.solve(time_limit, 1);
        if matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
            for (&j, &v) in rest.iter().zip(&vars) {
                x[j] = r.x[v] > 0.5;
            }
        }
        assert!(self.leader_feasible(&x));
        x
    }

    /// Cut loop: the master picks the leader choice minimising the best
    /// follower profit over a pool of follower packings (a valid lower
    /// bound); the follower's real response at that choice gives an upper
    /// bound and a new packing for the pool.
    pub fn cegar(&self, time_limit: f64, follower_time: f64, log: bool) -> CegarResult {
        let start = Instant::now();
        let n = self.n;
        let left = |s: &Instant| (time_limit - s.elapsed().as_secs_f64()).max(0.0);
        let mut pool: Vec<Vec<usize>> = Vec::new();
        let mut best: Option<(i64, Vec<bool>, Vec<bool>)> = None;
        let mut lb = 0i64;
        let consider =
            |x: &[bool], fr: &FollowerResult, best: &mut Option<(i64, Vec<bool>, Vec<bool>)>| {
                if fr.proven && best.as_ref().is_none_or(|b| fr.value < b.0) {
                    *best = Some((fr.value, x.to_vec(), fr.y.clone()));
                }
            };
        let x0 = self.greedy_leader(follower_time.min(left(&start)));
        let f0 = self.follower(&x0, follower_time.min(left(&start)));
        consider(&x0, &f0, &mut best);
        pool.push((0..n).filter(|&j| f0.y[j]).collect());
        let mut iterations = 0;
        let total = self.total_profit() as f64;
        loop {
            iterations += 1;
            if let Some((ub, _, _)) = &best {
                if lb >= *ub {
                    break;
                }
            }
            if left(&start) <= 0.0 {
                break;
            }
            let mut mp = Milp::new(false);
            let xs: Vec<usize> = (0..n).map(|_| mp.add_var(0.0, 0.0, 1.0, true)).collect();
            let theta = mp.add_var(1.0, 0.0, total, false);
            for (w, &c) in self.lead_w.iter().zip(&self.lead_cap) {
                let coefs = (0..n)
                    .filter(|&j| w[j] != 0)
                    .map(|j| (xs[j], w[j] as f64))
                    .collect();
                mp.add_row(coefs, f64::NEG_INFINITY, c as f64);
            }
            for y in &pool {
                let py: i64 = y.iter().map(|&j| self.profit[j]).sum();
                let mut coefs: Vec<(usize, f64)> =
                    y.iter().map(|&j| (xs[j], self.profit[j] as f64)).collect();
                coefs.push((theta, 1.0));
                mp.add_row(coefs, py as f64, f64::INFINITY);
            }
            let r = mp.solve(left(&start), 1);
            if r.bound.is_finite() {
                lb = lb.max((r.bound - 1e-6).ceil() as i64);
            }
            if !matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
                break;
            }
            if r.status == MilpStatus::Optimal {
                lb = lb.max((r.obj - 1e-6).ceil() as i64);
            }
            let x: Vec<bool> = xs.iter().map(|&v| r.x[v] > 0.5).collect();
            if !self.leader_feasible(&x) {
                break;
            }
            let fr = self.follower(&x, follower_time.min(left(&start)));
            consider(&x, &fr, &mut best);
            let y: Vec<usize> = (0..n).filter(|&j| fr.y[j]).collect();
            if log {
                eprintln!(
                    "iter {iterations}: master {:.1} (lb {lb}) follower {} proven {} ub {:?} pool {} {:.1}s",
                    r.obj,
                    fr.value,
                    fr.proven,
                    best.as_ref().map(|b| b.0),
                    pool.len(),
                    start.elapsed().as_secs_f64()
                );
            }
            if pool.contains(&y) {
                // The master already accounts for this packing: with an
                // optimal master and a proven follower the loop has met.
                if !(fr.proven && r.status == MilpStatus::Optimal) {
                    break;
                }
            }
            pool.push(y);
        }
        let (ub, bx, by) = match best {
            Some((v, x, y)) => (Some(v), x, y),
            None => (None, x0, f0.y),
        };
        CegarResult {
            proven: ub.is_some_and(|u| lb >= u),
            best_x: bx,
            best_y: by,
            ub,
            lb,
            iterations,
            seconds: start.elapsed().as_secs_f64(),
        }
    }
}

/// Weninger and Fukasawa's online-relaxation lower bound, generalised to
/// several leader budgets and large integer data by two relaxations that
/// keep it a valid lower bound:
///
/// * the leader's rows are merged into one surrogate row with multipliers
///   `mu` (any leader choice feasible for all rows is feasible for the
///   merged row), with merged costs rounded down and the merged capacity
///   rounded up after scaling by `su`;
/// * the follower's weights are rounded up and its capacity down after
///   scaling by `sl`, which only shrinks what the follower can pack.
///
/// Items are processed in decreasing follower efficiency. Returns the root
/// value omega(1, B', C').
pub fn omega_root(pi: &PackingInterdiction, mu: &[f64], su: f64, sl: f64) -> i64 {
    assert_eq!(pi.foll_w.len(), 1, "single follower knapsack only");
    let n = pi.n;
    let w = &pi.foll_w[0];
    let mut order: Vec<usize> = (0..n).filter(|&j| pi.profit[j] > 0).collect();
    order.sort_by(|&a, &b| {
        crate::kp::eff_cmp(pi.profit[a], w[a], pi.profit[b], w[b])
            .then(pi.profit[b].cmp(&pi.profit[a]))
    });
    let agg: Vec<f64> = (0..n)
        .map(|j| {
            pi.lead_w
                .iter()
                .zip(mu)
                .map(|(r, &m)| m * r[j] as f64)
                .sum()
        })
        .collect();
    let cap_agg: f64 = pi
        .lead_cap
        .iter()
        .zip(mu)
        .map(|(&c, &m)| m * c as f64)
        .sum();
    let bu = (cap_agg * su).ceil() as usize;
    let cl = ((pi.foll_cap[0] as f64) * sl).floor() as usize;
    let wu: Vec<usize> = (0..n).map(|j| (agg[j] * su).floor() as usize).collect();
    let wl: Vec<usize> = (0..n)
        .map(|j| ((w[j] as f64) * sl).ceil() as usize)
        .collect();
    let (nb, nc) = (bu + 1, cl + 1);
    // next[b * nc + c] = omega(i + 1, b, c); start with omega(n + 1, ., .) = 0.
    let mut next = vec![0i64; nb * nc];
    let mut cur = vec![0i64; nb * nc];
    for &j in order.iter().rev() {
        let (u, l, p) = (wu[j], wl[j], pi.profit[j]);
        for b in 0..nb {
            for c in 0..nc {
                let skip = next[b * nc + c];
                let follower = if l <= c {
                    skip.max(next[b * nc + c - l] + p)
                } else {
                    skip
                };
                cur[b * nc + c] = if u <= b {
                    follower.min(next[(b - u) * nc + c])
                } else {
                    follower
                };
            }
        }
        std::mem::swap(&mut cur, &mut next);
    }
    next[bu * nc + cl]
}
