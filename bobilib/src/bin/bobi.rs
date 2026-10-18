use bobi::{
    kip::{Kip, SsOrder},
    mps,
    robust_ss::Outcome,
    solution::SolutionFile,
    structure,
};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser)]
#[command(about = "Solvers and tools for BOBILib bilevel instances")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Parse every instance under a directory and report detected structure.
    Survey {
        dir: PathBuf,
        /// Skip MPS files larger than this many bytes (compressed).
        #[arg(long, default_value_t = 50_000_000)]
        max_bytes: u64,
    },
    /// Check a solution file's constraints and objective against an instance.
    Check { mps: PathBuf, solution: PathBuf },
    /// Packing interdiction (one or more budgets and knapsacks) by the cut loop.
    Mkip {
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = 600.0)]
        time: f64,
        #[arg(long, default_value_t = 60.0)]
        follower_time: f64,
        #[arg(long)]
        log: bool,
    },
    /// LP-follower bracket for single-knapsack followers.
    LpBound {
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = 300.0)]
        time: f64,
        /// Seconds of local search from the bracket's leader choice.
        #[arg(long, default_value_t = 0.0)]
        ls_time: f64,
        /// Write certified feasible points here (BOBILib solution format).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Bounds and a certified solution for single-follower-row interdiction:
    /// the tuned scaled online bound, the LP-follower bracket and local search.
    Bounds {
        files: Vec<PathBuf>,
        /// Seconds for the LP-follower MILP.
        #[arg(long, default_value_t = 60.0)]
        milp_time: f64,
        /// Seconds of leader local search.
        #[arg(long, default_value_t = 30.0)]
        ls_time: f64,
        /// Online-bound table entries per item (leader x follower).
        #[arg(long, default_value_t = 4.0e6)]
        cells: f64,
        /// Write certified solutions here.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Upper bounds (follower LP at the LP-follower minimiser) and, when the
    /// follower can be solved exactly, certified points, for any number of
    /// follower rows.
    UpperMulti {
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = 120.0)]
        milp_time: f64,
        #[arg(long, default_value_t = 300.0)]
        follower_time: f64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Feasible points for general instances: HPR, follower, tie-break.
    General {
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = 300.0)]
        hpr_time: f64,
        #[arg(long, default_value_t = 300.0)]
        follower_time: f64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Certify that the follower is unbounded for every leader choice.
    Ray { files: Vec<PathBuf> },
    /// Root online-relaxation bound with merged leader rows and scaling.
    Omega {
        mps: PathBuf,
        #[arg(long, default_value_t = 2000)]
        bu: usize,
        #[arg(long, default_value_t = 4000)]
        cl: usize,
        /// Tune the leader-row multipliers at this coarse resolution first.
        #[arg(long)]
        tune: Option<usize>,
    },
    /// Solve subset-sum knapsack interdiction instances exactly.
    SsSolve {
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = 200_000_000)]
        node_limit: u64,
        #[arg(long, value_enum, default_value_t = SsOrder::Ascending)]
        order: SsOrder,
        /// First try a follower limited to this many copies per weight.
        #[arg(long)]
        max_copies: Option<usize>,
    },
    /// Subset-sum knapsack interdiction: can the leader hold the follower below LO?
    Ss {
        mps: PathBuf,
        /// Threshold; defaults to the greedy heuristic's value.
        #[arg(long)]
        lo: Option<i64>,
        #[arg(long, default_value_t = u64::MAX)]
        node_limit: u64,
        /// Also prune with the robust online table.
        #[arg(long)]
        online: bool,
        #[arg(long, value_enum, default_value_t = SsOrder::Ascending)]
        order: SsOrder,
        /// Limit the follower to this many copies per weight (proves safety only).
        #[arg(long)]
        max_copies: Option<usize>,
    },
}

