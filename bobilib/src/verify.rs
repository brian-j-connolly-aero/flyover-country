//! A bilevel solution verifier modelled on BOBILib's feasibility checker
//! (Section 5.2 of the BOBILib paper). A solution file passes when
//!
//! 1. its point satisfies every row, bound and integrality requirement of the
//!    high-point relaxation within [`FEAS_TOL`] (absolute),
//! 2. the upper-level objective of that point matches the value the file
//!    states, and
//! 3. its follower part is optimal for the follower problem with the leader
//!    part fixed: `|phi_sol - phi_check| / (1e-10 + |phi_check|) <= OPT_TOL`,
//!    where phi is the follower objective in BOBILib's (minimising) sense
//!    (differences below [`NOISE`] count as equal).
//!
//! With [`Tight`] tolerances (`bobi verify --tight`) step 3 asks instead
//! that `phi_sol` lie within `max(abs, rel |phi|)` of every optimum phi the
//! solver's proof allows (the follower MILP solved with zero relative gap
//! and absolute gap 1e-9, at HiGHS's default feasibility tolerance 1e-7,
//! finer than the 1e-6 the file's own rows get); the report keeps BOBILib's
//! verdict on the same solve beside it.
//!
//! The follower problem keeps only the follower's rows, as BOBILib's checker
//! does: upper-level rows, coupling ones included, constrain the leader.
//! Packing interdiction instances get an exact oracle (or a MILP with a proof
//! flag when the follower has several knapsacks); any other instance is
//! solved as a MILP built from the model, under a time limit, so its verdict
//! can stay unproven.

use crate::milp::{Milp, MilpStatus, SolveOpts};
use crate::model::Bilevel;
use crate::solution::SolutionFile;
use crate::structure;
use std::time::Instant;

/// Absolute tolerance on rows, bounds and integrality.
pub const FEAS_TOL: f64 = 1e-6;
/// Relative tolerance on the follower's objective.
pub const OPT_TOL: f64 = 1e-4;
/// Follower values this close are equal whatever their size: near zero the
/// relative test's 1e-10 guard would otherwise flag a solver's 1e-14
/// rounding residue, far below the 1e-6 gap its optimality proof allows.
pub const NOISE: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Yes,
    No,
    Unproven,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Yes => "yes",
            Verdict::No => "no",
            Verdict::Unproven => "unproven",
        }
    }
}

/// Tight follower tolerances: absolute and relative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tight {
    pub abs: f64,
    pub rel: f64,
}

impl Default for Tight {
    fn default() -> Self {
        Tight {
            abs: 1e-6,
            rel: 1e-9,
        }
    }
}

