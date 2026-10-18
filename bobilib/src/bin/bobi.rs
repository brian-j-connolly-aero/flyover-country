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
    /// Root online-relaxation bound with merged leader rows and scaling.
    Omega {
        mps: PathBuf,
        #[arg(long, default_value_t = 2000)]
        bu: usize,
        #[arg(long, default_value_t = 4000)]
        cl: usize,
    },
    /// Solve subset-sum knapsack interdiction instances exactly.
    SsSolve {
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = 200_000_000)]
        node_limit: u64,
        #[arg(long, value_enum, default_value_t = SsOrder::Ascending)]
        order: SsOrder,
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
        Cmd::Omega { mps: path, bu, cl } => {
            let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
            let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
            // Normalise each leader row by its capacity.
            let mu: Vec<f64> = pi.lead_cap.iter().map(|&c| 1.0 / c.max(1) as f64).collect();
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
                let s = k.solve_subset_sum(node_limit, true, order);
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
        } => {
            let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
            let pi = structure::packing_interdiction(&m).unwrap_or_else(|e| panic!("{e}"));
            let k = Kip::from_packing(&pi).expect("not a single-row knapsack interdiction");
            assert!(k.is_subset_sum(), "profits differ from follower weights");
            let (_, greedy) = k.greedy();
            let lo = lo.unwrap_or(greedy);
            let t = Instant::now();
            let (out, st, classes, g) = k.subset_sum_threshold(lo, node_limit, online, order);
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
