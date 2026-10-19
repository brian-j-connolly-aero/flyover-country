//! Bilevel feasible points for general MIBLPs whose leader rows involve
//! follower variables (coupling rows).
//!
//! A point (x, y) is bilevel feasible (optimistic convention) when it
//! satisfies every row and y is an optimal follower response to x. The
//! follower's problem depends on the leader only through the *linking*
//! variables x_L (leader variables with a nonzero in a follower row), so
//! for a fixed x_L the leader's best feasible point is one MILP over the
//! other leader variables x_N and y: all rows, plus f(y) <= phi(x_L), the
//! follower's optimum. Without coupling rows the high point relaxation's
//! (HPR's) x_L always gives a point. With coupling rows the follower's
//! optima at that x_L may all violate them, and the search below looks for
//! an x_L where they do not:
//!
//! * `hpr`: the HPR's x_L, then the best response.
//! * `follow`: best-response dynamics. The leader re-optimises against the
//!   follower's last response (its integer part fixed, or the HPR limited
//!   to follower values at least as good), then the follower answers.
//! * `nogood`: repeat with a no-good cut on the integer linking variables
//!   after each failure (exact when every linking variable is integer).
//! * `penalty`: the HPR with objective F + rho f for increasing rho.
//! * `vcut`: value-function cuts. A follower response y^ with value f^ at
//!   some x_L stays available at every x_L' under which y^ satisfies the
//!   follower rows, so phi(x_L') <= f^ there; the cut "y^ is infeasible at
//!   x_L, or f(y) <= f^" (binary indicators per linking follower row) is
//!   valid for every bilevel feasible point and the HPR with these cuts
//!   converges to one (the method of Mitsos 2010 and Lozano and Smith 2017).
//! * `padm`: a penalty alternating direction method in the spirit of
//!   Kleinert and Schmidt (INFORMS JoC 2021): duals of the follower's LP
//!   relaxation at x give a linear lower bound L(x) on its value, the HPR
//!   minimises F + rho (f(y) - L(x)) (zero penalty certifies follower
//!   optimality), then the duals are refreshed at the new x.
//! * `perturb`: for continuous linking variables, move x_L from the
//!   HPR's point towards an interior point of the HPR's LP relaxation.
//!
//! Every point returned is re-checked from scratch by [`certify`]: rows and
//! bounds within 1e-6 and a fresh follower solve whose dual bound shows the
//! point's follower value within BOBILib's relative 1e-4 of the optimum.
//!
//! [`Tolerance::Strict`] (`bobi coupled --strict`, `bobi coupled-tighten`,
//! `bobi certify --tight`) holds the follower to its optimum instead: the
//! follower is solved with zero relative gap, best responses cap its value
//! at the optimum plus [`STRICT_CAPS`] (absolute), and the certificate asks
//! for the point's follower value at most [`STRICT_ABS`] above the proven
//! bound, so the leader cannot use the relative slack BOBILib allows.

use crate::milp::{Milp, MilpResult, MilpStatus, SolveOpts};
use crate::model::{Bilevel, Row};
use std::collections::HashMap;
use std::time::Instant;

/// Absolute row tolerance BOBILib's checker uses.
pub const ROW_TOL: f64 = 1e-6;
/// BOBILib's relative tolerance on the follower's objective.
pub const BOBILIB_REL: f64 = 1e-4;
/// We certify follower optimality with a 10x margin on BOBILib's tolerance.
const CERT_REL: f64 = 1e-5;
/// Strict mode: the follower's value at most this far (absolute) above the
/// proven lower bound on its optimum.
pub const STRICT_ABS: f64 = 1e-6;
/// Strict mode: the caps on the follower's value in a best response, above
/// the follower's optimum (absolute), well inside [`STRICT_ABS`] so that
/// rounding and polishing keep the final point within it. The second is
/// tried when the first gives no point: on objectives near 1e7 a cap of 1e-9
/// is about one unit in the last place, below what the solver can resolve.
pub const STRICT_CAPS: [f64; 2] = [1e-9, 1e-7];
/// Strict mode: the leader's objective at most this far (relative, on top
/// of [`STRICT_ABS`] absolute) above the proven tie-break bound.
const STRICT_TIE_REL: f64 = 1e-9;

/// Which follower-optimality standard points must meet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tolerance {
    /// BOBILib's relative 1e-4 (certified with a 10x margin); best responses
    /// let the follower's value sit up to 1e-7 (1 + |phi|) above its optimum.
    #[default]
    Bobilib,
    /// The follower's value within [`STRICT_ABS`] (absolute) of its optimum.
    Strict,
}

impl Tolerance {
    fn strict(self) -> bool {
        self == Tolerance::Strict
    }

    /// Solver settings for the follower's problem at fixed leader values.
    fn follower_opts(self, time: f64) -> SolveOpts {
        let mut o = SolveOpts::new(time);
        o.abs_gap = 1e-9;
        if self.strict() {
            o.rel_gap = 0.0;
            o.feas_tol = Some(1e-9);
        } else {
            o.rel_gap = 1e-7;
        }
        o
    }
}

/// Which variables and rows play which part.
#[derive(Clone, Debug)]
pub struct Split {
    /// Leader variables with a nonzero in some follower row.
    pub linking: Vec<bool>,
    /// Leader rows with follower variables.
    pub coupling: Vec<usize>,
    /// The follower's objective takes integer values at integer points.
    pub ll_integral: bool,
}

impl Split {
    pub fn new(m: &Bilevel) -> Self {
        let mut linking = vec![false; m.vars.len()];
        let mut coupling = Vec::new();
        for (i, r) in m.rows.iter().enumerate() {
            if r.lower {
                for &(j, _) in &r.coefs {
                    if !m.vars[j].lower {
                        linking[j] = true;
                    }
                }
            } else if r.coefs.iter().any(|&(j, _)| m.vars[j].lower) {
                coupling.push(i);
            }
        }
        let ll_integral = m
            .vars
            .iter()
            .all(|v| v.ll_obj == 0.0 || (v.integer && v.ll_obj == v.ll_obj.round()));
        Split {
            linking,
            coupling,
            ll_integral,
        }
    }
}

/// A MILP over the variables `fixed` leaves free, with fixed values moved
/// to the right-hand sides.
struct Sub {
    mp: Milp,
    /// Model variable -> MILP column (usize::MAX when fixed).
    col: Vec<usize>,
    /// False when a row over fixed variables only is violated.
    consistent: bool,
}

fn submodel(
    m: &Bilevel,
    fixed: &[Option<f64>],
    keep: impl Fn(&Row) -> bool,
    obj: impl Fn(usize) -> f64,
) -> Sub {
    let mut mp = Milp::new(false);
    let mut col = vec![usize::MAX; m.vars.len()];
    for (j, v) in m.vars.iter().enumerate() {
        if fixed[j].is_none() {
            col[j] = mp.add_var(obj(j), v.lb, v.ub, v.integer);
        }
    }
    let mut consistent = true;
    for r in m.rows.iter().filter(|r| keep(r)) {
        let mut shift = 0.0;
        let mut coefs = Vec::with_capacity(r.coefs.len());
        for &(j, c) in &r.coefs {
            match fixed[j] {
                Some(v) => shift += c * v,
                None => coefs.push((col[j], c)),
            }
        }
        if coefs.is_empty() {
            if shift < r.lo - ROW_TOL || shift > r.hi + ROW_TOL {
                consistent = false;
            }
            continue;
        }
        mp.add_row(coefs, r.lo - shift, r.hi - shift);
    }
    Sub {
        mp,
        col,
        consistent,
    }
}

/// Full-length values from a submodel's solution, fixed ones from `base`.
fn expand(sub: &Sub, x: &[f64], base: &[f64]) -> Vec<f64> {
    sub.col
        .iter()
        .enumerate()
        .map(|(j, &c)| if c == usize::MAX { base[j] } else { x[c] })
        .collect()
}

fn round_integers(m: &Bilevel, v: &mut [f64]) {
    for (var, x) in m.vars.iter().zip(v.iter_mut()) {
        if var.integer {
            *x = x.round();
        }
        // Snap to bounds the solver overshot by its tolerance.
        if var.lb <= var.ub {
            *x = x.clamp(var.lb, var.ub);
        }
    }
}

/// True when a follower value at a point is certified optimal: the true
/// optimum lies in [bound, value], so the relative error BOBILib computes is
/// at most (value - bound) / (1e-10 + min |p| over that interval).
pub fn gap_certified(value: f64, bound: f64, ll_integral: bool) -> bool {
    gap_ok(value, bound, ll_integral, Tolerance::Bobilib)
}

/// [`gap_certified`] under either tolerance; strict: at most [`STRICT_ABS`]
/// (absolute) above the bound.
pub fn gap_ok(value: f64, bound: f64, ll_integral: bool, tol: Tolerance) -> bool {
    let bound = if ll_integral {
        (bound - 1e-6).ceil()
    } else {
        bound
    };
    match tol {
        Tolerance::Bobilib => worst_rel_error(value, bound) <= CERT_REL,
        Tolerance::Strict => value.is_finite() && bound.is_finite() && value - bound <= STRICT_ABS,
    }
}

