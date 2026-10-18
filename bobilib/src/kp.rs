//! Exact 0-1 knapsack: the follower's best response once the leader has
//! interdicted. Depth-first branch and bound in efficiency order with the
//! Dantzig (LP) bound, falling back to dynamic programming over capacity when
//! the search grows large.

use std::cmp::Ordering;

/// Compare efficiencies p_a / w_a and p_b / w_b exactly (zero weight first).
pub fn eff_cmp(pa: i64, wa: i64, pb: i64, wb: i64) -> Ordering {
    // a before b when p_a / w_a > p_b / w_b.
    let l = pa as i128 * wb as i128;
    let r = pb as i128 * wa as i128;
    r.cmp(&l)
}

#[derive(Clone, Debug, Default)]
pub struct KpResult {
    pub value: i64,
    /// Chosen items, as indices into the input slices.
    pub take: Vec<usize>,
    /// True when the search hit a limit and `value` is only a lower bound.
    pub truncated: bool,
}

/// Solve max { p y : w y <= cap, y binary } over the items in `items`
/// (indices into `p` and `w`).
pub fn solve(p: &[i64], w: &[i64], cap: i64, items: &[usize]) -> KpResult {
    let mut free_value = 0i64;
    let mut take = Vec::new();
    let mut it: Vec<usize> = Vec::with_capacity(items.len());
    for &j in items {
        if p[j] <= 0 {
            continue;
        }
        if w[j] <= 0 {
            free_value += p[j];
            take.push(j);
        } else if w[j] <= cap {
            it.push(j);
        }
    }
    it.sort_by(|&a, &b| eff_cmp(p[a], w[a], p[b], w[b]).then(a.cmp(&b)));
    let mut r = bnb(p, w, cap, &it, 2_000_000);
    if r.truncated {
        let cells = (it.len() as u64).saturating_mul(cap.max(0) as u64 + 1);
        if cells <= 400_000_000 {
            r = dp_capacity(p, w, cap, &it);
        }
    }
    r.value += free_value;
    r.take.extend(take);
    r
}

/// Dantzig bound of items `it[k..]` with remaining capacity `cap` (exact
/// arithmetic, rounded down).
#[inline]
fn dantzig(p: &[i64], w: &[i64], it: &[usize], mut k: usize, mut cap: i64) -> i64 {
    let mut v = 0i64;
    while k < it.len() {
        let j = it[k];
        if w[j] <= cap {
            cap -= w[j];
            v += p[j];
        } else {
            return v + (p[j] as i128 * cap as i128 / w[j] as i128) as i64;
        }
        k += 1;
    }
    v
}

fn bnb(p: &[i64], w: &[i64], cap: i64, it: &[usize], node_limit: u64) -> KpResult {
    let n = it.len();
    // Greedy incumbent.
    let mut best = 0i64;
    let mut best_sel = vec![false; n];
    {
        let mut c = cap;
        for (k, &j) in it.iter().enumerate() {
            if w[j] <= c {
                c -= w[j];
                best += p[j];
                best_sel[k] = true;
            }
        }
    }
    let mut sel = vec![false; n];
    let mut nodes = 0u64;
    // Explicit-stack DFS: (depth, remaining capacity, value, phase).
    // phase 0 = try include, 1 = try exclude, 2 = done.
    let mut stack: Vec<(usize, i64, i64, u8)> = vec![(0, cap, 0, 0)];
    while let Some(top) = stack.last_mut() {
        let (k, c, v, phase) = *top;
        if k == n {
            if v > best {
                best = v;
                best_sel.copy_from_slice(&sel);
            }
            stack.pop();
            continue;
        }
        nodes += 1;
        if nodes > node_limit {
            return KpResult {
                value: best,
                take: pick(it, &best_sel),
                truncated: true,
            };
        }
        match phase {
            0 => {
                top.3 = 1;
                if v + dantzig(p, w, it, k, c) <= best {
                    stack.pop();
                    continue;
                }
                let j = it[k];
                if w[j] <= c {
                    sel[k] = true;
                    stack.push((k + 1, c - w[j], v + p[j], 0));
                }
            }
            1 => {
                top.3 = 2;
                sel[k] = false;
                if v + dantzig(p, w, it, k + 1, c) > best {
                    stack.push((k + 1, c, v, 0));
                }
            }
            _ => {
                stack.pop();
            }
        }
    }
    KpResult {
        value: best,
        take: pick(it, &best_sel),
        truncated: false,
    }
}

fn pick(it: &[usize], sel: &[bool]) -> Vec<usize> {
    it.iter()
        .zip(sel)
        .filter(|(_, &s)| s)
        .map(|(&j, _)| j)
        .collect()
}

/// Classic O(n * cap) dynamic program with a bit matrix for reconstruction.
pub fn dp_capacity(p: &[i64], w: &[i64], cap: i64, it: &[usize]) -> KpResult {
    let c = cap.max(0) as usize;
    let words = c / 64 + 1;
    let mut f = vec![0i64; c + 1];
    let mut keep = vec![0u64; words * it.len()];
    for (k, &j) in it.iter().enumerate() {
        let wj = w[j] as usize;
        if wj > c {
            continue;
        }
        let row = &mut keep[k * words..(k + 1) * words];
        for cc in (wj..=c).rev() {
            let cand = f[cc - wj] + p[j];
            if cand > f[cc] {
                f[cc] = cand;
                row[cc / 64] |= 1 << (cc % 64);
            }
        }
    }
    let mut take = Vec::new();
    let mut cc = c;
    for k in (0..it.len()).rev() {
        if keep[k * words + cc / 64] >> (cc % 64) & 1 == 1 {
            take.push(it[k]);
            cc -= w[it[k]] as usize;
        }
    }
    KpResult {
        value: f[c],
        take,
        truncated: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, m: u64) -> i64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) % m) as i64
        }
    }

    #[test]
    fn bnb_matches_dp() {
        let mut g = Lcg(7);
        for round in 0..400 {
            let n = 1 + g.next(40) as usize;
            let w: Vec<i64> = (0..n).map(|_| g.next(100)).collect();
            let p: Vec<i64> = (0..n)
                .map(|j| {
                    if round % 2 == 0 {
                        g.next(100)
                    } else {
                        w[j] + 10
                    }
                })
                .collect();
            let cap = g.next(w.iter().sum::<i64>() as u64 + 1);
            let items: Vec<usize> = (0..n).filter(|_| g.next(4) != 0).collect();
            let a = solve(&p, &w, cap, &items);
            let mut it: Vec<usize> = items
                .iter()
                .copied()
                .filter(|&j| w[j] > 0 && p[j] > 0)
                .collect();
            it.sort();
            let free: i64 = items.iter().filter(|&&j| w[j] == 0).map(|&j| p[j]).sum();
            let b = dp_capacity(&p, &w, cap, &it);
            assert_eq!(a.value, b.value + free, "round {round}");
            let wt: i64 = a.take.iter().map(|&j| w[j]).sum();
            let pt: i64 = a.take.iter().map(|&j| p[j]).sum();
            assert!(wt <= cap);
            assert_eq!(pt, a.value);
        }
    }
}
