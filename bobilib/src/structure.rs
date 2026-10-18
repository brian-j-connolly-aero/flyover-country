//! Recognise problem structure that specialised algorithms can exploit.
//!
//! The main one is *packing interdiction*, which covers BOBILib's knapsack
//! (KIP) and multidimensional knapsack (MKIP) interdiction sets:
//!
//! ```text
//! min_x  max_y  sum_j p_j y_j
//!   leader:   sum_j a_ij x_j <= A_i          (one or more budgets)
//!   follower: sum_j w_kj y_j <= C_k          (one or more knapsacks)
//!             x_j + y_j <= 1                 (an interdicted item is gone)
//!   x, y binary, all of p, a, w non-negative integers.
//! ```
//!
//! The game is zero-sum (the leader's objective is the negated follower
//! objective), so the optimistic and pessimistic versions coincide.

use crate::model::Bilevel;

#[derive(Clone, Debug)]
pub struct PackingInterdiction {
    pub name: String,
    pub n: usize,
    /// Follower profit of item j.
    pub profit: Vec<i64>,
    /// Leader budget rows: `lead_w[i][j]` is the cost of interdicting item j.
    pub lead_w: Vec<Vec<i64>>,
    pub lead_cap: Vec<i64>,
    /// Follower knapsack rows.
    pub foll_w: Vec<Vec<i64>>,
    pub foll_cap: Vec<i64>,
    /// Model indices of x_j and y_j.
    pub x_var: Vec<usize>,
    pub y_var: Vec<usize>,
    /// Upper-level objective = `scale * sum p_j y_j + offset`.
    pub scale: f64,
    pub offset: f64,
}

fn as_int(v: f64) -> Option<i64> {
    (v.is_finite() && (v - v.round()).abs() < 1e-9 && v.abs() < 9e15).then(|| v.round() as i64)
}

fn is_binary(m: &Bilevel, j: usize) -> bool {
    let v = &m.vars[j];
    v.integer && v.lb == 0.0 && v.ub == 1.0
}