/// A follower response at some leader values.
#[derive(Clone, Debug)]
pub struct Response {
    pub status: MilpStatus,
    /// Follower objective of the response (NaN without one).
    pub value: f64,
    /// Proven lower bound on the follower's optimum.
    pub bound: f64,
    /// Full-length values: the leader's as given, the follower's response.
    pub point: Vec<f64>,
    pub secs: f64,
}

impl Response {
    /// The follower value best responses are capped against: the response's
    /// own value or, under the strict tolerance, the proven bound when
    /// rounding left the value below it (the optimum is at least the bound).
    fn reference(&self, tol: Tolerance) -> f64 {
        if tol.strict() {
            self.value.max(self.bound)
        } else {
            self.value
        }
    }
}

/// Solve the follower at `x`'s leader values (rows with follower variables
/// that belong to the follower; coupling rows are ignored).
pub fn follower_response(m: &Bilevel, x: &[f64], time: f64) -> Response {
    follower_response_with(m, x, time, Tolerance::Bobilib)
}

/// [`follower_response`] with the solver settings of `tol` (strict: zero
/// relative gap, feasibility tolerance 1e-9).
pub fn follower_response_with(m: &Bilevel, x: &[f64], time: f64, tol: Tolerance) -> Response {
    let t = Instant::now();
    let fixed: Vec<Option<f64>> = m
        .vars
        .iter()
        .zip(x)
        .map(|(v, &val)| if v.lower { None } else { Some(val) })
        .collect();
    let sub = submodel(m, &fixed, |r| r.lower, |j| m.vars[j].ll_obj);
    if !sub.consistent {
        return Response {
            status: MilpStatus::Infeasible,
            value: f64::NAN,
            bound: f64::NAN,
            point: x.to_vec(),
            secs: t.elapsed().as_secs_f64(),
        };
    }
    let r = sub.mp.solve_with(&tol.follower_opts(time));
    let has = matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible);
    let point = if has {
        let mut p = expand(&sub, &r.x, x);
        round_integers(m, &mut p);
        p
    } else {
        x.to_vec()
    };
    let value = if has { m.lower_obj(&point) } else { f64::NAN };
    Response {
        status: r.status,
        value,
        bound: r.bound,
        point,
        secs: t.elapsed().as_secs_f64(),
    }
}

/// The result of re-checking a point from scratch.
#[derive(Clone, Debug)]
pub struct Certificate {
    pub ok: bool,
    pub max_violation: f64,
    pub worst: String,
    pub upper_obj: f64,
    pub lower_obj: f64,
    /// The fresh follower solve's incumbent and dual bound.
    pub phi_value: f64,
    pub phi_bound: f64,
    /// BOBILib's relative error on the follower's value against the worst
    /// optimum the bound allows.
    pub rel_error: f64,
    /// Proven lower bound on the leader's objective over the follower's
    /// optima at the point's leader values (the optimistic tie-break), and
    /// BOBILib's relative error of the point's leader objective against it.
    pub tie_bound: f64,
    pub tie_rel_error: f64,
    /// The point's follower value minus the proven bound on the follower's
    /// optimum (absolute), and the leader's objective minus the tie-break
    /// bound.
    pub slack: f64,
    pub tie_slack: f64,
    /// The standard the certificate applied.
    pub tolerance: Tolerance,
}

/// BOBILib's relative error |v - p| / (1e-10 + |p|) maximised over the
/// optima p in [bound, value] the bound allows.
pub fn worst_rel_error(value: f64, bound: f64) -> f64 {
    if !value.is_finite() || !bound.is_finite() {
        return f64::INFINITY;
    }
    if value <= bound {
        return 0.0;
    }
    let lo_abs = if bound <= 0.0 && value >= 0.0 {
        0.0
    } else {
        bound.abs().min(value.abs())
    };
    (value - bound) / (1e-10 + lo_abs)
}

/// Solve `mp`, whose last row caps the follower's value at `phi + caps[0]`,
/// widening the cap through `caps` while the solver finds no point.
fn solve_capped(mp: &mut Milp, phi: f64, caps: &[f64], o: &SolveOpts) -> MilpResult {
    let mut r = mp.solve_with(o);
    for &cap in &caps[1..] {
        if matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
            break;
        }
        if let Some(row) = mp.rows.last_mut() {
            row.2 = phi + cap;
        }
        r = mp.solve_with(o);
    }
    r
}

/// Integer-valued objective at integer points: bounds can be rounded up.
fn integral_objective(m: &Bilevel, coef: impl Fn(&crate::model::Var) -> f64) -> bool {
    m.vars
        .iter()
        .all(|v| coef(v) == 0.0 || (v.integer && coef(v) == coef(v).round()))
}

/// Re-check a point the way BOBILib's checker does, from scratch: bounds,
/// integrality and rows within 1e-6; the follower's value optimal at the
/// point's leader values; and the leader's value optimal among the
/// follower's optima there (all rows, follower value at most the point's).
/// Both optimality checks use proven dual bounds with a 10x margin on
/// BOBILib's relative 1e-4.
pub fn certify(m: &Bilevel, val: &[f64], time: f64) -> Certificate {
    certify_with(m, val, time, Tolerance::Bobilib)
}

/// [`certify`] under either tolerance. Strict: the follower solved with zero
/// relative gap and its value at most [`STRICT_ABS`] above the proven bound;
/// the tie-break over follower values at most the point's own (plus 1e-9,
/// absolute, or 1e-7 if the solver finds no point with that), solved to a
/// relative gap of 1e-9, with the leader's objective
/// at most [`STRICT_ABS`] + 1e-9 |F| above its bound.
pub fn certify_with(m: &Bilevel, val: &[f64], time: f64, tol: Tolerance) -> Certificate {
    let (viol, worst) = m.max_violation(val);
    let ll = m.lower_obj(val);
    let upper = m.upper_obj(val);
    let ll_integral = integral_objective(m, |v| v.ll_obj);
    let f = follower_response_with(m, val, time, tol);
    let mut bound = f.bound;
    if ll_integral && bound.is_finite() {
        bound = (bound - 1e-6).ceil();
    }
    let rel_error = worst_rel_error(ll, bound);
    let opt_ok = matches!(f.status, MilpStatus::Optimal | MilpStatus::Feasible)
        && gap_ok(ll, f.bound, ll_integral, tol);
    // The optimistic tie-break at the point's leader values.
    let fixed: Vec<Option<f64>> = m
        .vars
        .iter()
        .zip(val)
        .map(|(v, &x)| if v.lower { None } else { Some(x) })
        .collect();
    let mut sub = submodel(m, &fixed, |_| true, |j| m.vars[j].obj);
    let ll_terms: Vec<(usize, f64)> = m
        .vars
        .iter()
        .enumerate()
        .filter(|(_, v)| v.lower && v.ll_obj != 0.0)
        .map(|(j, v)| (sub.col[j], v.ll_obj))
        .collect();
    let caps: &[f64] = if tol.strict() {
        &STRICT_CAPS
    } else {
        &[1e-9 * (1.0 + ll.abs())]
    };
    // A point whose rows are met only within the tolerance can sit below
    // the follower's proven optimum; the region starts at the optimum then.
    let ll_ref = if tol.strict() { ll.max(bound) } else { ll };
    sub.mp
        .add_row(ll_terms, f64::NEG_INFINITY, ll_ref + caps[0]);
    let mut o = SolveOpts::new(time);
    o.rel_gap = if tol.strict() { STRICT_TIE_REL } else { 1e-7 };
    o.abs_gap = 1e-9;
    if tol.strict() {
        o.feas_tol = Some(1e-9);
    }
    let t = solve_capped(&mut sub.mp, ll_ref, caps, &o);
    let fixed_part: f64 = m.obj_offset
        + m.vars
            .iter()
            .zip(val)
            .filter(|(v, _)| !v.lower)
            .map(|(v, &x)| v.obj * x)
            .sum::<f64>();
    let mut tie_bound = if matches!(t.status, MilpStatus::Optimal | MilpStatus::Feasible) {
        fixed_part + t.bound
    } else {
        f64::NAN
    };
    if integral_objective(m, |v| v.obj) && m.obj_offset == m.obj_offset.round() {
        tie_bound = (tie_bound - 1e-6).ceil();
    }
    let tie_rel_error = worst_rel_error(upper, tie_bound);
    let tie_slack = upper - tie_bound;
    let tie_ok = match tol {
        Tolerance::Bobilib => tie_rel_error <= CERT_REL,
        Tolerance::Strict => tie_slack <= STRICT_ABS + STRICT_TIE_REL * upper.abs(),
    };
    Certificate {
        ok: viol <= ROW_TOL && opt_ok && tie_ok,
        max_violation: viol,
        worst,
        upper_obj: upper,
        lower_obj: ll,
        phi_value: f.value,
        phi_bound: bound,
        rel_error,
        tie_bound,
        tie_rel_error,
        slack: ll - bound,
        tie_slack,
        tolerance: tol,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Hpr,
    NoGood,
    Penalty,
    VCut,
    Padm,
    Perturb,
    Follow,
}

impl Method {
    pub fn name(self) -> &'static str {
        match self {
            Method::Hpr => "hpr",
            Method::NoGood => "nogood",
            Method::Penalty => "penalty",
            Method::VCut => "vcut",
            Method::Padm => "padm",
            Method::Perturb => "perturb",
            Method::Follow => "follow",
        }
    }

    pub fn parse(s: &str) -> Option<Method> {
        Some(match s {
            "hpr" => Method::Hpr,
            "nogood" => Method::NoGood,
            "penalty" => Method::Penalty,
            "vcut" => Method::VCut,
            "padm" => Method::Padm,
            "perturb" => Method::Perturb,
            "follow" => Method::Follow,
            _ => return None,
        })
    }
}

