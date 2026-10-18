//! A thin wrapper over HiGHS for the small MILPs the interdiction solvers
//! need (follower subproblems, master problems).

use highs::{HighsModelStatus, RowProblem, Sense};
use std::ffi::CString;

#[derive(Clone, Debug)]
pub struct Milp {
    pub obj: Vec<f64>,
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    pub integer: Vec<bool>,
    pub rows: Vec<(Vec<(usize, f64)>, f64, f64)>,
    pub maximize: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MilpStatus {
    Optimal,
    /// Stopped by the time limit with a feasible point.
    Feasible,
    Infeasible,
    /// Stopped without a feasible point, or another failure.
    Unknown,
}

#[derive(Clone, Debug)]
pub struct MilpResult {
    pub status: MilpStatus,
    pub obj: f64,
    /// Proven bound on the optimum (upper for max, lower for min).
    pub bound: f64,
    pub x: Vec<f64>,
}

impl Milp {
    pub fn new(maximize: bool) -> Self {
        Milp {
            obj: vec![],
            lb: vec![],
            ub: vec![],
            integer: vec![],
            rows: vec![],
            maximize,
        }
    }

    pub fn add_var(&mut self, obj: f64, lb: f64, ub: f64, integer: bool) -> usize {
        self.obj.push(obj);
        self.lb.push(lb);
        self.ub.push(ub);
        self.integer.push(integer);
        self.obj.len() - 1
    }

    pub fn add_row(&mut self, coefs: Vec<(usize, f64)>, lo: f64, hi: f64) {
        self.rows.push((coefs, lo, hi));
    }

    pub fn solve(&self, time_limit: f64, threads: i32) -> MilpResult {
        let mut pb = RowProblem::new();
        let cols: Vec<_> = (0..self.obj.len())
            .map(|j| {
                pb.add_column_with_integrality(
                    self.obj[j],
                    self.lb[j]..=self.ub[j],
                    self.integer[j],
                )
            })
            .collect();
        for (coefs, lo, hi) in &self.rows {
            let f: Vec<_> = coefs.iter().map(|&(j, c)| (cols[j], c)).collect();
            // HiGHS reads infinite bounds as free.
            pb.add_row(*lo..=*hi, f);
        }
        let sense = if self.maximize {
            Sense::Maximise
        } else {
            Sense::Minimise
        };
        let mut model = pb.optimise(sense);
        model.make_quiet();
        model.set_option("time_limit", time_limit);
        model.set_option("threads", threads);
        model.set_option("mip_rel_gap", 0.0);
        model.set_option("mip_abs_gap", 1e-6);
        let solved = model.solve();
        let status = solved.status();
        let mut bound = f64::NAN;
        let key = CString::new("mip_dual_bound").unwrap();
        unsafe {
            highs_sys::Highs_getDoubleInfoValue(solved.as_ptr(), key.as_ptr(), &mut bound);
        }
        let x = solved.get_solution().columns().to_vec();
        let has_x = x.len() == self.obj.len() && x.iter().all(|v| v.is_finite());
        let obj = if has_x {
            self.obj.iter().zip(&x).map(|(c, v)| c * v).sum()
        } else {
            f64::NAN
        };
        let status = match status {
            HighsModelStatus::Optimal => MilpStatus::Optimal,
            HighsModelStatus::Infeasible => MilpStatus::Infeasible,
            _ if has_x && self.feasible(&x) => MilpStatus::Feasible,
            _ => MilpStatus::Unknown,
        };
        if status == MilpStatus::Optimal && !bound.is_finite() {
            bound = obj;
        }
        MilpResult {
            status,
            obj,
            bound,
            x,
        }
    }

    pub fn feasible(&self, x: &[f64]) -> bool {
        let tol = 1e-6;
        (0..x.len()).all(|j| x[j] >= self.lb[j] - tol && x[j] <= self.ub[j] + tol)
            && self.rows.iter().all(|(c, lo, hi)| {
                let a: f64 = c.iter().map(|&(j, v)| v * x[j]).sum();
                a >= lo - tol * (1.0 + lo.abs()) && a <= hi + tol * (1.0 + hi.abs())
            })
    }
}
