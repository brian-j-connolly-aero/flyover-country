"""Recompile Lefebvre and Schmidt's published points for the multidimensional
knapsack interdiction instances (columns `lefebvre_schmidt_ub` and
`best_known_ub` of results/literature/mkip_literature_status.csv) from their
run files, with the authors' own rule for a point.

usage: python3 -I ls_mkip_points.py RUN_DIR mkip_literature_status.csv [--out FILE]

RUN_DIR holds their run files results-MILP.csv, results-MILP-first.csv,
results-PADM.csv and results-PADM-interdiction.csv (open-science page of
H. Lefebvre and M. Schmidt, "A Dantzig-Wolfe single-level reformulation for
mixed-integer linear bilevel optimization", 2025). Their analysis script
(methodology.render.Rmd) counts a run as having found a point only if it
ended before the time limit of 3,600 s, and a PADM run only if its status is
`Feasible`; rows that hit the limit can carry relaxation values that no
bilevel feasible point has (results/notes/lefebvre_schmidt.md). This script
applies the same rule: `lefebvre_schmidt_ub` is the smallest finite value of
a non-`Infeasible` row with time < 3600 (PADM rows: status `Feasible`), or
empty. `best_known_ub` is the minimum of Lozano, Bergman and Cire's point,
that value, Fischetti et al.'s 2019 optimum and BOBILib's value. The column
`lefebvre_schmidt_ub_timelimit` records, for reference only, the smallest
nonzero value of the rows the rule does not count when it is below
`lefebvre_schmidt_ub` (or when no row counts); it enters no comparison. The
value 0 of those rows, the optimum of the high point relaxation, is not
recorded. The other columns are copied unchanged. Prints what changed.
"""
import csv
import os
import sys


def num(s):
    try:
        return float(s)
    except (TypeError, ValueError):
        return None


def main():
    args = sys.argv[1:]
    out = None
    if "--out" in args:
        i = args.index("--out")
        out = args[i + 1]
        del args[i:i + 2]
    run_dir, lit_path = args
    out = out or lit_path
    best, best_tl = {}, {}
    for fn in ("results-MILP.csv", "results-MILP-first.csv", "results-PADM.csv", "results-PADM-interdiction.csv"):
        padm = "PADM" in fn
        with open(os.path.join(run_dir, fn), encoding="latin-1") as f:
            for line in f:
                t = line.strip().split(",")
                if len(t) < 8:
                    continue
                inst = os.path.basename(t[1])
                if inst.endswith(".aux"):
                    inst = inst[:-4]
                time, status, val = (float(t[6]), t[7], t[9]) if padm else (float(t[3]), t[4], t[6])
                v = num(val)
                if v is None or abs(v) >= 1e19 or status.startswith("Infeasible"):
                    continue
                if time < 3600 and (not padm or status == "Feasible"):
                    if inst not in best or v < best[inst]:
                        best[inst] = v
                elif v != 0 and (inst not in best_tl or v < best_tl[inst]):
                    best_tl[inst] = v
    with open(lit_path, newline="") as f:
        rows = list(csv.DictReader(f))
        fields = list(rows[0].keys())
    tl_col = "lefebvre_schmidt_ub_timelimit"
    if tl_col not in fields:
        fields.insert(fields.index("lefebvre_schmidt_ub") + 1, tl_col)
    changed = {"ls": 0, "ls_added": 0, "ls_dropped": 0, "best": 0}
    for r in rows:
        old_ls = num(r["lefebvre_schmidt_ub"])
        new_ls = best.get(r["name"])
        if old_ls != new_ls:
            changed["ls"] += 1
            changed["ls_added"] += old_ls is None
            changed["ls_dropped"] += new_ls is None
            r["lefebvre_schmidt_ub"] = "" if new_ls is None else repr(new_ls)
        tl = best_tl.get(r["name"])
        r[tl_col] = repr(tl) if tl is not None and (new_ls is None or tl < new_ls) else ""
        cand = [(num(r[k]), r[k]) for k in ("dd_lcb2026_ub", "lefebvre_schmidt_ub", "flms2019_opt", "bobilib_local_obj")]
        cand = [c for c in cand if c[0] is not None]
        new_best = min(cand)[0] if cand else None
        if num(r["best_known_ub"]) != new_best:
            print(f"{r['name']}: best published point {r['best_known_ub']} -> {new_best}")
            changed["best"] += 1
            r["best_known_ub"] = "" if new_best is None else min(cand)[1]
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    print(f"lefebvre_schmidt_ub changed on {changed['ls']} rows ({changed['ls_added']} added, "
          f"{changed['ls_dropped']} dropped); best_known_ub changed on {changed['best']} rows -> {out}")


if __name__ == "__main__":
    main()
