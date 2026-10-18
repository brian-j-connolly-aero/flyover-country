use bobi::{
    kip::{Kip, SsOrder},
    mps,
    robust_ss::Outcome,
    solution::SolutionFile,
    structure::{self, PackingInterdiction},
    verify, wf,
};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    /// Verify solution files as BOBILib's feasibility checker does: rows,
    /// bounds and integrality, the stated objective, and follower optimality
    /// with the leader's part fixed. Takes `MPS SOL`, or `--instances DIR`
    /// and any number of solution files or directories of them.
    Verify {
        paths: Vec<PathBuf>,
        /// Batch mode: find each solution's `<name>.mps(.gz)` under this directory.
        #[arg(long)]
        instances: Option<PathBuf>,
        /// Seconds per follower MILP.
        #[arg(long, default_value_t = 60.0)]
        time_limit: f64,
        /// Solution files checked in parallel; each MILP uses one thread.
        #[arg(short, long, default_value_t = 1)]
        jobs: usize,
        /// Solve every follower problem as a MILP built from the model, even
        /// where the exact knapsack oracle applies.
        #[arg(long)]
        generic: bool,
    },
    /// Carry Weninger and Fukasawa's published KIP optima over to BOBILib's
    /// instances and write a verified solution file for each.
    KipImport {
        /// BOBILib's knapsack-interdiction directory (one subdirectory per set).
        instances: PathBuf,
        /// BOBILib's metadata, collection.json.
        #[arg(long)]
        meta: PathBuf,
        /// A checkout of github.com/nwoeanhinnogaehr/bkpsolver.
        #[arg(long)]
        wf: PathBuf,
        /// Where the solution files and summary.csv go.
        #[arg(long)]
        out: PathBuf,
        /// Seconds of greedy plus local search where no run reproduces the optimum.
        #[arg(long, default_value_t = 10.0)]
        ls_time: f64,
    },
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
    /// Exact optimum by enumerating every pattern of the binary linking variables.
    Enumerate {
        mps: PathBuf,
        #[arg(long, default_value_t = 60.0)]
        follower_time: f64,
        #[arg(long, default_value_t = 1)]
        threads: usize,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Certify that the follower is unbounded for every leader choice.
    Ray {
        files: Vec<PathBuf>,
        /// Write each direction as JSON {variable: value} into this directory.
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
        /// Write solution files (BOBILib format) for solved instances here.
        #[arg(long)]
        out: Option<PathBuf>,
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

fn strip_suffixes(p: &Path, suffixes: &[&str]) -> String {
    let f = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    suffixes
        .iter()
        .find_map(|s| f.strip_suffix(s))
        .map(str::to_string)
        .unwrap_or(f)
}

fn instance_name(p: &Path) -> String {
    strip_suffixes(p, &[".mps.gz", ".mps"])
}

fn parent_name(p: &Path) -> String {
    p.parent()
        .and_then(Path::file_name)
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Solution files as given, plus every `*.res.json` under given directories.
fn solution_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if !p.is_dir() {
            out.push(p.clone());
            continue;
        }
        let mut stack = vec![p.clone()];
        let mut found = Vec::new();
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in rd.flatten() {
                let q = e.path();
                if q.is_dir() {
                    stack.push(q);
                } else if q.to_string_lossy().ends_with(".res.json") {
                    found.push(q);
                }
            }
        }
        found.sort();
        out.extend(found);
    }
    out
}