fn instances(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.to_string_lossy().ends_with(".mps.gz")
                || p.to_string_lossy().ends_with(".mps")
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Ray { files } => {
            for path in files {
                let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
                match bobi::general::follower_unbounded_ray(&m, 300.0) {
                    None => println!("{}\tno follower ray", m.name),
                    Some((d, val)) => {
                        let support = d.iter().filter(|x| x.abs() > 1e-9).count();
                        println!("{}\tfollower unbounded: direction with objective {val:.4} on {support} variables", m.name);
                    }
                }
            }
        }
        Cmd::General {
            files,
            hpr_time,
            follower_time,
            out,
        } => run_general(files, hpr_time, follower_time, out.as_deref()),
        Cmd::UpperMulti {
            files,
            milp_time,
            follower_time,
            out,
        } => run_upper_multi(files, milp_time, follower_time, out.as_deref()),
        Cmd::Bounds {
            files,
            milp_time,
            ls_time,
            cells,
            out,
        } => run_bounds(files, milp_time, ls_time, cells, out.as_deref()),
        Cmd::Survey { dir, max_bytes } => {
            let files = instances(&dir);
            let lines: Vec<String> = files
                .par_iter()
                .map(|p| {
                    let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                    if size > max_bytes {
                        return format!("{}\tSKIP\tcompressed size {size}", p.display());
                    }
                    match mps::read_instance(p) {
                        Err(e) => format!("{}\tERROR\t{e}", p.display()),
                        Ok(m) => match structure::packing_interdiction(&m) {
                            Ok(pi) => format!(
                                "{}\tPACKING\tn={} leader_rows={} follower_rows={} scale={}",
                                m.name,
                                pi.n,
                                pi.lead_w.len(),
                                pi.foll_w.len(),
                                pi.scale
                            ),
                            Err(why) => format!("{}\tOTHER\t{}\t{why}", m.name, m.summary()),
                        },
                    }
                })
                .collect();
            for l in lines {
                println!("{l}");
            }
        }
        Cmd::Mkip {
            files,
            time,
            follower_time,
            log,
        } => {
            for path in files {
                let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
                let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
                let r = pi.cegar(time, follower_time, log);
                println!(
                    "{}\t{}\tub={:?}\tlb={}\titers={}\t{:.1}s\tleader_rows={}\tfollower_rows={}",
                    m.name,
                    if r.proven { "OPTIMAL" } else { "OPEN" },
                    r.ub.map(|u| pi.objective(u)),
                    pi.objective(r.lb),
                    r.iterations,
                    r.seconds,
                    pi.lead_w.len(),
                    pi.foll_w.len()
                );
            }
        }
        Cmd::LpBound {
            files,
            time,
            ls_time,
            out,
        } => {
            for path in files {
                let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
                let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
                let t = Instant::now();
                match pi.lp_follower_bound(time, time) {
                    None => println!("{}\tNA", m.name),
                    Some(mut b) => {
                        if ls_time > 0.0 && b.follower_proven {
                            let (x, f) = pi.local_search(&b.x, ls_time, time);
                            if f.proven && f.value < b.value_at_x {
                                b.x = x;
                                b.y = f.y;
                                b.value_at_x = f.value;
                            }
                        }
                        let pmax = *pi.profit.iter().max().unwrap();
                        let lb = (b.min_lp - (pi.foll_w.len() as i64 * pmax) as f64).max(0.0);
                        println!(
                            "{}\tmin_lp={:.1}\tlb={:.1}\tlp_at_x={:.1}\tvalue_at_x={}\tfollower_proven={}\tfollower_bound={}\tproven_min={}\tfrows={}\t{:.1}s",
                            m.name, b.min_lp, lb, b.lp_at_x, b.value_at_x, b.follower_proven, b.follower_bound, b.proven_min,
                            pi.foll_w.len(), t.elapsed().as_secs_f64()
                        );
                        if let Some(dir) = &out {
                            if b.follower_proven {
                                let val = pi.assignment(m.vars.len(), &b.x, &b.y);
                                let sol = SolutionFile::from_values(&m, "feasible", &val);
                                sol.write(&dir.join(format!("{}.res.json", m.name)))
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
        Cmd::Omega {
            mps: path,
            bu,
            cl,
            tune,
        } => {
            let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
            let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
            // Normalise each leader row by its capacity, then optionally tune.
            let mut mu: Vec<f64> = pi.lead_cap.iter().map(|&c| 1.0 / c.max(1) as f64).collect();
            if let Some(coarse) = tune {
                let (cb, cc) = bobi::mkip::omega_scales((coarse * coarse) as f64);
                let (tuned, v) = bobi::mkip::tune_mu(&pi, cb, cc, 3);
                eprintln!("tuned multipliers at {cb}x{cc}: coarse omega {v}");
                mu = tuned;
            }
            let cap: f64 = pi
                .lead_cap
                .iter()
                .zip(&mu)
                .map(|(&c, &m)| m * c as f64)
                .sum();
            let su = bu as f64 / cap;
            let sl = cl as f64 / pi.foll_cap[0] as f64;
            let t = Instant::now();
            let v = bobi::mkip::omega_root(&pi, &mu, su, sl);
            println!(
                "{}\tomega_root={}\tbu={}\tcl={}\t{:.1}s",
                m.name,
                v,
                bu,
                cl,
                t.elapsed().as_secs_f64()
            );
        }
        Cmd::SsSolve {
            files,
            node_limit,
            order,
            max_copies,
        } => {
            for path in files {
                let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
                let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
                let k = Kip::from_packing(&pi).expect("not a single-row knapsack interdiction");
                if !k.is_subset_sum() {
                    println!("{}\tSKIP\tnot subset-sum", m.name);
                    continue;
                }
                let t = Instant::now();
                let s = k.solve_subset_sum(node_limit, true, order, max_copies);
                println!(
                    "{}\t{}\t{}\tnodes={}\trounds={}\t{:.2}s",
                    m.name,
                    if s.proven { "OPTIMAL" } else { "UNPROVEN" },
                    s.value,
                    s.nodes,
                    s.rounds,
                    t.elapsed().as_secs_f64()
                );
            }
        }
        Cmd::Ss {
            mps: path,
            lo,
            node_limit,
            online,
            order,
            max_copies,
        } => {
            let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
            let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
            let k = Kip::from_packing(&pi).expect("not a single-row knapsack interdiction");
            assert!(k.is_subset_sum(), "profits differ from follower weights");
            let (_, greedy) = k.greedy();
            let lo = lo.unwrap_or(greedy);
            let t = Instant::now();
            let (out, st, classes, g) =
                k.subset_sum_threshold(lo, node_limit, online, order, max_copies);
            let verdict = match &out {
                Outcome::Safe => "SAFE (follower always reaches the threshold)".to_string(),
                Outcome::Blocking(r) => format!("BLOCKED by deleting {:?}", r),
                Outcome::Unknown => "UNKNOWN (node limit)".to_string(),
            };
            println!(
                "{}: n={} CL={} CU={} greedy={} threshold={} classes={} gcd={} -> {} in {:.2}s; {:?}",
                m.name, k.n, k.cl, k.cu, greedy, lo, classes.len(), g, verdict,
                t.elapsed().as_secs_f64(), st
            );
        }
        Cmd::Check {
            mps: path,
            solution,
        } => {
            let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
            let s = SolutionFile::read(&solution).unwrap_or_else(|e| panic!("{e}"));
            let val = s.values(&m);
            let (viol, what) = m.max_violation(&val);
            println!(
                "{}: upper objective {} (file says {:?}), lower objective {}, max violation {viol:.2e} {what}",
                m.name,
                m.upper_obj(&val),
                s.objective_value,
                m.lower_obj(&val)
            );
        }
    }
}

/// Bounds and a certified solution for single-follower-row packing
/// interdiction, one JSON line per instance.
fn run_bounds(files: Vec<PathBuf>, milp_time: f64, ls_time: f64, cells: f64, out: Option<&Path>) {
    for path in files {
        let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
        let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
        if pi.foll_w.len() != 1 {
            println!(
                "{{\"name\":\"{}\",\"skip\":\"follower has {} rows\"}}",
                m.name,
                pi.foll_w.len()
            );
            continue;
        }
        let t = Instant::now();
        let (cb, cc) = bobi::mkip::omega_scales(cells / 9.0);
        let (mu, _) = bobi::mkip::tune_mu(&pi, cb, cc, 2);
        let (bu, cl) = bobi::mkip::omega_scales(cells);
        let cap: f64 = pi
            .lead_cap
            .iter()
            .zip(&mu)
            .map(|(&c, &w)| w * c as f64)
            .sum();
        let omega =
            bobi::mkip::omega_root(&pi, &mu, bu as f64 / cap, cl as f64 / pi.foll_cap[0] as f64);
        let t_omega = t.elapsed().as_secs_f64();
        let pmax = *pi.profit.iter().max().unwrap();
        let (mut lb_milp, mut x, mut y, mut ub) =
            (0i64, vec![false; pi.n], vec![false; pi.n], i64::MAX);
        if let Some(b) = pi.lp_follower_bound(milp_time, 60.0) {
            lb_milp = (b.min_lp - pmax as f64).ceil() as i64;
            if b.follower_proven {
                x = b.x;
                y = b.y;
                ub = b.value_at_x;
            }
        }
        if ub == i64::MAX {
            x = pi.greedy_leader(30.0);
            let f = pi.follower(&x, 60.0);
            y = f.y;
            ub = f.value;
        }
        let (lx, lf) = pi.local_search(&x, ls_time, 60.0);
        if lf.proven && lf.value < ub {
            x = lx;
            y = lf.y;
            ub = lf.value;
        }
        assert!(pi.leader_feasible(&x) && pi.follower_feasible(&x, &y));
        let lb = omega.max(lb_milp);
        println!(
            "{{\"name\":\"{}\",\"n\":{},\"leader_rows\":{},\"lb\":{},\"lb_omega\":{},\"lb_milp\":{},\"ub\":{},\"gap\":{:.5},\"omega_secs\":{:.1},\"secs\":{:.1}}}",
            m.name,
            pi.n,
            pi.lead_w.len(),
            lb,
            omega,
            lb_milp,
            ub,
            (ub - lb) as f64 / ub.max(1) as f64,
            t_omega,
            t.elapsed().as_secs_f64()
        );
        if let Some(dir) = out {
            let val = pi.assignment(m.vars.len(), &x, &y);
            let status = if lb >= ub { "optimal" } else { "feasible" };
            SolutionFile::from_values(&m, status, &val)
                .write(&dir.join(format!("{}.res.json", m.name)))
                .unwrap();
        }
    }
}

/// For each instance: the leader choice minimising the follower's LP value,
/// the follower's LP value there (a valid upper bound on the optimum) and,
/// if the follower's MILP is solved to optimality, a certified point.
fn run_upper_multi(files: Vec<PathBuf>, milp_time: f64, follower_time: f64, out: Option<&Path>) {
    for path in files {
        let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
        let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
        let t = Instant::now();
        let Some(b) = pi.lp_follower_bound(milp_time, follower_time) else {
            println!("{{\"name\":\"{}\",\"error\":\"no leader choice\"}}", m.name);
            continue;
        };
        let ub_lp = pi
            .follower_lp(&b.x, 60.0)
            .map(|v| (v + 1e-6).floor() as i64);
        println!(
            "{{\"name\":\"{}\",\"n\":{},\"follower_rows\":{},\"min_lp_bound\":{:.1},\"ub_lp\":{},\"follower_value\":{},\"follower_bound\":{},\"certified\":{},\"secs\":{:.1}}}",
            m.name,
            pi.n,
            pi.foll_w.len(),
            b.min_lp,
            ub_lp.map_or("null".to_string(), |v| v.to_string()),
            b.value_at_x,
            if b.follower_bound == i64::MAX { "null".to_string() } else { b.follower_bound.to_string() },
            b.follower_proven,
            t.elapsed().as_secs_f64()
        );
        if let (Some(dir), true) = (out, b.follower_proven) {
            let val = pi.assignment(m.vars.len(), &b.x, &b.y);
            SolutionFile::from_values(&m, "feasible", &val)
                .write(&dir.join(format!("{}.res.json", m.name)))
                .unwrap();
        }
    }
}

fn run_general(files: Vec<PathBuf>, hpr_time: f64, follower_time: f64, out: Option<&Path>) {
    use bobi::general::{check_point, feasible_point, has_coupling, GeneralOutcome};
    for path in files {
        let t = Instant::now();
        let m = match mps::read_instance(&path) {
            Ok(m) => m,
            Err(e) => {
                println!("{{\"file\":\"{}\",\"error\":{:?}}}", path.display(), e);
                continue;
            }
        };
        let coupling = has_coupling(&m);
        match feasible_point(&m, hpr_time, follower_time) {
            GeneralOutcome::Point(p) => {
                let (viol, phi, ll) = check_point(&m, &p.values, follower_time);
                let ok = viol <= 1e-6 && phi.is_some_and(|f| (ll - f).abs() / (1e-10 + f.abs()) <= 1e-4);
                println!(
                    "{{\"name\":\"{}\",\"coupling\":{},\"status\":\"point\",\"upper_obj\":{},\"lower_obj\":{},\"recheck_phi\":{},\"max_violation\":{:.2e},\"verified\":{},\"secs\":{:.1}}}",
                    m.name, coupling, p.upper_obj, ll,
                    phi.map_or("null".into(), |f| f.to_string()), viol, ok, t.elapsed().as_secs_f64()
                );
                if let (Some(dir), true) = (out, ok) {
                    SolutionFile::from_values(&m, "feasible", &p.values)
                        .write(&dir.join(format!("{}.res.json", m.name)))
                        .unwrap();
                }
            }
            GeneralOutcome::Infeasible => println!(
                "{{\"name\":\"{}\",\"coupling\":{},\"status\":\"hpr_infeasible\",\"secs\":{:.1}}}",
                m.name, coupling, t.elapsed().as_secs_f64()
            ),
            GeneralOutcome::Failed(why) => println!(
                "{{\"name\":\"{}\",\"coupling\":{},\"status\":\"failed\",\"why\":{:?},\"secs\":{:.1}}}",
                m.name, coupling, why, t.elapsed().as_secs_f64()
            ),
        }
    }
}