/// What one method did.
#[derive(Clone, Debug, Default)]
pub struct MethodReport {
    pub name: String,
    /// HPR-type MILPs solved.
    pub masters: usize,
    /// Leader choices whose follower was solved.
    pub leaders: usize,
    /// Leader choices where no follower optimum met the coupling rows.
    pub blocked: usize,
    pub secs: f64,
    /// Why the method stopped.
    pub end: String,
    pub found: bool,
}

/// A verified point and how it was found.
#[derive(Clone, Debug)]
pub struct Found {
    pub values: Vec<f64>,
    pub method: String,
    pub secs: f64,
    pub cert: Certificate,
    /// The best response optimised over the non-linking leader variables too.
    pub free_leader: bool,
}

#[derive(Clone, Debug)]
pub struct SearchOpts {
    /// Total wall-clock budget in seconds.
    pub time: f64,
    /// Cap on one follower or best-response solve.
    pub follower_time: f64,
    /// Cap on one HPR-type master solve, as a fraction of the budget.
    pub master_frac: f64,
    pub methods: Vec<Method>,
    pub log: bool,
    /// Keep searching after the first point for this many seconds of the
    /// budget, keeping the best leader objective (0 stops at the first).
    pub improve: bool,
    /// The follower-optimality standard of best responses and certificates.
    pub tolerance: Tolerance,
}

/// Outcome of a leader choice.
enum Eval {
    Point(Box<Found>),
    /// No follower optimum meets the coupling rows (the follower response
    /// is kept for value-function cuts).
    Blocked(Box<Response>),
    /// The follower or the best response did not finish, or the follower
    /// has no optimal response here.
    Unresolved(String, Option<Box<Response>>),
    /// The same linking values were tried before.
    Seen,
}

/// Value-function cut: y^ (full-length values) with follower value f^.
#[derive(Clone, Debug)]
struct VCut {
    point: Vec<f64>,
    value: f64,
}

pub struct Search<'a> {
    m: &'a Bilevel,
    pub split: Split,
    opts: SearchOpts,
    start: Instant,
    /// Follower responses by quantised linking values.
    seen: HashMap<Vec<i64>, Option<Response>>,
    /// Linking values shown blocked (no follower optimum meets the coupling
    /// rows for any values of the other leader variables).
    blocked: std::collections::HashSet<Vec<i64>>,
    /// A method proved that no bilevel feasible point exists.
    pub proven_infeasible: bool,
    pub reports: Vec<MethodReport>,
    pub best: Option<Found>,
    hpr0: Option<Vec<f64>>,
    pub hpr_status: Option<MilpStatus>,
    /// Scale making rho = 1 balance the two objectives.
    rho_scale: f64,
    current: MethodReport,
}

fn objective_scale(m: &Bilevel) -> f64 {
    let up: f64 = m.vars.iter().map(|v| v.obj.abs()).sum();
    let lo: f64 = m.vars.iter().map(|v| v.ll_obj.abs()).sum();
    if up > 0.0 && lo > 0.0 {
        up / lo
    } else {
        1.0
    }
}

impl<'a> Search<'a> {
    pub fn new(m: &'a Bilevel, opts: SearchOpts) -> Self {
        Search {
            m,
            split: Split::new(m),
            opts,
            start: Instant::now(),
            seen: HashMap::new(),
            blocked: Default::default(),
            proven_infeasible: false,
            reports: Vec::new(),
            best: None,
            hpr0: None,
            hpr_status: None,
            rho_scale: objective_scale(m),
            current: MethodReport::default(),
        }
    }

    fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn left(&self) -> f64 {
        (self.opts.time - self.elapsed()).max(0.0)
    }

    fn log(&self, msg: &str) {
        if self.opts.log {
            eprintln!("[{:7.1}s] {}: {msg}", self.elapsed(), self.m.name);
        }
    }

    fn master_cap(&self) -> f64 {
        (self.opts.time * self.opts.master_frac).min(self.left())
    }

    fn key(&self, x: &[f64]) -> Vec<i64> {
        self.m
            .vars
            .iter()
            .enumerate()
            .filter(|(j, _)| self.split.linking[*j])
            .map(|(j, _)| (x[j] * 1e6).round() as i64)
            .collect()
    }