/// Try to read `m` as a packing interdiction game. On failure the reason is
/// returned, which is handy when surveying the library.
pub fn packing_interdiction(m: &Bilevel) -> Result<PackingInterdiction, String> {
    let ys: Vec<usize> = (0..m.vars.len()).filter(|&j| m.vars[j].lower).collect();
    let xs: Vec<usize> = (0..m.vars.len()).filter(|&j| !m.vars[j].lower).collect();
    if ys.len() != xs.len() {
        return Err(format!(
            "{} leader vs {} follower variables",
            xs.len(),
            ys.len()
        ));
    }
    // Leader variables must be binary. A follower variable may also be an
    // unbounded non-negative integer: its interdiction row x_j + y_j <= 1
    // (checked below) caps it at 1.
    for &j in &xs {
        if !is_binary(m, j) {
            return Err(format!("variable {} is not binary", m.vars[j].name));
        }
    }
    for &j in &ys {
        let v = &m.vars[j];
        if !(v.integer && v.lb == 0.0 && v.ub >= 1.0) {
            return Err(format!("variable {} is not binary", v.name));
        }
    }
    let n = ys.len();
    let mut item_of_y = vec![usize::MAX; m.vars.len()];
    for (k, &j) in ys.iter().enumerate() {
        item_of_y[j] = k;
    }
    // Interdiction rows pair each x with one y.
    let mut x_of_item = vec![usize::MAX; n];
    let mut is_interdiction = vec![false; m.rows.len()];
    for (i, r) in m.rows.iter().enumerate() {
        if !r.lower || r.coefs.len() != 2 || r.hi != 1.0 || r.lo > -1.0 {
            continue;
        }
        let (a, b) = (r.coefs[0], r.coefs[1]);
        let (xv, yv) = match (m.vars[a.0].lower, m.vars[b.0].lower) {
            (false, true) => (a, b),
            (true, false) => (b, a),
            _ => continue,
        };
        if xv.1 != 1.0 || yv.1 != 1.0 {
            continue;
        }
        let k = item_of_y[yv.0];
        if x_of_item[k] != usize::MAX {
            return Err(format!("item {} interdicted twice", m.vars[yv.0].name));
        }
        x_of_item[k] = xv.0;
        is_interdiction[i] = true;
    }
    if let Some(k) = x_of_item.iter().position(|&x| x == usize::MAX) {
        return Err(format!(
            "follower variable {} has no interdiction row",
            m.vars[ys[k]].name
        ));
    }
    let mut item_of_x = vec![usize::MAX; m.vars.len()];
    for (k, &x) in x_of_item.iter().enumerate() {
        if item_of_x[x] != usize::MAX {
            return Err(format!(
                "leader variable {} interdicts two items",
                m.vars[x].name
            ));
        }
        item_of_x[x] = k;
    }

    // Objectives: follower maximises p y; leader minimises the same p y.
    let mut profit = vec![0i64; n];
    for (k, &j) in ys.iter().enumerate() {
        profit[k] = as_int(-m.vars[j].ll_obj).ok_or("non-integer follower objective")?;
        if profit[k] < 0 {
            return Err(format!("negative profit on {}", m.vars[j].name));
        }
    }
    for &j in &xs {
        if m.vars[j].obj != 0.0 {
            return Err(format!(
                "leader variable {} has an objective cost",
                m.vars[j].name
            ));
        }
    }
    // Upper objective must be a positive multiple of p.
    let (mut scale, mut found) = (0.0, false);
    for (k, &j) in ys.iter().enumerate() {
        let c = m.vars[j].obj;
        if profit[k] == 0 {
            if c != 0.0 {
                return Err("upper objective not proportional to profits".into());
            }
            continue;
        }
        let s = c / profit[k] as f64;
        if !found {
            scale = s;
            found = true;
        } else if (s - scale).abs() > 1e-9 * scale.abs().max(1.0) {
            return Err("upper objective not proportional to profits".into());
        }
    }
    if found && scale <= 0.0 {
        return Err("leader does not oppose the follower".into());
    }

    let (mut lead_w, mut lead_cap, mut foll_w, mut foll_cap) = (vec![], vec![], vec![], vec![]);
    for (i, r) in m.rows.iter().enumerate() {
        if is_interdiction[i] {
            continue;
        }
        if r.lo.is_finite() && r.lo > 0.0 {
            return Err(format!("row {} has a positive lower bound", r.name));
        }
        let cap = as_int(r.hi).ok_or_else(|| format!("row {} has no integer capacity", r.name))?;
        let mut w = vec![0i64; n];
        let mut level = None;
        for &(j, c) in &r.coefs {
            let c =
                as_int(c).ok_or_else(|| format!("row {} has a fractional coefficient", r.name))?;
            if c < 0 {
                return Err(format!("row {} has a negative coefficient", r.name));
            }
            let (k, lower) = if m.vars[j].lower {
                (item_of_y[j], true)
            } else {
                (item_of_x[j], false)
            };
            if level.is_some_and(|l| l != lower) {
                return Err(format!(
                    "row {} mixes leader and follower variables",
                    r.name
                ));
            }
            level = Some(lower);
            w[k] += c;
        }
        match level {
            None => {
                if cap < 0 {
                    return Err(format!("row {} is infeasible", r.name));
                }
            }
            Some(false) if !r.lower => {
                lead_w.push(w);
                lead_cap.push(cap);
            }
            Some(true) if r.lower => {
                foll_w.push(w);
                foll_cap.push(cap);
            }
            Some(false) => {
                return Err(format!(
                    "follower row {} uses only leader variables",
                    r.name
                ))
            }
            Some(true) => {
                return Err(format!(
                    "leader row {} constrains the follower (coupling)",
                    r.name
                ))
            }
        }
    }
    Ok(PackingInterdiction {
        name: m.name.clone(),
        n,
        profit,
        lead_w,
        lead_cap,
        foll_w,
        foll_cap,
        x_var: x_of_item,
        y_var: ys,
        scale: if found { scale } else { 1.0 },
        offset: m.obj_offset,
    })
}

impl PackingInterdiction {
    /// Leader objective value of a follower profit.
    pub fn objective(&self, follower_profit: i64) -> f64 {
        self.scale * follower_profit as f64 + self.offset
    }

    pub fn leader_feasible(&self, x: &[bool]) -> bool {
        self.lead_w
            .iter()
            .zip(&self.lead_cap)
            .all(|(w, &c)| (0..self.n).filter(|&j| x[j]).map(|j| w[j]).sum::<i64>() <= c)
    }

    pub fn follower_feasible(&self, x: &[bool], y: &[bool]) -> bool {
        (0..self.n).all(|j| !(x[j] && y[j]))
            && self
                .foll_w
                .iter()
                .zip(&self.foll_cap)
                .all(|(w, &c)| (0..self.n).filter(|&j| y[j]).map(|j| w[j]).sum::<i64>() <= c)
    }

    /// Full variable assignment for the model, for writing solution files.
    pub fn assignment(&self, nvars: usize, x: &[bool], y: &[bool]) -> Vec<f64> {
        let mut val = vec![0.0; nvars];
        for j in 0..self.n {
            val[self.x_var[j]] = x[j] as u8 as f64;
            val[self.y_var[j]] = y[j] as u8 as f64;
        }
        val
    }
}