impl Tight {
    fn tol(&self, phi: f64) -> f64 {
        self.abs.max(self.rel * phi.abs())
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    /// Seconds for each follower MILP.
    pub time_limit: f64,
    pub threads: i32,
    /// Build the follower MILP from the model even when an exact oracle
    /// applies, to cross-check one against the other.
    pub generic: bool,
    /// The tight follower test in place of BOBILib's relative 1e-4.
    pub tight: Option<Tight>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            time_limit: 60.0,
            threads: 1,
            generic: false,
            tight: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Report {
    pub name: String,
    /// Problems with the file itself (unknown or misplaced variables,
    /// non-numeric values). Any of them makes the point infeasible.
    pub file_errors: Vec<String>,
    /// Largest violation of rows, bounds and integrality, and where.
    pub violation: f64,
    pub violation_at: String,
    /// Upper-level objective of the file's point, and the value it states.
    pub objective: f64,
    pub file_objective: Option<f64>,
    pub follower: Verdict,
    /// BOBILib's verdict (relative 1e-4) from the same follower solve; equal
    /// to `follower` unless the tolerances are tight.
    pub follower_bobilib: Verdict,
    /// Follower objective (minimised) of the file's point.
    pub phi_sol: f64,
    /// Best follower value the oracle found, and a proven lower bound on
    /// the follower's optimum.
    pub phi_check: Option<f64>,
    pub phi_bound: Option<f64>,
    /// Which follower oracle ran: `kp`, `mkp-milp`, `milp` or `none`.
    pub oracle: &'static str,
    pub note: String,
    pub seconds: f64,
}

impl Report {
    pub fn feasible(&self) -> bool {
        self.file_errors.is_empty() && self.violation <= FEAS_TOL
    }

    pub fn objective_matches(&self) -> bool {
        self.file_objective
            .is_some_and(|f| (f - self.objective).abs() <= FEAS_TOL * self.objective.abs().max(1.0))
    }

    pub fn passed(&self) -> bool {
        self.feasible() && self.objective_matches() && self.follower == Verdict::Yes
    }

    /// Passes BOBILib's tolerance-based test (rows, objective, follower
    /// within relative 1e-4) on the same solve.
    pub fn benchmark_accepted(&self) -> bool {
        self.feasible() && self.objective_matches() && self.follower_bobilib == Verdict::Yes
    }

    /// File errors, then the other notes, for display.
    pub fn notes(&self) -> String {
        let mut all = self.file_errors.clone();
        if !self.note.is_empty() {
            all.push(self.note.clone());
        }
        all.join("; ")
    }
}

/// True when the file carries a point to check; BOBILib writes `null`
/// decisions and no objective for instances without a known solution.
pub fn has_solution(sol: &SolutionFile) -> bool {
    sol.status.as_deref() != Some("infeasible")
        && (sol.objective_value.is_some()
            || !sol.upper_level_decisions.is_empty()
            || !sol.lower_level_decisions.is_empty())
}

pub fn verify(m: &Bilevel, sol: &SolutionFile, opts: &Options) -> Report {
    let start = Instant::now();
    let file_errors = file_errors(m, sol);
    let val = sol.values(m);
    let (violation, violation_at) = m.max_violation(&val);
    let phi_sol = m.lower_obj(&val);
    let f = follower_check(m, &val, phi_sol, opts);
    let mut notes = Vec::new();
    if sol.name != m.name {
        notes.push(format!("file names instance {}", sol.name));
    }
    if !f.note.is_empty() {
        notes.push(f.note);
    }
    Report {
        name: m.name.clone(),
        file_errors,
        violation,
        violation_at,
        objective: m.upper_obj(&val),
        file_objective: sol.objective_value,
        follower: f.verdict,
        follower_bobilib: f.bobilib,
        phi_sol,
        phi_check: f.best,
        phi_bound: f.bound,
        oracle: f.oracle,
        note: notes.join("; "),
        seconds: start.elapsed().as_secs_f64(),
    }
}

/// [`SolutionFile::values`] reads missing variables as zero and ignores
/// names it does not know, so check the names separately.
fn file_errors(m: &Bilevel, sol: &SolutionFile) -> Vec<String> {
    let index = m.var_index();
    let mut errors = Vec::new();
    for (map, lower) in [
        (&sol.upper_level_decisions, false),
        (&sol.lower_level_decisions, true),
    ] {
        for (name, v) in map {
            match index.get(name.as_str()) {
                None => errors.push(format!("unknown variable {name}")),
                Some(&j) if m.vars[j].lower != lower => {
                    errors.push(format!("variable {name} is listed at the wrong level"))
                }
                Some(_) if !v.as_f64().is_some_and(f64::is_finite) => {
                    errors.push(format!("variable {name} has value {v}"))
                }
                Some(_) => {}
            }
        }
    }
    errors
}

struct FollowerCheck {
    verdict: Verdict,
    bobilib: Verdict,
    best: Option<f64>,
    bound: Option<f64>,
    oracle: &'static str,
    note: String,
}

fn follower_check(m: &Bilevel, val: &[f64], phi_sol: f64, opts: &Options) -> FollowerCheck {
    if !opts.generic {
        if let Ok(pi) = structure::packing_interdiction(m) {
            // Leader values off {0, 1} already fail the integrality check.
            let x: Vec<bool> = pi.x_var.iter().map(|&j| val[j] > 0.5).collect();
            let r = pi.follower(&x, opts.time_limit);
            // The follower maximises profit; BOBILib minimises its negation.
            let best = -(r.value as f64);
            let bound = (r.bound != i64::MAX).then_some(-(r.bound as f64));
            let (verdict, bobilib) = verdicts(phi_sol, Some(best), bound, r.proven, opts.tight);
            return FollowerCheck {
                verdict,
                bobilib,
                best: Some(best),
                bound,
                oracle: match pi.foll_w.len() {
                    0 => "none",
                    1 => "kp",
                    _ => "mkp-milp",
                },
                note: if r.proven {
                    String::new()
                } else {
                    "follower optimum not proven".into()
                },
            };
        }
    }
    let fail = |note: String| FollowerCheck {
        verdict: Verdict::No,
        bobilib: Verdict::No,
        best: None,
        bound: None,
        oracle: "milp",
        note,
    };
    let mp = match follower_milp(m, val) {
        Ok(mp) => mp,
        Err(why) => return fail(why),
    };
    if mp.obj.is_empty() {
        // No follower variables: the follower's value is a constant.
        let (verdict, bobilib) = verdicts(phi_sol, Some(0.0), Some(0.0), true, opts.tight);
        return FollowerCheck {
            verdict,
            bobilib,
            best: Some(0.0),
            bound: Some(0.0),
            oracle: "none",
            note: String::new(),
        };
    }
    let mut so = SolveOpts::new(opts.time_limit);
    so.threads = opts.threads;
    if opts.tight.is_some() {
        so.abs_gap = 1e-9;
    }
    let r = mp.solve_with(&so);
    let mut bound = r.bound.is_finite().then_some(r.bound);
    if opts.tight.is_some() && integral_follower(m) {
        // The optimum is an integer at least the bound.
        bound = bound.map(|b| (b - 1e-6).ceil());
    }
    match r.status {
        MilpStatus::Optimal | MilpStatus::Feasible => {
            let proven = r.status == MilpStatus::Optimal;
            let bound = if proven { bound.or(Some(r.obj)) } else { bound };
            let (verdict, bobilib) = verdicts(phi_sol, Some(r.obj), bound, proven, opts.tight);
            FollowerCheck {
                verdict,
                bobilib,
                best: Some(r.obj),
                bound,
                oracle: "milp",
                note: if proven {
                    String::new()
                } else {
                    "follower MILP stopped at the time limit".into()
                },
            }
        }
        MilpStatus::Infeasible => fail("follower problem infeasible".into()),
        MilpStatus::Unbounded => fail("follower problem unbounded".into()),
        MilpStatus::InfeasibleOrUnbounded => {
            fail("follower problem infeasible or unbounded".into())
        }
        MilpStatus::Unknown => FollowerCheck {
            verdict: Verdict::Unproven,
            bobilib: Verdict::Unproven,
            best: None,
            bound,
            oracle: "milp",
            note: "follower MILP found no point".into(),
        },
    }
}

/// The follower's problem once the leader's variables are fixed at `val`:
/// the follower's variables with their bounds and integrality, the
/// follower's rows with the leader's part moved into the row bounds, and
/// the follower's objective, minimised. Fails when a follower row has no
/// follower variable and the leader's values violate it.
pub fn follower_milp(m: &Bilevel, val: &[f64]) -> Result<Milp, String> {
    let mut mp = Milp::new(false);
    let mut col = vec![usize::MAX; m.vars.len()];
    for (j, v) in m.vars.iter().enumerate() {
        if v.lower {
            col[j] = mp.add_var(v.ll_obj, v.lb, v.ub, v.integer);
        }
    }
    for r in m.rows.iter().filter(|r| r.lower) {
        let mut fixed = 0.0;
        let mut coefs = Vec::new();
        for &(j, c) in &r.coefs {
            if m.vars[j].lower {
                coefs.push((col[j], c));
            } else {
                fixed += c * val[j];
            }
        }
        let (lo, hi) = (r.lo - fixed, r.hi - fixed);
        if coefs.is_empty() {
            if lo > FEAS_TOL || hi < -FEAS_TOL {
                return Err(format!(
                    "follower row {} is violated by the leader's values",
                    r.name
                ));
            }
            continue;
        }
        mp.add_row(coefs, lo, hi);
    }
    Ok(mp)
}

/// The follower's objective takes integer values at integer points.
fn integral_follower(m: &Bilevel) -> bool {
    m.vars
        .iter()
        .filter(|v| v.lower)
        .all(|v| v.ll_obj == 0.0 || (v.integer && v.ll_obj == v.ll_obj.round()))
}

/// The verdict under the options' tolerance, and BOBILib's.
fn verdicts(
    phi_sol: f64,
    best: Option<f64>,
    bound: Option<f64>,
    proven: bool,
    tight: Option<Tight>,
) -> (Verdict, Verdict) {
    let bobilib = follower_verdict(phi_sol, best, bound, proven);
    match tight {
        Some(t) => (tight_verdict(phi_sol, best, bound, &t), bobilib),
        None => (bobilib, bobilib),
    }
}

/// The tight test: every optimum phi in [bound, best] the solver's proof
/// allows is within `max(abs, rel |phi|)` of phi_sol. A strictly better
/// response found (best below phi_sol by more), or phi_sol below the proven
/// bound by more, is a definite "no".
pub fn tight_verdict(phi_sol: f64, best: Option<f64>, bound: Option<f64>, t: &Tight) -> Verdict {
    if let Some(b) = best {
        if phi_sol - b > t.tol(b) {
            return Verdict::No;
        }
    }
    let Some(lb) = bound else {
        return Verdict::Unproven;
    };
    if lb - phi_sol > t.tol(lb) {
        return Verdict::No;
    }
    let best_ok = best.is_none_or(|b| b - phi_sol <= t.tol(b));
    if phi_sol - lb <= t.tol(lb) && best_ok {
        Verdict::Yes
    } else {
        Verdict::Unproven
    }
}

/// BOBILib's test `|sol - check| / (1e-10 + |check|) <= OPT_TOL`, up to [`NOISE`].
fn agrees(sol: f64, check: f64) -> bool {
    let d = (sol - check).abs();
    d <= NOISE || d / (1e-10 + check.abs()) <= OPT_TOL
}

/// BOBILib's optimality test, extended to a solver that stopped early with
/// a best value `best` and a lower bound `bound` on the (minimised)
/// follower optimum. A strictly better point found is a definite "no". The
/// values that pass the test form an interval around phi_sol, so it holds on
/// all of [bound, phi_sol] once it holds at both ends; this assumes the file's follower part is
/// feasible, which the feasibility check covers.
pub fn follower_verdict(
    phi_sol: f64,
    best: Option<f64>,
    bound: Option<f64>,
    proven: bool,
) -> Verdict {
    if proven {
        if let Some(b) = best {
            return if agrees(phi_sol, b) {
                Verdict::Yes
            } else {
                Verdict::No
            };
        }
    }
    if let Some(b) = best {
        if b < phi_sol && !agrees(phi_sol, b) {
            return Verdict::No;
        }
    }
    match bound {
        Some(lb) if lb <= phi_sol && agrees(phi_sol, lb) => Verdict::Yes,
        _ => Verdict::Unproven,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Row, Var};
    use serde_json::Value;

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

    fn row(name: &str, lo: f64, hi: f64, lower: bool, coefs: &[(usize, f64)]) -> Row {
        Row {
            name: name.into(),
            lo,
            hi,
            lower,
            coefs: coefs.to_vec(),
        }
    }

    const INF: f64 = f64::INFINITY;

    /// Knapsack interdiction with profits 5, 4, 3, follower weights 2 and
    /// capacity 4 (`knapsacks` copies of that row), one unit of leader
    /// budget: the leader blocks item 0 and the follower packs 4 + 3 = 7.
    /// Variables x0..x2 are 0..3 and y0..y2 are 3..6.
    fn kip(knapsacks: usize) -> Bilevel {
        let p = [5.0, 4.0, 3.0];
        let mut vars: Vec<Var> = (0..3)
            .map(|j| var(&format!("x{j}"), 1.0, true, 0.0, false, 0.0))
            .collect();
        vars.extend((0..3).map(|j| var(&format!("y{j}"), 1.0, true, p[j], true, -p[j])));
        let mut rows = vec![row(
            "budget",
            -INF,
            1.0,
            false,
            &[(0, 1.0), (1, 1.0), (2, 1.0)],
        )];
        for k in 0..knapsacks {
            rows.push(row(
                &format!("knap{k}"),
                -INF,
                4.0,
                true,
                &[(3, 2.0), (4, 2.0), (5, 2.0)],
            ));
        }
        for j in 0..3 {
            rows.push(row(
                &format!("block{j}"),
                -INF,
                1.0,
                true,
                &[(j, 1.0), (3 + j, 1.0)],
            ));
        }
        Bilevel {
            name: "tiny-kip".into(),
            vars,
            rows,
            obj_offset: 0.0,
        }
    }

    fn solution(m: &Bilevel, val: &[f64]) -> SolutionFile {
        SolutionFile::from_values(m, "feasible", val)
    }

    fn generic() -> Options {
        Options {
            generic: true,
            ..Options::default()
        }
    }

    #[test]
    fn optimal_kip_point_passes_with_both_oracles() {
        let m = kip(1);
        let s = solution(&m, &[1.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
        let r = verify(&m, &s, &Options::default());
        assert!(r.passed(), "{r:?}");
        assert_eq!(
            (r.oracle, r.objective, r.phi_check),
            ("kp", 7.0, Some(-7.0))
        );
        let g = verify(&m, &s, &generic());
        assert!(g.passed(), "{g:?}");
        assert_eq!(g.oracle, "milp");
    }

    #[test]
    fn lazy_follower_is_not_optimal() {
        let m = kip(1);
        let s = solution(&m, &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        for opts in [Options::default(), generic()] {
            let r = verify(&m, &s, &opts);
            assert!(r.feasible() && r.objective_matches());
            assert_eq!(r.follower, Verdict::No, "{r:?}");
            assert!(!r.passed());
        }
    }

    #[test]
    fn interdicted_item_makes_the_point_infeasible() {
        let m = kip(1);
        let s = solution(&m, &[1.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
        let r = verify(&m, &s, &Options::default());
        assert!(!r.feasible());
        assert_eq!(r.violation, 1.0);
        assert!(r.violation_at.contains("block0"), "{}", r.violation_at);
    }

    #[test]
    fn wrong_objective_and_unknown_names_fail() {
        let m = kip(1);
        let mut s = solution(&m, &[1.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
        s.objective_value = Some(7.5);
        let r = verify(&m, &s, &Options::default());
        assert!(r.feasible() && !r.objective_matches() && !r.passed());

        let mut s = solution(&m, &[1.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
        s.upper_level_decisions.insert("z".into(), Value::from(1.0));
        s.upper_level_decisions.remove("x0");
        s.lower_level_decisions
            .insert("x0".into(), Value::from(1.0));
        let r = verify(&m, &s, &Options::default());
        assert_eq!(r.file_errors.len(), 2, "{:?}", r.file_errors);
        assert!(!r.feasible());
    }

    #[test]
    fn several_knapsacks_use_the_mkp_oracle() {
        let m = kip(2);
        let s = solution(&m, &[1.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
        let r = verify(&m, &s, &Options::default());
        assert!(r.passed(), "{r:?}");
        assert_eq!(r.oracle, "mkp-milp");
    }

    /// Not packing interdiction: an integer leader `x` in [0, 10] and a
    /// continuous follower maximising `y` subject to `y <= x + 1` and
    /// `y <= 5`; the leader pays `x - y`, with a coupling row `x + y >= 1`
    /// that the follower problem must leave out.
    fn continuous() -> Bilevel {
        Bilevel {
            name: "tiny-lp".into(),
            vars: vec![
                var("x", 10.0, true, 1.0, false, 0.0),
                var("y", INF, false, -1.0, true, -1.0),
            ],
            rows: vec![
                row("link", -INF, 1.0, true, &[(1, 1.0), (0, -1.0)]),
                row("cap", -INF, 5.0, true, &[(1, 1.0)]),
                row("coupling", 1.0, INF, false, &[(0, 1.0), (1, 1.0)]),
            ],
            obj_offset: 0.0,
        }
    }

    #[test]
    fn generic_follower_problem() {
        let m = continuous();
        let r = verify(&m, &solution(&m, &[2.0, 3.0]), &Options::default());
        assert!(r.passed(), "{r:?}");
        assert_eq!((r.oracle, r.objective), ("milp", -1.0));
        let r = verify(&m, &solution(&m, &[2.0, 2.5]), &Options::default());
        assert!(r.feasible());
        assert_eq!(r.follower, Verdict::No);
        // Within the relative tolerance: 3 - 2.9999 is 3.3e-5 of 3.
        let r = verify(&m, &solution(&m, &[2.0, 2.9999]), &Options::default());
        assert_eq!(r.follower, Verdict::Yes);
    }

    #[test]
    fn unbounded_follower_is_not_optimal() {
        let mut m = continuous();
        m.rows.truncate(1);
        m.rows[0] = row("link", 0.0, INF, true, &[(1, 1.0), (0, -1.0)]);
        let r = verify(&m, &solution(&m, &[0.0, 0.0]), &Options::default());
        assert!(r.feasible());
        assert_eq!(r.follower, Verdict::No);
        assert!(r.note.contains("unbounded"), "{}", r.note);
    }

    #[test]
    fn verdict_from_bounds() {
        use Verdict::*;
        assert_eq!(follower_verdict(-10.0, Some(-10.0), None, true), Yes);
        assert_eq!(follower_verdict(-10.0, Some(-11.0), None, true), No);
        // A file value better than the proven optimum is wrong too.
        assert_eq!(follower_verdict(-12.0, Some(-11.0), None, true), No);
        assert_eq!(
            follower_verdict(-10.0, Some(-10.0), Some(-12.0), false),
            Unproven
        );
        assert_eq!(
            follower_verdict(-10.0, Some(-10.0), Some(-10.0005), false),
            Yes
        );
        assert_eq!(follower_verdict(-10.0, Some(-11.0), Some(-12.0), false), No);
        assert_eq!(follower_verdict(-10.0, None, None, false), Unproven);
        // Near zero, rounding residue is not a better response; 1e-6 is.
        assert_eq!(follower_verdict(0.0, Some(-1.8e-14), None, true), Yes);
        assert_eq!(follower_verdict(0.0, Some(-1e-6), None, true), No);
    }

    #[test]
    fn tight_verdict_from_bounds() {
        use Verdict::*;
        let t = Tight::default();
        // Absolute 1e-6 for small values, relative 1e-9 for large ones.
        assert_eq!(tight_verdict(-10.0, Some(-10.0), Some(-10.0), &t), Yes);
        assert_eq!(tight_verdict(-10.0 + 5e-7, Some(-10.0), Some(-10.0), &t), Yes);
        assert_eq!(tight_verdict(-10.0 + 1e-5, Some(-10.0), Some(-10.0), &t), No);
        assert_eq!(tight_verdict(-10.0 - 1e-5, Some(-10.0), Some(-10.0), &t), No);
        assert_eq!(tight_verdict(6e7 + 0.05, Some(6e7), Some(6e7), &t), Yes);
        assert_eq!(tight_verdict(6e7 + 6.0, Some(6e7), Some(6e7), &t), No);
        // An open gap: unproven unless phi_sol is within the tolerance of both ends.
        assert_eq!(tight_verdict(-10.0, Some(-10.0), Some(-10.5), &t), Unproven);
        assert_eq!(tight_verdict(-10.0, Some(-10.0), None, &t), Unproven);
        assert_eq!(tight_verdict(-10.0, Some(-11.0), Some(-12.0), &t), No);
    }

    #[test]
    fn tight_mode_rejects_relative_slack() {
        let m = continuous();
        let tight = Options {
            tight: Some(Tight::default()),
            ..Options::default()
        };
        // y = 2.9999 is within BOBILib's relative 1e-4 of 3, not within 1e-6.
        let r = verify(&m, &solution(&m, &[2.0, 2.9999]), &tight);
        assert_eq!((r.follower, r.follower_bobilib), (Verdict::No, Verdict::Yes));
        assert!(!r.passed() && r.benchmark_accepted());
        let r = verify(&m, &solution(&m, &[2.0, 3.0 - 1e-7]), &tight);
        assert!(r.passed(), "{r:?}");
        // The exact oracle under the tight test.
        let k = kip(1);
        let r = verify(&k, &solution(&k, &[1.0, 0.0, 0.0, 0.0, 1.0, 1.0]), &tight);
        assert!(r.passed() && r.oracle == "kp", "{r:?}");
    }

    #[test]
    fn open_instance_file_has_no_solution() {
        let s: SolutionFile = serde_json::from_str(
            r#"{"name": "a", "bilevel_type": "optimistic", "status": "open",
                "difficulty": "hard", "objective_value": null,
                "upper_level_decisions": null, "lower_level_decisions": null}"#,
        )
        .unwrap();
        assert!(!has_solution(&s));
        let m = kip(1);
        assert!(has_solution(&solution(&m, &[0.0; 6])));
    }
}
