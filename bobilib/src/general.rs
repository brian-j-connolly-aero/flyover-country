//! Feasible points for general MIBLPs (any variable types, including
//! continuous linking variables, which neither of BOBILib's two solvers
//! accepts).
//!
//! A point (x, y) is bilevel feasible when it satisfies every row and y is
//! an optimal follower response to x. Without coupling rows (leader rows
//! that involve follower variables), any x from the high point relaxation
//! (HPR, the single-level problem with every row) gives one: the follower
//! problem at that x is feasible (the HPR's own y shows it), so solve it to
//! optimality and, among its optima, pick the one best for the leader (the
//! optimistic convention BOBILib uses). With coupling rows the last step may
//! fail for the HPR's x; we report that rather than guess.

use crate::milp::{Milp, MilpStatus};
use crate::model::Bilevel;

#[derive(Clone, Debug)]
pub struct GeneralPoint {
    pub values: Vec<f64>,
    pub upper_obj: f64,
    pub lower_obj: f64,
}

#[derive(Clone, Debug)]
pub enum GeneralOutcome {
    Point(GeneralPoint),
    /// The HPR is infeasible, so the bilevel problem is too.
    Infeasible,
    /// Some stage did not finish; the string says which.
    Failed(String),
}

fn integer_round(m: &Bilevel, v: &mut [f64]) {
    for (var, x) in m.vars.iter().zip(v.iter_mut()) {
        if var.integer {
            *x = x.round();
        }
    }
}

/// The HPR: every variable and row, the leader's objective.
pub fn hpr(m: &Bilevel) -> Milp {
    let mut mp = Milp::new(false);
    for v in &m.vars {
        mp.add_var(v.obj, v.lb, v.ub, v.integer);
    }
    for r in &m.rows {
        mp.add_row(r.coefs.clone(), r.lo, r.hi);
    }
    mp
}

/// The follower's problem with the leader's variables fixed to `val`.
/// Returns the MILP over follower variables (indexed as in `idx`) and the
/// map from model variables to MILP columns. With `coupling`, the leader's
/// rows that involve follower variables are added too.
fn follower_problem(
    m: &Bilevel,
    val: &[f64],
    obj_upper: bool,
    coupling: bool,
) -> (Milp, Vec<usize>) {
    let mut mp = Milp::new(false);
    let mut col = vec![usize::MAX; m.vars.len()];
    for (j, v) in m.vars.iter().enumerate() {
        if v.lower {
            col[j] = mp.add_var(
                if obj_upper { v.obj } else { v.ll_obj },
                v.lb,
                v.ub,
                v.integer,
            );
        }
    }
    for r in &m.rows {
        let has_follower = r.coefs.iter().any(|&(j, _)| m.vars[j].lower);
        if !(r.lower || (coupling && has_follower)) {
            continue;
        }
        let fixed: f64 = r
            .coefs
            .iter()
            .filter(|&&(j, _)| !m.vars[j].lower)
            .map(|&(j, c)| c * val[j])
            .sum();
        let coefs: Vec<(usize, f64)> = r
            .coefs
            .iter()
            .filter(|&&(j, _)| m.vars[j].lower)
            .map(|&(j, c)| (col[j], c))
            .collect();
        if coefs.is_empty() {
            continue;
        }
        mp.add_row(coefs, r.lo - fixed, r.hi - fixed);
    }
    (mp, col)
}

pub fn has_coupling(m: &Bilevel) -> bool {
    m.rows
        .iter()
        .any(|r| !r.lower && r.coefs.iter().any(|&(j, _)| m.vars[j].lower))
}