/// Integers without a fractional part, other values as Rust prints them.
fn num(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn opt_num(v: Option<f64>) -> String {
    v.map_or("-".into(), num)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Checked {
    Pass,
    Fail,
    NoSolution,
    Error,
}

const VERIFY_HEADER: &str = "name\tresult\tfeasible\tmax_violation\tviolation_at\tobjective\tfile_objective\tobjective_match\tfollower_optimal\tphi_sol\tphi_check\tphi_bound\toracle\tseconds\tnote";

/// Verify one solution file against `mps`, or in batch mode against the
/// instance of the same name in `index` (preferring one in a directory of
/// the solution's own set name, as BOBILib's two trees mirror each other).
fn verify_file(
    index: &HashMap<String, Vec<PathBuf>>,
    mps: Option<&Path>,
    sol_path: &Path,
    opts: &verify::Options,
) -> (Checked, Option<verify::Report>, String) {
    let blank = "\t-".repeat(12);
    let fallback = strip_suffixes(sol_path, &[".res.json"]);
    let error =
        |name: &str, why: String| (Checked::Error, None, format!("{name}\tERROR{blank}\t{why}"));
    let sol = match SolutionFile::read(sol_path) {
        Ok(s) => s,
        Err(e) => return error(&fallback, e),
    };
    let mps = match mps {
        Some(p) => p.to_path_buf(),
        None => {
            let cands = index.get(&sol.name).or_else(|| index.get(&fallback));
            let set = parent_name(sol_path);
            match cands.and_then(|c| c.iter().find(|p| parent_name(p) == set).or(c.first())) {
                Some(p) => p.clone(),
                None => return error(&sol.name, "instance not found".into()),
            }
        }
    };
    if !verify::has_solution(&sol) {
        let status = sol.status.as_deref().unwrap_or("null");
        return (
            Checked::NoSolution,
            None,
            format!("{}\tNOSOL{blank}\tstatus {status}, no point", sol.name),
        );
    }
    let m = match mps::read_instance(&mps) {
        Ok(m) => m,
        Err(e) => return error(&sol.name, e),
    };
    let r = verify::verify(&m, &sol, opts);
    let yes = |b: bool| if b { "yes" } else { "no" };
    let note = r.notes();
    let row = format!(
        "{}\t{}\t{}\t{:.1e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{}",
        r.name,
        if r.passed() { "PASS" } else { "FAIL" },
        yes(r.feasible()),
        r.violation,
        if r.violation_at.is_empty() {
            "-"
        } else {
            &r.violation_at
        },
        num(r.objective),
        opt_num(r.file_objective),
        yes(r.objective_matches()),
        r.follower.as_str(),
        num(r.phi_sol),
        opt_num(r.phi_check),
        opt_num(r.phi_bound),
        r.oracle,
        r.seconds,
        if note.is_empty() { "-" } else { &note },
    );
    let c = if r.passed() {
        Checked::Pass
    } else {
        Checked::Fail
    };
    (c, Some(r), row)
}

/// A leader choice that reproduces the published optimum `ans`, and where
/// it came from: their runs on a file with this optimum (Comb first, then
/// DCS), else the greedy leader improved by local search.
fn wf_leader(
    pi: &PackingInterdiction,
    k: &Kip,
    files: &[&wf::KiFile],
    runs: &HashMap<String, Vec<wf::Run>>,
    ans: i64,
    ls_time: f64,
) -> Result<(Vec<bool>, String), String> {
    let mut why = Vec::new();
    for alg in ["comb", "dcs"] {
        for f in files {
            let ok = runs.get(&f.file).into_iter().flatten().filter(|r| {
                r.algorithm == alg && r.status == "ok" && r.profit == Some(ans) && r.upper.is_some()
            });
            for (r, order) in ok.flat_map(|r| [(r, wf::BitOrder::File), (r, wf::BitOrder::Sorted)])
            {
                let x = match wf::leader_from_bits(k, f, r.upper.as_deref().unwrap(), order) {
                    Ok(x) => x,
                    Err(e) => {
                        why.push(e);
                        continue;
                    }
                };
                let v = k.follower(&x);
                if !pi.leader_feasible(&x) {
                    why.push(format!("{alg} run in {order:?} order exceeds the budget"));
                } else if v.truncated || v.value != ans {
                    why.push(format!("{alg} run in {order:?} order gives {}", v.value));
                } else {
                    let file = f.file.strip_prefix("instances/").unwrap_or(&f.file);
                    let tag = match order {
                        wf::BitOrder::File => "",
                        wf::BitOrder::Sorted => "-sorted",
                    };
                    return Ok((x, format!("wf-{alg}{tag}:{file}")));
                }
            }
        }
    }
    if why.is_empty() {
        why.push("no run with this optimum and a leader solution".into());
    }
    eprintln!("note: {}: {}", pi.name, why.join("; "));
    let (x0, _) = k.greedy();
    let (x, f) = pi.local_search(&x0, ls_time, ls_time);
    if f.proven && f.value == ans {
        return Ok((x, "greedy+ls".into()));
    }
    if f.proven && f.value < ans {
        why.push(format!(
            "local search beats the published optimum: {}",
            f.value
        ));
    } else {
        why.push(format!("local search reaches {}", f.value));
    }
    Err(why.join("; "))
}

/// The `kip-import` command: one verified solution file per BOBILib KIP
/// instance with a published optimum, and a summary over all of them.
fn kip_import(
    dir: &Path,
    meta: &Path,
    wf_root: &Path,
    out: &Path,
    ls_time: f64,
) -> Result<(), String> {
    let text = std::fs::read_to_string(meta).map_err(|e| format!("{}: {e}", meta.display()))?;
    let meta: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let (index, bad) = wf::index(wf_root);
    let results = wf_root.join("full_results").join("all_results.json");
    let (runs, bad_runs) = wf::read_runs(&results)?;
    eprintln!(
        "WF: {} instance signatures ({} unreadable .ki files), runs for {} files ({} unreadable lines)",
        index.len(),
        bad.len(),
        runs.len(),
        bad_runs
    );
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut csv = String::from(
        "name,set,bobilib_status,bobilib_value,literature_optimum,verified_value,source\n",
    );
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    let mut problems = Vec::new();
    for path in instances(dir) {
        let name = instance_name(&path);
        let set = parent_name(&path);
        let info = meta
            .get(&name)
            .ok_or_else(|| format!("{name}: not in the metadata"))?;
        let status = info["Status"].as_str().unwrap_or("").to_string();
        let best = match &info["Best known objective"] {
            Value::Number(v) => v.as_f64().map(num).unwrap_or_default(),
            Value::String(s) => s.clone(),
            _ => String::new(),
        };
        let m = mps::read_instance(&path)?;
        let pi = structure::packing_interdiction(&m).map_err(|e| format!("{name}: {e}"))?;
        let k = Kip::from_packing(&pi).ok_or_else(|| format!("{name}: not single-row KIP"))?;
        let files: Vec<&wf::KiFile> = index
            .get(&wf::kip_signature(&k))
            .map(|v| v.iter().collect())
            .unwrap_or_default();
        let answers: BTreeSet<i64> = files.iter().filter_map(|f| f.ans).collect();
        if answers.len() > 1 {
            problems.push(format!("{name}: their files disagree: {answers:?}"));
        }
        let mut row = |lit: String, verified: String, source: &str| {
            csv.push_str(&format!(
                "{name},{set},{status},{best},{lit},{verified},{source}\n"
            ));
        };
        let Some(&ans) = answers.first() else {
            let what = if files.is_empty() {
                "unmatched"
            } else {
                "no-optimum"
            };
            *tally.entry(format!("{what} ({status})")).or_default() += 1;
            row(String::new(), String::new(), what);
            continue;
        };
        let lit = pi.objective(ans);
        let with_ans: Vec<&wf::KiFile> = files.into_iter().filter(|f| f.ans == Some(ans)).collect();
        let (x, source) = match wf_leader(&pi, &k, &with_ans, &runs, ans, ls_time) {
            Ok(found) => found,
            Err(why) => {
                problems.push(format!("{name}: no leader reproduces {ans}: {why}"));
                *tally.entry(format!("unresolved ({status})")).or_default() += 1;
                row(num(lit), String::new(), "none");
                continue;
            }
        };
        let mut y = vec![false; k.n];
        for j in k.follower(&x).take {
            y[j] = true;
        }
        let val = pi.assignment(m.vars.len(), &x, &y);
        let mut sol = SolutionFile::from_values(&m, "optimal", &val);
        sol.name = name.clone();
        sol.difficulty = info["Difficulty"].as_str().map(str::to_string);
        let r = verify::verify(&m, &sol, &verify::Options::default());
        let value = sol.objective_value.unwrap_or(f64::NAN);
        let at_optimum = (value - lit).abs() <= 1e-9 * lit.abs().max(1.0);
        if !r.passed() || !at_optimum {
            problems.push(format!("{name}: built solution fails verification: {r:?}"));
            *tally.entry(format!("unresolved ({status})")).or_default() += 1;
            row(num(lit), String::new(), "none");
            continue;
        }
        sol.write(&out.join(format!("{name}.res.json")))?;
        row(num(lit), num(value), &source);
        *tally.entry(format!("verified ({status})")).or_default() += 1;
        if let Ok(b) = best.parse::<f64>() {
            if b < lit {
                problems.push(format!(
                    "{name}: BOBILib's value {b} is below the optimum {lit}"
                ));
            } else if status == "optimal" && b != lit {
                problems.push(format!("{name}: BOBILib's optimum {b} differs from {lit}"));
            }
        }
        let kind = source.split(':').next().unwrap_or_default();
        *tally.entry(format!("source {kind}")).or_default() += 1;
    }
    std::fs::write(out.join("summary.csv"), csv).map_err(|e| e.to_string())?;
    for (what, n) in &tally {
        eprintln!("{n}\t{what}");
    }
    for p in &problems {
        eprintln!("PROBLEM {p}");
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Enumerate {
            mps: path,
            follower_time,
            threads,
            out,
        } => {
            let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
            let t = Instant::now();
            match bobi::general::enumerate_linking(&m, follower_time, threads) {
                None => println!("{}\tno bilevel feasible leader choice", m.name),
                Some((v, val, feasible)) => {
                    println!(
                        "{}\tOPTIMAL\t{v}\tfeasible_leader_choices={feasible}\t{:.1}s",
                        m.name,
                        t.elapsed().as_secs_f64()
                    );
                    if let Some(dir) = out {
                        SolutionFile::from_values(&m, "optimal", &val)
                            .write(&dir.join(format!("{}.res.json", m.name)))
                            .unwrap();
                    }
                }
            }
        }
        Cmd::Ray { files, out } => {
            for path in files {
                let m = mps::read_instance(&path).unwrap_or_else(|e| panic!("{e}"));
                match bobi::general::follower_unbounded_ray(&m, 300.0) {
                    None => println!("{}\tno follower ray", m.name),
                    Some((d, val)) => {
                        let support = d.iter().filter(|x| x.abs() > 1e-9).count();
                        println!("{}\tfollower unbounded: direction with objective {val:.4} on {support} variables", m.name);
                        if let Some(dir) = &out {
                            let map: serde_json::Map<String, serde_json::Value> = m
                                .vars
                                .iter()
                                .zip(&d)
                                .filter(|(_, x)| **x != 0.0)
                                .map(|(v, &x)| (v.name.clone(), serde_json::Value::from(x)))
                                .collect();
                            std::fs::write(
                                dir.join(format!("{}.ray.json", m.name)),
                                serde_json::to_string(&map).unwrap(),
                            )
                            .unwrap();
                        }
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
            out,
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
                if let Some(dir) = &out {
                    let f = k.follower(&s.x);
                    assert_eq!(f.value, s.value, "follower value disagrees with the solver");
                    let mut y = vec![false; k.n];
                    for &j in &f.take {
                        y[j] = true;
                    }
                    let val = pi.assignment(m.vars.len(), &s.x, &y);
                    let status = if s.proven { "optimal" } else { "feasible" };
                    SolutionFile::from_values(&m, status, &val)
                        .write(&dir.join(format!("{}.res.json", m.name)))
                        .unwrap();
                }
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
        Cmd::Verify {
            paths,
            instances: dir,
            time_limit,
            jobs,
            generic,
        } => {
            let opts = verify::Options {
                time_limit,
                threads: 1,
                generic,
            };
            let mut index: HashMap<String, Vec<PathBuf>> = HashMap::new();
            let pairs: Vec<(Option<PathBuf>, PathBuf)> = match &dir {
                None => {
                    let [mps, sol] = &paths[..] else {
                        eprintln!("give MPS SOL, or --instances DIR and solution files");
                        std::process::exit(2);
                    };
                    vec![(Some(mps.clone()), sol.clone())]
                }
                Some(d) => {
                    for p in instances(d) {
                        index.entry(instance_name(&p)).or_default().push(p);
                    }
                    solution_files(&paths)
                        .into_iter()
                        .map(|s| (None, s))
                        .collect()
                }
            };
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(jobs.max(1))
                .build()
                .expect("thread pool");
            let results: Vec<_> = pool.install(|| {
                pairs
                    .par_iter()
                    .map(|(mps, sol)| verify_file(&index, mps.as_deref(), sol, &opts))
                    .collect()
            });
            println!("{VERIFY_HEADER}");
            for (_, _, row) in &results {
                println!("{row}");
            }
            let count = |c: Checked| results.iter().filter(|r| r.0 == c).count();
            let reports: Vec<&verify::Report> =
                results.iter().filter_map(|r| r.1.as_ref()).collect();
            let follower = |v: verify::Verdict| reports.iter().filter(|r| r.follower == v).count();
            eprintln!(
                "{} files: {} pass, {} fail ({} infeasible, {} objective mismatch, {} follower not optimal, {} follower unproven), {} without a solution, {} errors",
                results.len(),
                count(Checked::Pass),
                count(Checked::Fail),
                reports.iter().filter(|r| !r.feasible()).count(),
                reports.iter().filter(|r| !r.objective_matches()).count(),
                follower(verify::Verdict::No),
                follower(verify::Verdict::Unproven),
                count(Checked::NoSolution),
                count(Checked::Error),
            );
            if count(Checked::Fail) + count(Checked::Error) > 0 {
                std::process::exit(1);
            }
        }
        Cmd::KipImport {
            instances: dir,
            meta,
            wf: wf_root,
            out,
            ls_time,
        } => {
            if let Err(e) = kip_import(&dir, &meta, &wf_root, &out, ls_time) {
                eprintln!("{e}");
                std::process::exit(1);
            }
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