    /// Why the HPR's leader choice fails: the least total coupling-row
    /// violation over the follower's optima there, the rows that stay
    /// violated, and whether the follower's LP region bounds them at all.
    pub fn diagnose(&mut self) -> Vec<String> {
        let m = self.m;
        let mut out = Vec::new();
        let x = match self.hpr_point() {
            Ok(x) => x,
            Err(e) => return vec![e],
        };
        let mut x = x;
        round_integers(m, &mut x);
        let resp = follower_response(m, &x, self.opts.follower_time);
        out.push(format!(
            "follower {:?} value {} bound {} in {:.1}s; HPR's own follower value {}",
            resp.status,
            resp.value,
            resp.bound,
            resp.secs,
            m.lower_obj(&x)
        ));
        if !resp.value.is_finite() {
            return out;
        }
        let phi = resp.value;
        // Elastic tie-break: linking values fixed, follower optimal, the
        // other leader variables free, coupling rows soft.
        let fixed: Vec<Option<f64>> = m
            .vars
            .iter()
            .enumerate()
            .map(|(j, v)| {
                if v.lower || !self.split.linking[j] {
                    None
                } else {
                    Some(x[j])
                }
            })
            .collect();
        let has_y = |r: &Row| r.coefs.iter().any(|&(j, _)| m.vars[j].lower);
        let mut sub = submodel(m, &fixed, |r| r.lower || !has_y(r), |_| 0.0);
        let ll: Vec<(usize, f64)> = m
            .vars
            .iter()
            .enumerate()
            .filter(|(_, v)| v.lower && v.ll_obj != 0.0)
            .map(|(j, v)| (sub.col[j], v.ll_obj))
            .collect();
        sub.mp
            .add_row(ll, f64::NEG_INFINITY, phi + 1e-7 * (1.0 + phi.abs()));
        let mut slack_of = Vec::new();
        for &i in &self.split.coupling {
            let r = &m.rows[i];
            let mut shift = 0.0;
            let mut coefs = Vec::new();
            for &(j, c) in &r.coefs {
                match fixed[j] {
                    Some(v) => shift += c * v,
                    None => coefs.push((sub.col[j], c)),
                }
            }
            let sp = sub.mp.add_var(1.0, 0.0, f64::INFINITY, false);
            let sn = sub.mp.add_var(1.0, 0.0, f64::INFINITY, false);
            coefs.push((sp, 1.0));
            coefs.push((sn, -1.0));
            sub.mp.add_row(coefs, r.lo - shift, r.hi - shift);
            slack_of.push((i, sp, sn));
        }
        let r = sub.mp.solve_with(&SolveOpts::new(self.opts.follower_time));
        out.push(format!(
            "elastic tie-break {:?}: least total coupling violation {}",
            r.status, r.obj
        ));
        if !matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
            return out;
        }
        let y = expand(&sub, &r.x, &x);
        let mut viol: Vec<(f64, usize)> = slack_of
            .iter()
            .map(|&(i, sp, sn)| (r.x[sp] + r.x[sn], i))
            .filter(|&(v, _)| v > 1e-6)
            .collect();
        viol.sort_by(|a, b| b.0.total_cmp(&a.0));
        out.push(format!("{} coupling rows stay violated", viol.len()));
        for &(v, i) in viol.iter().take(8) {
            let row = &m.rows[i];
            let nl = row.coefs.iter().filter(|&&(j, _)| !m.vars[j].lower).count();
            let nf = row.coefs.len() - nl;
            let nf_obj = row
                .coefs
                .iter()
                .filter(|&&(j, _)| m.vars[j].lower && m.vars[j].ll_obj != 0.0)
                .count();
            let act: f64 = row.coefs.iter().map(|&(j, c)| c * y[j]).sum();
            // Can the follower's LP region push this row's activity past its
            // bound at all? max / min of the row's follower part over Y_LP(x).
            let leader_fixed: Vec<Option<f64>> = m
                .vars
                .iter()
                .zip(&x)
                .map(|(v, &val)| if v.lower { None } else { Some(val) })
                .collect();
            let mut lp = submodel(
                m,
                &leader_fixed,
                |rr| rr.lower,
                |j| {
                    row.coefs
                        .iter()
                        .find(|&&(k, _)| k == j)
                        .map_or(0.0, |&(_, c)| c)
                },
            );
            for i in lp.mp.integer.iter_mut() {
                *i = false;
            }
            let lo_r = lp.mp.solve_with(&SolveOpts::new(30.0));
            lp.mp.maximize = true;
            let hi_r = lp.mp.solve_with(&SolveOpts::new(30.0));
            let fixed_part: f64 = row
                .coefs
                .iter()
                .filter(|&&(j, _)| !m.vars[j].lower)
                .map(|&(j, c)| c * y[j])
                .sum();
            out.push(format!(
                "  {} [{}, {}] activity {:.4} violation {:.4}: {} leader / {} follower terms ({} in f); follower LP range [{:?} {:.4}, {:?} {:.4}] + leader part {:.4}",
                row.name, row.lo, row.hi, act, v, nl, nf, nf_obj,
                lo_r.status, lo_r.obj, hi_r.status, hi_r.obj, fixed_part
            ));
        }
        out
    }

    /// The leader's best point among the follower's optima at x's linking
    /// values; leader variables outside the follower's rows are fixed to x's
    /// values unless `free_leader`.
    fn best_response(&self, x: &[f64], phi: f64, free_leader: bool, time: f64) -> MilpResult {
        let m = self.m;
        let fixed: Vec<Option<f64>> = m
            .vars
            .iter()
            .enumerate()
            .map(|(j, v)| {
                if v.lower || (free_leader && !self.split.linking[j]) {
                    None
                } else {
                    Some(x[j])
                }
            })
            .collect();
        let mut sub = submodel(m, &fixed, |_| true, |j| m.vars[j].obj);
        if !sub.consistent {
            return MilpResult {
                status: MilpStatus::Infeasible,
                obj: f64::NAN,
                bound: f64::NAN,
                x: vec![],
                row_duals: vec![],
                col_duals: vec![],
            };
        }
        let ll: Vec<(usize, f64)> = m
            .vars
            .iter()
            .enumerate()
            .filter(|(_, v)| v.lower && v.ll_obj != 0.0)
            .map(|(j, v)| (sub.col[j], v.ll_obj))
            .collect();
        let strict = self.opts.tolerance.strict();
        let wide = [1e-7 * (1.0 + phi.abs())];
        let caps: &[f64] = if self.split.ll_integral {
            &[1e-6]
        } else if strict {
            &STRICT_CAPS
        } else {
            &wide
        };
        sub.mp.add_row(ll, f64::NEG_INFINITY, phi + caps[0]);
        let mut o = SolveOpts::new(time);
        if strict {
            // The optimistic tie-break proper, not just a feasible response.
            o.feas_tol = Some(1e-9);
            o.rel_gap = STRICT_TIE_REL;
            o.abs_gap = 1e-9;
        } else {
            o.feas_tol = Some(1e-8);
            o.rel_gap = 1e-6;
        }
        let r = solve_capped(&mut sub.mp, phi, caps, &o);
        if matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
            let full = expand(&sub, &r.x, x);
            return MilpResult { x: full, ..r };
        }
        r
    }

    /// Round, polish the continuous variables if rows are off by more than
    /// the tolerance, and certify.
    /// `tied`: y is already the optimistic tie-break at val's leader values
    /// (a best response with every leader variable fixed, solved to
    /// optimality); otherwise that tie-break is solved here first.
    fn finish(&self, val: Vec<f64>, method: &str, free_leader: bool, tied: bool) -> Option<Found> {
        let m = self.m;
        let clean = |mut v: Vec<f64>| {
            round_integers(m, &mut v);
            if m.max_violation(&v).0 > 1e-7 {
                if let Some(p) = self.polish(&v) {
                    return p;
                }
            }
            v
        };
        let mut val = clean(val);
        let time = self.opts.follower_time.min(self.left().max(30.0));
        let tol = self.opts.tolerance;
        if !tied {
            let resp = follower_response_with(m, &val, time, tol);
            if matches!(resp.status, MilpStatus::Optimal | MilpStatus::Feasible)
                && gap_ok(resp.value, resp.bound, self.split.ll_integral, tol)
            {
                let b = self.best_response(&val, resp.reference(tol), false, time);
                if matches!(b.status, MilpStatus::Optimal | MilpStatus::Feasible) {
                    let tie = clean(b.x);
                    let ok_now = m.max_violation(&val).0 <= ROW_TOL;
                    if !ok_now || m.upper_obj(&tie) <= m.upper_obj(&val) + 1e-9 {
                        val = tie;
                    }
                }
            }
        }
        let cert = certify_with(m, &val, time, tol);
        if !cert.ok {
            self.log(&format!(
                "candidate rejected: violation {:.2e} ({}), follower {} vs bound {}, leader {} vs tie-break bound {}",
                cert.max_violation, cert.worst, cert.lower_obj, cert.phi_bound, cert.upper_obj, cert.tie_bound
            ));
            return None;
        }
        Some(Found {
            values: val,
            method: method.to_string(),
            secs: self.elapsed(),
            cert,
            free_leader,
        })
    }

    /// Fix the integers and the linking variables, then re-solve the LP over
    /// the remaining continuous variables with tight tolerances, keeping the
    /// follower's value no worse.
    fn polish(&self, val: &[f64]) -> Option<Vec<f64>> {
        let m = self.m;
        let fixed: Vec<Option<f64>> = m
            .vars
            .iter()
            .enumerate()
            .map(|(j, v)| {
                if v.integer || self.split.linking[j] {
                    Some(val[j])
                } else {
                    None
                }
            })
            .collect();
        let mut sub = submodel(m, &fixed, |_| true, |j| m.vars[j].obj);
        if !sub.consistent {
            return None;
        }
        let fixed_ll: f64 = m
            .vars
            .iter()
            .enumerate()
            .filter(|(j, v)| v.lower && fixed[*j].is_some())
            .map(|(j, v)| v.ll_obj * val[j])
            .sum();
        let ll: Vec<(usize, f64)> = m
            .vars
            .iter()
            .enumerate()
            .filter(|(j, v)| v.lower && v.ll_obj != 0.0 && fixed[*j].is_none())
            .map(|(j, v)| (sub.col[j], v.ll_obj))
            .collect();
        if !ll.is_empty() {
            let target = m.lower_obj(val) - fixed_ll;
            let slack = if self.opts.tolerance.strict() {
                1e-9
            } else {
                1e-9 * (1.0 + target.abs())
            };
            sub.mp.add_row(ll, f64::NEG_INFINITY, target + slack);
        }
        let mut o = SolveOpts::new(self.left().clamp(5.0, 60.0));
        o.feas_tol = Some(1e-9);
        let r = sub.mp.solve_with(&o);
        if r.status != MilpStatus::Optimal {
            return None;
        }
        let mut out = expand(&sub, &r.x, val);
        round_integers(m, &mut out);
        Some(out)
    }

    /// Solve the follower at x and look for a follower optimum that meets
    /// the coupling rows (first with x fixed, then letting the non-linking
    /// leader variables move). `own` is the master's full point, checked
    /// directly when its linking values were seen before.
    fn evaluate(&mut self, x: &[f64], method: &str) -> Eval {
        let m = self.m;
        let mut x = x.to_vec();
        round_integers(m, &mut x);
        let key = self.key(&x);
        if let Some(prev) = self.seen.get(&key) {
            // The master may have picked a different y at known linking
            // values: it is feasible if the follower cannot beat it.
            if let Some(resp) = prev {
                let ll = m.lower_obj(&x);
                if ll <= resp.value + 1e-6 * (1.0 + resp.value.abs())
                    && gap_ok(ll, resp.bound, self.split.ll_integral, self.opts.tolerance)
                {
                    if let Some(f) = self.finish(x.clone(), method, false, false) {
                        return Eval::Point(Box::new(f));
                    }
                }
            }
            return Eval::Seen;
        }
        self.current.leaders += 1;
        let ft = self.opts.follower_time.min(self.left());
        let tol = self.opts.tolerance;
        let resp = follower_response_with(m, &x, ft, tol);
        let proven = matches!(resp.status, MilpStatus::Optimal | MilpStatus::Feasible)
            && gap_ok(resp.value, resp.bound, self.split.ll_integral, tol);
        if !proven {
            let why = format!("follower {:?} after {:.1}s", resp.status, resp.secs);
            self.log(&why);
            self.seen.insert(key, None);
            return Eval::Unresolved(why, None);
        }
        self.seen.insert(key, Some(resp.clone()));
        let phi = resp.reference(tol);
        // (If the master's own y is a follower optimum, the first best
        // response below has it as a feasible point.)
        let has_free = self
            .m
            .vars
            .iter()
            .enumerate()
            .any(|(j, v)| !v.lower && !self.split.linking[j] && v.lb < v.ub);
        let mut last = String::new();
        for free in [false, true] {
            if free && !has_free {
                break;
            }
            let t = self.opts.follower_time.min(self.left());
            if t <= 0.0 {
                break;
            }
            let b = self.best_response(&x, phi, free, t);
            match b.status {
                MilpStatus::Optimal | MilpStatus::Feasible => {
                    let tied = !free && b.status == MilpStatus::Optimal;
                    if let Some(f) = self.finish(b.x, method, free, tied) {
                        return Eval::Point(Box::new(f));
                    }
                    last = "best response failed certification".into();
                }
                MilpStatus::Infeasible => last = "blocked".into(),
                s => last = format!("best response {s:?}"),
            }
        }
        if last == "blocked" {
            self.current.blocked += 1;
            let key = self.key(&x);
            self.blocked.insert(key);
            Eval::Blocked(Box::new(resp))
        } else {
            Eval::Unresolved(last, Some(Box::new(resp)))
        }
    }

    /// The point the search makes of `val`'s leader values: the follower
    /// solved there, then the optimistic tie-break with every leader variable
    /// fixed or, when no follower response within the tolerance meets the
    /// coupling rows that way, with only the linking variables fixed (the
    /// other leader variables re-optimised; the follower's problem is the
    /// same). `bobi coupled-tighten` uses it to re-derive a solution file's
    /// point under [`Tolerance::Strict`].
    pub fn at_leader(&mut self, val: &[f64]) -> Result<Found, String> {
        self.current = MethodReport {
            name: "file".into(),
            ..Default::default()
        };
        match self.evaluate(val, "file") {
            Eval::Point(f) => Ok(*f),
            Eval::Blocked(r) => Err(format!(
                "blocked: no follower response within the tolerance of {} meets the coupling rows, with only the linking variables fixed either",
                r.value
            )),
            Eval::Unresolved(why, _) => Err(why),
            Eval::Seen => Err("leader values seen before".into()),
        }
    }

    fn is_blocked(&self, x: &[f64]) -> bool {
        let mut x = x.to_vec();
        round_integers(self.m, &mut x);
        self.blocked.contains(&self.key(&x))
    }

    /// True when value-function cuts are exact (valid for every bilevel
    /// feasible point): every follower row's linking part takes integer
    /// values, so "y^ violates the row" has an exact integer threshold, and
    /// every big-M comes from finite bounds.
    fn vcuts_exact(&self) -> bool {
        let m = self.m;
        let fmax_finite = m
            .vars
            .iter()
            .filter(|v| v.lower && v.ll_obj != 0.0)
            .all(|v| {
                if v.ll_obj > 0.0 {
                    v.ub.is_finite()
                } else {
                    v.lb.is_finite()
                }
            });
        fmax_finite
            && m.rows.iter().filter(|r| r.lower).all(|r| {
                r.coefs
                    .iter()
                    .filter(|&&(j, _)| !m.vars[j].lower)
                    .all(|&(j, a)| {
                        let v = &m.vars[j];
                        v.integer && a == a.round() && v.lb.is_finite() && v.ub.is_finite()
                    })
            })
    }

    /// The HPR with objective coefficients `obj`, extra rows and extra
    /// binary columns added by `extend`, solved within `time`. The caller
    /// counts it in `current.masters`.
    fn master(
        &self,
        obj: &dyn Fn(usize) -> f64,
        extend: &dyn Fn(&mut Milp, &[usize]),
        time: f64,
        start: Option<&[f64]>,
    ) -> MilpResult {
        let m = self.m;
        let none = vec![None; m.vars.len()];
        let mut sub = submodel(m, &none, |_| true, obj);
        let n0 = sub.mp.obj.len();
        extend(&mut sub.mp, &sub.col);
        let mut o = SolveOpts::new(time);
        o.rel_gap = 1e-4;
        if let Some(s) = start {
            if sub.mp.obj.len() == n0 {
                o.start = Some(s.to_vec());
            }
        }
        let mut r = sub.mp.solve_with(&o);
        if matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
            r.x = expand(&sub, &r.x[..n0], &vec![0.0; m.vars.len()]);
        }
        r
    }

    /// Run the configured methods in order, sharing the budget.
    pub fn run(&mut self) {
        let m = self.m;
        let int_link = (0..m.vars.len()).any(|j| self.split.linking[j] && m.vars[j].integer);
        let cont_link = (0..m.vars.len()).any(|j| self.split.linking[j] && !m.vars[j].integer);
        // Skip methods that cannot apply, so their share goes to the others.
        let methods: Vec<Method> = self
            .opts
            .methods
            .iter()
            .copied()
            .filter(|&k| match k {
                Method::NoGood => int_link,
                Method::Perturb => cont_link,
                _ => true,
            })
            .collect();
        // Open-ended cut loops get larger shares than one-shot methods.
        let weight = |m: Method| match m {
            Method::Hpr | Method::Penalty | Method::Padm => 1.0,
            Method::Perturb => 0.5,
            Method::Follow | Method::NoGood => 2.0,
            Method::VCut => 3.0,
        };
        for (k, &meth) in methods.iter().enumerate() {
            if self.left() <= 1.0 || (self.best.is_some() && !self.opts.improve) {
                break;
            }
            // Each method gets its weight's share of what is left, so later
            // methods inherit the time earlier ones did not use.
            let rest: f64 = methods[k..].iter().map(|&m| weight(m)).sum();
            let share = self.left() * weight(meth) / rest;
            let until = self.elapsed() + share;
            self.current = MethodReport {
                name: meth.name().to_string(),
                ..Default::default()
            };
            let t0 = self.elapsed();
            self.log(&format!("method {} with {:.0}s", meth.name(), share));
            let end = match meth {
                Method::Hpr => self.run_hpr(),
                Method::NoGood => self.run_nogood(until),
                Method::Penalty => self.run_penalty(until),
                Method::VCut => self.run_vcut(until),
                Method::Padm => self.run_padm(until),
                Method::Perturb => self.run_perturb(until),
                Method::Follow => self.run_follow(until),
            };
            self.current.secs = self.elapsed() - t0;
            self.current.end = end;
            self.log(&format!(
                "method {} ended: {} ({} masters, {} leaders, {} blocked)",
                meth.name(),
                self.current.end,
                self.current.masters,
                self.current.leaders,
                self.current.blocked
            ));
            self.reports.push(std::mem::take(&mut self.current));
            if self.hpr_status == Some(MilpStatus::Infeasible) || self.proven_infeasible {
                break;
            }
        }
    }

    /// Record a point; returns true when the search should stop.
    fn record(&mut self, f: Found) -> bool {
        self.log(&format!(
            "point by {}: upper {} lower {} (free leader {})",
            f.method, f.cert.upper_obj, f.cert.lower_obj, f.free_leader
        ));
        self.current.found = true;
        let better = self
            .best
            .as_ref()
            .is_none_or(|b| f.cert.upper_obj < b.cert.upper_obj - 1e-9);
        if better {
            self.best = Some(f);
        }
        !self.opts.improve
    }

    /// Handle an evaluation; returns Some(reason) when the method should stop.
    fn handle(&mut self, e: Eval) -> (Option<String>, Option<Response>) {
        match e {
            Eval::Point(f) => {
                if self.record(*f) {
                    (Some("point".into()), None)
                } else {
                    (None, None)
                }
            }
            Eval::Blocked(r) => (None, Some(*r)),
            Eval::Unresolved(why, r) => {
                self.log(&format!("leader unresolved: {why}"));
                (None, r.map(|b| *b))
            }
            Eval::Seen => (None, None),
        }
    }

    fn hpr_point(&mut self) -> Result<Vec<f64>, String> {
        if let Some(x) = &self.hpr0 {
            return Ok(x.clone());
        }
        if self.hpr_status == Some(MilpStatus::Infeasible) {
            return Err("HPR infeasible".into());
        }
        // A retry after a fruitless first attempt gets half of what is left.
        let cap = if self.hpr_status.is_some() {
            self.left() / 2.0
        } else {
            self.master_cap()
        };
        let m = self.m;
        let r = self.master(&|j| m.vars[j].obj, &|_, _| {}, cap, None);
        self.current.masters += 1;
        self.hpr_status = Some(r.status);
        self.log(&format!("HPR {:?} obj {}", r.status, r.obj));
        match r.status {
            MilpStatus::Optimal | MilpStatus::Feasible => {
                self.hpr0 = Some(r.x.clone());
                Ok(r.x)
            }
            MilpStatus::Infeasible => Err("HPR infeasible".into()),
            s => Err(format!("HPR {s:?}")),
        }
    }

    fn run_hpr(&mut self) -> String {
        let x = match self.hpr_point() {
            Ok(x) => x,
            Err(e) => return e,
        };
        let blocked = self.current.blocked;
        let e = self.evaluate(&x, "hpr");
        match self.handle(e) {
            (Some(s), _) => s,
            // `handle` also passes on the response of an unresolved leader
            // (a best response that hit its time limit), so ask the counter.
            (None, Some(_)) if self.current.blocked > blocked => "HPR leader blocked".into(),
            _ => "HPR leader unresolved".into(),
        }
    }

    /// The HPR's point and its follower response (evaluating it under
    /// `method` if no earlier method did).
    fn hpr_evaluated(&mut self, method: &str) -> Result<(Vec<f64>, Option<Response>), String> {
        let x = self.hpr_point()?;
        let mut xr = x.clone();
        round_integers(self.m, &mut xr);
        let key = self.key(&xr);
        if !self.seen.contains_key(&key) {
            let e = self.evaluate(&x, method);
            if let (Some(s), _) = self.handle(e) {
                return Err(s);
            }
        }
        Ok((x, self.seen.get(&key).cloned().flatten()))
    }

    /// No-good cut on integer variables: sum of |x_j - v_j| >= 1.
    fn add_nogood(&self, mp: &mut Milp, col: &[usize], cut: &[(usize, f64)]) {
        let m = self.m;
        let mut terms: Vec<(usize, f64)> = Vec::new();
        let mut rhs = 1.0;
        for &(j, v) in cut {
            let var = &m.vars[j];
            let c = col[j];
            if v <= var.lb + 0.5 {
                terms.push((c, 1.0));
                rhs += var.lb;
            } else if v >= var.ub - 0.5 {
                terms.push((c, -1.0));
                rhs -= var.ub;
            } else {
                // x_j >= v + 1 when up = 1; x_j <= v - 1 when dn = 1.
                let lo = if var.lb.is_finite() { var.lb } else { v - 1e6 };
                let hi = if var.ub.is_finite() { var.ub } else { v + 1e6 };
                let up = mp.add_var(0.0, 0.0, 1.0, true);
                let dn = mp.add_var(0.0, 0.0, 1.0, true);
                mp.add_row(vec![(c, 1.0), (up, -(v + 1.0 - lo))], lo, f64::INFINITY);
                mp.add_row(vec![(c, 1.0), (dn, hi - v + 1.0)], f64::NEG_INFINITY, hi);
                terms.push((up, 1.0));
                terms.push((dn, 1.0));
            }
        }
        mp.add_row(terms, rhs, f64::INFINITY);
    }

    fn run_nogood(&mut self, until: f64) -> String {
        let m = self.m;
        let ints: Vec<usize> = (0..m.vars.len())
            .filter(|&j| self.split.linking[j] && m.vars[j].integer)
            .collect();
        if ints.is_empty() {
            return "no integer linking variables".into();
        }
        let all_int = ints.len() == self.split.linking.iter().filter(|&&l| l).count();
        let mut cuts: Vec<Vec<(usize, f64)>> = Vec::new();
        // The cuts prove infeasibility only if every linking variable is
        // integer and every cut-off choice was shown blocked.
        let mut exact = all_int;
        match self.hpr_evaluated("nogood") {
            Ok((x, _)) => {
                exact &= self.is_blocked(&x);
                cuts.push(ints.iter().map(|&j| (j, x[j].round())).collect())
            }
            Err(e) => return e,
        }
        loop {
            let time = self.master_cap().min(until - self.elapsed());
            if time <= 1.0 {
                return format!("time ({} cuts)", cuts.len());
            }
            let cuts_ref = &cuts;
            let r = self.master(
                &|j| m.vars[j].obj,
                &|mp, col| {
                    for c in cuts_ref {
                        self.add_nogood(mp, col, c);
                    }
                },
                time,
                None,
            );
            self.current.masters += 1;
            match r.status {
                MilpStatus::Optimal | MilpStatus::Feasible => {}
                MilpStatus::Infeasible => {
                    if exact {
                        self.proven_infeasible = true;
                        return format!(
                            "all {} integer linking choices the HPR allows are blocked: no bilevel feasible point",
                            cuts.len()
                        );
                    }
                    return format!(
                        "no-good cuts exhausted the integer linking choices ({} cuts)",
                        cuts.len()
                    );
                }
                s => return format!("master {s:?}"),
            }
            let e = self.evaluate(&r.x, "nogood");
            if let (Some(s), _) = self.handle(e) {
                return s;
            }
            exact &= self.is_blocked(&r.x);
            cuts.push(ints.iter().map(|&j| (j, r.x[j].round())).collect());
        }
    }

    fn run_penalty(&mut self, until: f64) -> String {
        let m = self.m;
        let start = match self.hpr_evaluated("penalty") {
            Ok((x, _)) => Some(x),
            Err(e) if e == "point" => return e,
            Err(_) => None,
        };
        let base = self.rho_scale;
        let ladder = [0.1, 1.0, 10.0, 100.0, 1e3, f64::INFINITY];
        let k = ladder.len();
        for (i, &mult) in ladder.iter().enumerate() {
            let time = ((until - self.elapsed()) / (k - i) as f64).min(self.master_cap());
            if time <= 1.0 {
                return "time".into();
            }
            let obj = |j: usize| {
                let v = &m.vars[j];
                if mult.is_infinite() {
                    // Lexicographic: the follower's objective first.
                    v.ll_obj * base + 1e-4 * v.obj
                } else {
                    v.obj + mult * base * v.ll_obj
                }
            };
            let r = self.master(&obj, &|_, _| {}, time, start.as_deref());
            self.current.masters += 1;
            if !matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
                self.log(&format!("penalty rho={mult}: master {:?}", r.status));
                continue;
            }
            let e = self.evaluate(&r.x, "penalty");
            if let (Some(s), _) = self.handle(e) {
                return s;
            }
        }
        "ladder done".into()
    }

    /// Add the value-function cut for response `c` to a master.
    fn add_vcut(&self, mp: &mut Milp, col: &[usize], c: &VCut) {
        let m = self.m;
        let mut ws: Vec<usize> = Vec::new();
        for r in m.rows.iter().filter(|r| r.lower) {
            let lin: Vec<(usize, f64)> = r
                .coefs
                .iter()
                .filter(|&&(j, _)| !m.vars[j].lower)
                .map(|&(j, a)| (j, a))
                .collect();
            if lin.is_empty() {
                continue;
            }
            let by: f64 = r
                .coefs
                .iter()
                .filter(|&&(j, _)| m.vars[j].lower)
                .map(|&(j, a)| a * c.point[j])
                .sum();
            let (mut amin, mut amax) = (0.0, 0.0);
            let mut integral = true;
            for &(j, a) in &lin {
                let v = &m.vars[j];
                let (p, q) = (a * v.lb, a * v.ub);
                amin += p.min(q);
                amax += p.max(q);
                integral &= v.integer && a == a.round();
            }
            let terms: Vec<(usize, f64)> = lin.iter().map(|&(j, a)| (col[j], a)).collect();
            let eps = |t: f64| 1e-4 * (1.0 + t.abs());
            if r.lo.is_finite() {
                // y^ violates the row's lower side when A x <= thr.
                let t = r.lo - by;
                let thr = if integral {
                    (t - 1e-9).ceil() - 1.0
                } else {
                    t - eps(t)
                };
                if amin <= thr {
                    let big = if amax.is_finite() {
                        amax - thr
                    } else {
                        1e6 * (1.0 + thr.abs())
                    };
                    let w = mp.add_var(0.0, 0.0, 1.0, true);
                    let mut row = terms.clone();
                    row.push((w, big));
                    mp.add_row(row, f64::NEG_INFINITY, thr + big);
                    ws.push(w);
                }
            }
            if r.hi.is_finite() {
                // ... or its upper side when A x >= thr.
                let t = r.hi - by;
                let thr = if integral {
                    (t + 1e-9).floor() + 1.0
                } else {
                    t + eps(t)
                };
                if amax >= thr {
                    let big = if amin.is_finite() {
                        thr - amin
                    } else {
                        1e6 * (1.0 + thr.abs())
                    };
                    let w = mp.add_var(0.0, 0.0, 1.0, true);
                    let mut row = terms.clone();
                    row.push((w, -big));
                    mp.add_row(row, thr - big, f64::INFINITY);
                    ws.push(w);
                }
            }
        }
        // f(y) <= f^ + M * sum(w).
        let mut fmax = 0.0;
        for v in m.vars.iter().filter(|v| v.lower) {
            fmax += (v.ll_obj * v.lb)
                .max(v.ll_obj * v.ub)
                .max(if v.ll_obj == 0.0 {
                    0.0
                } else {
                    f64::NEG_INFINITY
                });
        }
        let big = if fmax.is_finite() {
            (fmax - c.value).max(1.0)
        } else {
            1e6 * (1.0 + c.value.abs())
        };
        let mut row: Vec<(usize, f64)> = m
            .vars
            .iter()
            .enumerate()
            .filter(|(_, v)| v.lower && v.ll_obj != 0.0)
            .map(|(j, v)| (col[j], v.ll_obj))
            .collect();
        for &w in &ws {
            row.push((w, -big));
        }
        let tol = if self.split.ll_integral {
            0.0
        } else {
            1e-7 * (1.0 + c.value.abs())
        };
        mp.add_row(row, f64::NEG_INFINITY, c.value + tol);
    }

    fn run_vcut(&mut self, until: f64) -> String {
        let m = self.m;
        let mut cuts: Vec<VCut> = Vec::new();
        // Seed with the HPR point's follower response.
        match self.hpr_evaluated("vcut") {
            Ok((_, Some(r))) => cuts.push(VCut {
                point: r.point,
                value: r.value,
            }),
            Ok((_, None)) => {}
            Err(e) => return e,
        }
        loop {
            let time = self.master_cap().min(until - self.elapsed());
            if time <= 1.0 {
                return format!("time ({} cuts)", cuts.len());
            }
            let cuts_ref = &cuts;
            let r = self.master(
                &|j| m.vars[j].obj,
                &|mp, col| {
                    for c in cuts_ref {
                        self.add_vcut(mp, col, c);
                    }
                },
                time,
                None,
            );
            self.current.masters += 1;
            match r.status {
                MilpStatus::Optimal | MilpStatus::Feasible => {}
                MilpStatus::Infeasible => {
                    if self.vcuts_exact() {
                        self.proven_infeasible = true;
                        return format!(
                            "master infeasible after {} exact cuts: no bilevel feasible point",
                            cuts.len()
                        );
                    }
                    return format!("master infeasible after {} cuts", cuts.len());
                }
                s => return format!("master {s:?} ({} cuts)", cuts.len()),
            }
            let e = self.evaluate(&r.x, "vcut");
            match self.handle(e) {
                (Some(s), _) => return s,
                (None, Some(resp)) => {
                    if resp.value.is_finite() {
                        cuts.push(VCut {
                            point: resp.point,
                            value: resp.value,
                        })
                    }
                }
                (None, None) => {
                    // Seen linking values or no response: nothing new to cut
                    // with; a repeated master point means the loop stalled.
                    let key = self.key(&r.x);
                    match self.seen.get(&key) {
                        Some(Some(prev)) => {
                            // The master's y beat the cut? Re-add a cut at
                            // the stored response to force progress.
                            let c = VCut {
                                point: prev.point.clone(),
                                value: prev.value,
                            };
                            if cuts.iter().any(|k| k.point == c.point) {
                                return "stalled".into();
                            }
                            cuts.push(c);
                        }
                        _ => return "follower unresolved".into(),
                    }
                }
            }
        }
    }

    /// Linear lower bound L(x) = c0 + sum g_j x_j on the follower's LP value
    /// from the duals of its LP relaxation at x (weak duality holds for any
    /// row multipliers when each multiplier's sign picks the matching side).
    fn lp_value_bound(&self, x: &[f64], time: f64) -> Option<(f64, Vec<(usize, f64)>)> {
        let m = self.m;
        let fixed: Vec<Option<f64>> = m
            .vars
            .iter()
            .zip(x)
            .map(|(v, &val)| if v.lower { None } else { Some(val) })
            .collect();
        let mut sub = submodel(m, &fixed, |r| r.lower, |j| m.vars[j].ll_obj);
        for i in sub.mp.integer.iter_mut() {
            *i = false;
        }
        let r = sub.mp.solve_with(&SolveOpts::new(time));
        if r.status != MilpStatus::Optimal || r.row_duals.is_empty() {
            return None;
        }
        // Rows of the submodel follow m's follower rows with follower terms.
        let rows: Vec<&Row> = m
            .rows
            .iter()
            .filter(|r| r.lower && r.coefs.iter().any(|&(j, _)| m.vars[j].lower))
            .collect();
        // HiGHS's sign convention decides which multipliers are the duals;
        // weak duality holds for either, so keep the one tight at x.
        let at =
            |b: &(f64, Vec<(usize, f64)>)| b.0 + b.1.iter().map(|&(j, g)| g * x[j]).sum::<f64>();
        let best = [1.0, -1.0]
            .iter()
            .filter_map(|&sign| self.bound_from_duals(&rows, &r.row_duals, sign))
            .max_by(|a, b| at(a).total_cmp(&at(b)))?;
        let at_x = at(&best);
        if at_x < r.obj - 1e-3 * (1.0 + r.obj.abs()) {
            self.log(&format!(
                "LP dual bound {at_x} below the LP value {}",
                r.obj
            ));
        }
        Some(best)
    }

    /// (constant, linear terms in the linking variables) of the bound from
    /// multipliers sign * duals; None when an infinite side would be needed.
    fn bound_from_duals(
        &self,
        rows: &[&Row],
        duals: &[f64],
        sign: f64,
    ) -> Option<(f64, Vec<(usize, f64)>)> {
        let m = self.m;
        let mut reduced: Vec<f64> = m.vars.iter().map(|v| v.ll_obj).collect();
        let mut c0 = 0.0;
        let mut g: HashMap<usize, f64> = HashMap::new();
        for (r, &d) in rows.iter().zip(duals) {
            let p = sign * d;
            if p.abs() < 1e-12 {
                continue;
            }
            let side = if p > 0.0 { r.lo } else { r.hi };
            if !side.is_finite() {
                return None;
            }
            c0 += p * side;
            for &(j, a) in &r.coefs {
                if m.vars[j].lower {
                    reduced[j] -= p * a;
                } else {
                    *g.entry(j).or_insert(0.0) -= p * a;
                }
            }
        }
        for (j, v) in m.vars.iter().enumerate() {
            if !v.lower {
                continue;
            }
            let s = reduced[j];
            if s.abs() < 1e-12 {
                continue;
            }
            let b = if s > 0.0 { v.lb } else { v.ub };
            if !b.is_finite() {
                return None;
            }
            c0 += s * b;
        }
        let mut g: Vec<(usize, f64)> = g.into_iter().collect();
        g.sort_by_key(|&(j, _)| j);
        Some((c0, g))
    }

    fn run_padm(&mut self, until: f64) -> String {
        let m = self.m;
        let mut x = match self.hpr_evaluated("padm") {
            Ok((x, _)) => x,
            Err(e) => return e,
        };
        let mut rho = self.rho_scale;
        let mut stalls = 0;
        loop {
            let time = self.master_cap().min(until - self.elapsed());
            if time <= 1.0 {
                return "time".into();
            }
            let Some((_, g)) = self.lp_value_bound(&x, time.min(60.0)) else {
                return "follower LP has no usable duals".into();
            };
            let mut gx = vec![0.0; m.vars.len()];
            for &(j, gj) in &g {
                gx[j] = gj;
            }
            // min F + rho (f(y) - L(x)) = F + rho f(y) - rho g x - const.
            let obj = |j: usize| {
                let v = &m.vars[j];
                v.obj + rho * (v.ll_obj - gx[j])
            };
            let r = self.master(&obj, &|_, _| {}, time, Some(&x));
            self.current.masters += 1;
            if !matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
                return format!("master {:?}", r.status);
            }
            let same = self.key(&r.x) == self.key(&x);
            let e = self.evaluate(&r.x, "padm");
            if let (Some(s), _) = self.handle(e) {
                return s;
            }
            if same {
                rho *= 10.0;
                stalls += 1;
                if stalls > 6 {
                    return "stalled".into();
                }
            }
            x = r.x;
        }
    }

    /// Best-response dynamics. With the follower's last response y^ (value
    /// f^ = phi(x)), the leader re-optimises (a) with the follower's integer
    /// variables fixed to y^'s, adapting to the follower's discrete choices,
    /// and (b) over the HPR restricted to f(y) <= f^, i.e. only follower
    /// choices at least as good for the follower as its last response. Then
    /// the follower answers the new x. The chain of f^ never increases:
    /// phi(x') <= f(y') <= f^.
    fn run_follow(&mut self, until: f64) -> String {
        let m = self.m;
        let (mut x, resp) = match self.hpr_evaluated("follow") {
            Ok((x, r)) => (x, r),
            Err(e) => return e,
        };
        let Some(mut resp) = resp else {
            return "no follower response at the HPR's point".into();
        };
        let ll_row = |mp: &mut Milp, col: &[usize], bound: f64| {
            let terms: Vec<(usize, f64)> = m
                .vars
                .iter()
                .enumerate()
                .filter(|(_, v)| v.lower && v.ll_obj != 0.0)
                .map(|(j, v)| (col[j], v.ll_obj))
                .collect();
            mp.add_row(terms, f64::NEG_INFINITY, bound);
        };
        let mut rounds = 0;
        loop {
            rounds += 1;
            let tol = if self.split.ll_integral {
                0.0
            } else {
                1e-7 * (1.0 + resp.value.abs())
            };
            let f_hat = resp.value + tol;
            let yhat = resp.point.clone();
            let mut next: Option<Vec<f64>> = None;
            // (a) Fix the follower's integer choices.
            let time = self.master_cap().min((until - self.elapsed()) / 2.0);
            if time <= 1.0 {
                return format!("time ({rounds} rounds)");
            }
            let r = self.master(
                &|j| m.vars[j].obj,
                &|mp, col| {
                    for (j, v) in m.vars.iter().enumerate() {
                        if v.lower && v.integer {
                            mp.lb[col[j]] = yhat[j];
                            mp.ub[col[j]] = yhat[j];
                        }
                    }
                },
                time,
                None,
            );
            self.current.masters += 1;
            if matches!(r.status, MilpStatus::Optimal | MilpStatus::Feasible) {
                let e = self.evaluate(&r.x, "follow");
                match self.handle(e) {
                    (Some(s), _) => return s,
                    (None, Some(nr)) => {
                        next = Some(r.x.clone());
                        resp = nr;
                    }
                    _ => {}
                }
            }
            // (b) Only follower choices at least as good as y^.
            let time = self.master_cap().min(until - self.elapsed());
            if time <= 1.0 {
                return format!("time ({rounds} rounds)");
            }
            let r = self.master(
                &|j| m.vars[j].obj,
                &|mp, col| ll_row(mp, col, f_hat),
                time,
                Some(&x),
            );
            self.current.masters += 1;
            match r.status {
                MilpStatus::Optimal | MilpStatus::Feasible => {
                    let e = self.evaluate(&r.x, "follow");
                    match self.handle(e) {
                        (Some(s), _) => return s,
                        // Continue from the better of (a) and (b) for the follower.
                        (None, Some(nr)) if nr.value < resp.value || next.is_none() => {
                            resp = nr;
                            next = Some(r.x.clone());
                        }
                        _ => {}
                    }
                }
                MilpStatus::Infeasible if next.is_none() => {
                    return format!(
                        "no HPR point gives the follower {f_hat} or better ({rounds} rounds)"
                    )
                }
                _ => {}
            }
            match next {
                Some(nx) if self.key(&nx) != self.key(&x) => x = nx,
                _ => return format!("cycled ({rounds} rounds)"),
            }
        }
    }

    /// Move the continuous linking variables from the HPR's point towards
    /// an interior point of the HPR's LP relaxation (integers fixed).
    fn run_perturb(&mut self, until: f64) -> String {
        let m = self.m;
        if !m
            .vars
            .iter()
            .enumerate()
            .any(|(j, v)| self.split.linking[j] && !v.integer)
        {
            return "no continuous linking variables".into();
        }
        let x0 = match self.hpr_evaluated("perturb") {
            Ok((x, _)) => x,
            Err(e) => return e,
        };
        // Interior target: the average of the two extreme points of the
        // HPR's slice (integers fixed at the HPR's values) that minimise and
        // maximise the sum of the continuous linking variables. The slice is
        // convex, so every point between it and the HPR's point keeps the
        // HPR feasible and with it the follower.
        let fixed: Vec<Option<f64>> = m
            .vars
            .iter()
            .enumerate()
            .map(|(j, v)| if v.integer { Some(x0[j]) } else { None })
            .collect();
        let mut ends: Vec<Vec<f64>> = Vec::new();
        for dir in [1.0, -1.0] {
            let sub = submodel(
                m,
                &fixed,
                |_| true,
                |j| if self.split.linking[j] { dir } else { 0.0 },
            );
            let t = (until - self.elapsed()).min(60.0);
            if t <= 1.0 {
                return "time".into();
            }
            let r = sub.mp.solve_with(&SolveOpts::new(t));
            if r.status == MilpStatus::Optimal {
                ends.push(expand(&sub, &r.x, &x0));
            }
        }
        if ends.is_empty() {
            return "no interior direction".into();
        }
        let mut center = vec![0.0; m.vars.len()];
        for e in &ends {
            for (c, v) in center.iter_mut().zip(e) {
                *c += v / ends.len() as f64;
            }
        }
        // Step from the HPR's point towards the centre, then towards each end.
        let mut targets = vec![center];
        targets.extend(ends);
        for target in &targets {
            for &t in &[0.25, 0.5, 0.75, 1.0] {
                if self.elapsed() >= until {
                    return "time".into();
                }
                let x: Vec<f64> = x0
                    .iter()
                    .zip(target)
                    .enumerate()
                    .map(|(j, (&a, &b))| {
                        if m.vars[j].integer {
                            a
                        } else {
                            a + t * (b - a)
                        }
                    })
                    .collect();
                // Points between two points of the slice stay in it, so the
                // HPR stays feasible; the follower decides the rest.
                let e = self.evaluate(&x, "perturb");
                if let (Some(s), _) = self.handle(e) {
                    return s;
                }
            }
        }
        "steps done".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Var;

    fn var(name: &str, ub: f64, integer: bool, obj: f64, lower: bool, ll_obj: f64) -> Var {
        Var {
            name: name.into(),
            lb: 0.0,
            ub,
            integer,
            obj,
            lower,
            ll_obj,
        }
    }

    /// min -x - y over x in {0..3}; the follower maximises y subject to
    /// y <= x + 1; the leader's coupling row asks y <= 2. The HPR's x = 3
    /// lets the follower take y = 4, and only x <= 1 works: x = 1, y = 2.
    fn toy(integer: bool) -> Bilevel {
        Bilevel {
            name: "toy".into(),
            vars: vec![
                var("x", 3.0, integer, -1.0, false, 0.0),
                var("y", 10.0, integer, -1.0, true, -1.0),
            ],
            rows: vec![
                Row {
                    name: "follower".into(),
                    lo: f64::NEG_INFINITY,
                    hi: 1.0,
                    lower: true,
                    coefs: vec![(1, 1.0), (0, -1.0)],
                },
                Row {
                    name: "coupling".into(),
                    lo: f64::NEG_INFINITY,
                    hi: 2.0,
                    lower: false,
                    coefs: vec![(1, 1.0)],
                },
            ],
            obj_offset: 0.0,
        }
    }

    fn search(m: &Bilevel, methods: Vec<Method>) -> Option<Found> {
        let mut s = Search::new(
            m,
            SearchOpts {
                time: 30.0,
                follower_time: 10.0,
                master_frac: 0.3,
                methods,
                log: false,
                improve: false,
                tolerance: Tolerance::Bobilib,
            },
        );
        s.run();
        s.best
    }

    /// The follower minimises y subject to y >= x (x = 1e4 at the leader's
    /// values, so phi = 1e4); the leader minimises -y - z, z is a leader
    /// variable outside the follower's rows, and the coupling row y >= z.
    /// BOBILib's tolerance lets y sit 1e-7 (1 + 1e4) = 1e-3 above phi.
    fn slack_toy() -> Bilevel {
        Bilevel {
            name: "slack".into(),
            vars: vec![
                var("x", 2e4, false, 0.0, false, 0.0),
                var("z", 2e4, false, -1.0, false, 0.0),
                var("y", 2e4, false, -1.0, true, 1.0),
            ],
            rows: vec![
                Row {
                    name: "follower".into(),
                    lo: 0.0,
                    hi: f64::INFINITY,
                    lower: true,
                    coefs: vec![(2, 1.0), (0, -1.0)],
                },
                Row {
                    name: "coupling".into(),
                    lo: 0.0,
                    hi: f64::INFINITY,
                    lower: false,
                    coefs: vec![(2, 1.0), (1, -1.0)],
                },
            ],
            obj_offset: 0.0,
        }
    }

    fn at_leader(m: &Bilevel, val: &[f64], tolerance: Tolerance) -> Result<Found, String> {
        let opts = SearchOpts {
            time: 60.0,
            follower_time: 10.0,
            master_frac: 0.3,
            methods: vec![],
            log: false,
            improve: false,
            tolerance,
        };
        Search::new(m, opts).at_leader(val)
    }

    #[test]
    fn strict_mode_leaves_the_leader_no_follower_slack() {
        let m = slack_toy();
        // z = 1e4 + 5e-4 needs y above the follower's optimum.
        let file = [1e4, 1e4 + 5e-4, 0.0];
        let loose = at_leader(&m, &file, Tolerance::Bobilib).unwrap();
        assert!(!loose.free_leader && loose.cert.ok);
        assert!(loose.values[2] >= 1e4 + 5e-4 - 1e-9, "{:?}", loose.values);
        // That point fails the strict certificate ...
        let c = certify_with(&m, &loose.values, 10.0, Tolerance::Strict);
        assert!(!c.ok && c.slack > 1e-4, "{c:?}");
        // ... and strict mode re-optimises z with x fixed: y = z = phi.
        let tight = at_leader(&m, &file, Tolerance::Strict).unwrap();
        assert!(tight.free_leader && tight.cert.ok, "{:?}", tight.cert);
        assert_eq!(tight.cert.tolerance, Tolerance::Strict);
        assert!(tight.cert.slack <= STRICT_ABS, "{:?}", tight.cert);
        assert!((tight.values[2] - 1e4).abs() <= STRICT_ABS);
        assert!(tight.values[1] <= tight.values[2] + ROW_TOL);
        // Without the coupling row's pull the leader values stay fixed.
        let tight = at_leader(&m, &[1e4, 0.0, 0.0], Tolerance::Strict).unwrap();
        assert!(!tight.free_leader && tight.cert.ok);
        assert_eq!(tight.values[1], 0.0);
    }

    #[test]
    fn hpr_alone_is_blocked() {
        let m = toy(true);
        assert_eq!(Split::new(&m).coupling, vec![1]);
        assert!(search(&m, vec![Method::Hpr]).is_none());
    }

    #[test]
    fn value_function_cuts_find_the_optimum() {
        let f = search(&toy(true), vec![Method::Hpr, Method::VCut]).unwrap();
        assert_eq!(f.method, "vcut");
        assert!(f.cert.ok);
        assert_eq!(f.values, vec![1.0, 2.0]);
        assert_eq!(f.cert.upper_obj, -3.0);
    }

    #[test]
    fn nogood_cuts_find_a_point() {
        let f = search(&toy(true), vec![Method::NoGood]).unwrap();
        assert!(f.cert.ok);
        assert_eq!(f.values, vec![1.0, 2.0]);
    }

    #[test]
    fn continuous_linking_variable() {
        // With x continuous every x <= 1 works; the best is x = 1, y = 2.
        // (Value-function cuts only shave off a sliver of x per round here.)
        let m = toy(false);
        for meth in [Method::Padm, Method::Perturb] {
            let f = search(&m, vec![meth]);
            let f = f.unwrap_or_else(|| panic!("{} found nothing", meth.name()));
            assert!(f.cert.ok);
            assert!(f.values[0] <= 1.0 + 1e-6 && f.values[1] >= f.values[0] + 1.0 - 1e-6);
        }
    }

    #[test]
    fn certificate_rejects_a_suboptimal_follower() {
        let m = toy(true);
        let c = certify(&m, &[1.0, 1.0], 10.0);
        assert!(!c.ok && c.max_violation == 0.0 && c.phi_bound == -2.0);
        assert!(certify(&m, &[1.0, 2.0], 10.0).ok);
    }

    #[test]
    fn gap_certificate() {
        assert!(gap_certified(100.0, 100.0, false));
        assert!(gap_certified(100.0, 99.9995, false));
        assert!(!gap_certified(100.0, 99.9, false));
        assert!(gap_certified(0.0, -1e-7, true));
        assert!(!gap_certified(0.0, -1e-7, false));
        assert!(!gap_certified(1.0, -1.0, false));
        // Strict: absolute 1e-6 whatever the size of the value.
        let s = Tolerance::Strict;
        assert!(gap_ok(1e8 + 5e-7, 1e8, false, s));
        assert!(!gap_ok(1e8 + 1.0, 1e8, false, s));
        assert!(gap_certified(1e8 + 1.0, 1e8, false));
        assert!(gap_ok(7.0, 6.9999999, true, s));
        assert!(!gap_ok(7.0, f64::NEG_INFINITY, false, s));
    }
}