/// HPR, then the follower at the HPR's leader values, then the optimistic
/// tie-break. Time limits are per stage, in seconds.
pub fn feasible_point(m: &Bilevel, hpr_time: f64, follower_time: f64) -> GeneralOutcome {
    let h = hpr(m).solve(hpr_time, 1);
    match h.status {
        MilpStatus::Infeasible => return GeneralOutcome::Infeasible,
        MilpStatus::Optimal | MilpStatus::Feasible => {}
        MilpStatus::Unbounded => return GeneralOutcome::Failed("HPR unbounded".into()),
        MilpStatus::Unknown => {
            return GeneralOutcome::Failed(format!("HPR found no point in {hpr_time}s"))
        }
    }
    let mut val = h.x.clone();
    integer_round(m, &mut val);
    // Follower optimum at the HPR's leader values.
    let (fp, col) = follower_problem(m, &val, false, false);
    let f = fp.solve(follower_time, 1);
    if f.status == MilpStatus::Unbounded {
        // The follower can drive its objective to minus infinity here, so
        // this leader choice admits no optimal follower response.
        return GeneralOutcome::Failed("follower unbounded at the HPR's leader values".into());
    }
    if f.status != MilpStatus::Optimal {
        return GeneralOutcome::Failed(format!(
            "follower not solved to optimality ({:?})",
            f.status
        ));
    }
    let phi = f.obj;
    // Optimistic tie-break: best leader objective among follower optima
    // (within the 1e-4 relative tolerance BOBILib's checker uses, here 1e-7).
    let coupling = has_coupling(m);
    let (mut tb, col2) = follower_problem(m, &val, true, coupling);
    let ll: Vec<(usize, f64)> = m
        .vars
        .iter()
        .enumerate()
        .filter(|(_, v)| v.lower && v.ll_obj != 0.0)
        .map(|(j, v)| (col2[j], v.ll_obj))
        .collect();
    let tol = 1e-7 * (1.0 + phi.abs());
    tb.add_row(ll, f64::NEG_INFINITY, phi + tol);
    let t = tb.solve(follower_time, 1);
    let (chosen, cols) = match t.status {
        MilpStatus::Optimal | MilpStatus::Feasible => (&t.x, &col2),
        MilpStatus::Infeasible if coupling => {
            return GeneralOutcome::Failed(
                "no follower optimum satisfies the coupling rows at the HPR's leader values".into(),
            )
        }
        _ if !coupling => (&f.x, &col), // the follower optimum itself
        _ => return GeneralOutcome::Failed(format!("tie-break failed ({:?})", t.status)),
    };
    for (j, v) in m.vars.iter().enumerate() {
        if v.lower {
            val[j] = chosen[cols[j]];
        }
    }
    integer_round(m, &mut val);
    GeneralOutcome::Point(GeneralPoint {
        upper_obj: m.upper_obj(&val),
        lower_obj: m.lower_obj(&val),
        values: val,
    })
}

/// Re-solve the follower at the point's leader values and compare: returns
/// (worst row/bound violation, the follower's optimum there, the point's
/// follower objective). BOBILib accepts |phi_sol - phi| / (1e-10 + |phi|) <= 1e-4.
pub fn check_point(m: &Bilevel, val: &[f64], follower_time: f64) -> (f64, Option<f64>, f64) {
    let (viol, _) = m.max_violation(val);
    let (fp, _) = follower_problem(m, val, false, false);
    let f = fp.solve(follower_time, 1);
    let phi = (f.status == MilpStatus::Optimal).then_some(f.obj);
    (viol, phi, m.lower_obj(val))
}

/// A certificate that the follower is unbounded for every leader choice
/// that leaves it feasible: a direction d of the follower's variables with
/// follower objective e d < 0 that every follower row and bound admits
/// (a d <= 0 where the row has an upper bound, a d >= 0 where it has a lower
/// bound; d_j >= 0 under a finite lower bound, d_j <= 0 under a finite upper
/// bound). Leader variables only shift the rows' right-hand sides, so d is a
/// recession direction of the follower's region for every x; with rational
/// data an unbounded LP relaxation makes a feasible MILP unbounded too
/// (Meyer 1974). Then no x has an optimal follower response and the
/// instance has no bilevel feasible point. Returns d scaled to max |d_j| = 1
/// and e d, or None if no such direction exists.
pub fn follower_unbounded_ray(m: &Bilevel, time_limit: f64) -> Option<(Vec<f64>, f64)> {
    let mut mp = Milp::new(false);
    let mut col = vec![usize::MAX; m.vars.len()];
    for (j, v) in m.vars.iter().enumerate() {
        if v.lower {
            let lo = if v.lb.is_finite() { 0.0 } else { -1.0 };
            let hi = if v.ub.is_finite() { 0.0 } else { 1.0 };
            col[j] = mp.add_var(v.ll_obj, lo, hi, false);
        }
    }
    for r in m.rows.iter().filter(|r| r.lower) {
        let coefs: Vec<(usize, f64)> = r
            .coefs
            .iter()
            .filter(|&&(j, _)| m.vars[j].lower)
            .map(|&(j, c)| (col[j], c))
            .collect();
        if coefs.is_empty() {
            continue;
        }
        let lo = if r.lo.is_finite() {
            0.0
        } else {
            f64::NEG_INFINITY
        };
        let hi = if r.hi.is_finite() { 0.0 } else { f64::INFINITY };
        mp.add_row(coefs, lo, hi);
    }
    let r = mp.solve(time_limit, 1);
    if r.status != MilpStatus::Optimal || r.obj > -1e-7 {
        return None;
    }
    let mut d = vec![0.0; m.vars.len()];
    for (j, v) in m.vars.iter().enumerate() {
        if v.lower {
            d[j] = r.x[col[j]];
        }
    }
    Some((d, r.obj))
}
