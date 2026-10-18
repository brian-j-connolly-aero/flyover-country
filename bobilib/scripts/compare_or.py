"""Compare our bounds on BOBILib's 'or' multidimensional knapsack
interdiction set with BOBILib's values and the best published bounds.

usage: python3 -I compare_or.py LITERATURE_CSV BOBILIB_JSON RESULTS.jsonl... [--csv OUT]

LITERATURE_CSV has columns name, dd_lcb2026_lb, best_known_ub,
proven_optimal_in_literature (one row per instance), collected from
Lozano, Bergman and Cire (Operations Research 2026), Fischetti et al.
(INFORMS JoC 2019) and Lefebvre and Schmidt (2025). Result files are the
JSON lines written by `bobi bounds` (lb/ub) and `bobi upper-multi`
(ub_lp, follower_value when certified).
"""
import collections
import csv
import json
import re
import sys


def num(s):
    try:
        return float(s)
    except (TypeError, ValueError):
        return None


def main():
    args = sys.argv[1:]
    out = None
    if "--csv" in args:
        i = args.index("--csv")
        out = args[i + 1]
        del args[i : i + 2]
    lit_path, meta_path, *results = args
    lit = {r["name"]: r for r in csv.DictReader(open(lit_path))}
    meta = json.load(open(meta_path))
    ours = {}
    for path in results:
        for line in open(path):
            line = line.strip()
            if not line:
                continue
            d = json.loads(line)
            if "name" not in d:
                continue
            o = ours.setdefault(d["name"], {})
            if "ub" in d:  # bobi bounds: certified point and a lower bound
                o["lb"] = max(o.get("lb", 0), d["lb"])
                o["ub_point"] = min(o.get("ub_point", float("inf")), d["ub"])
            if d.get("certified"):
                o["ub_point"] = min(o.get("ub_point", float("inf")), d["follower_value"])
            if d.get("ub_lp") is not None:
                o["ub_bound"] = min(o.get("ub_bound", float("inf")), d["ub_lp"])

    rows = []
    tally = collections.Counter()
    for name, o in sorted(ours.items()):
        m = re.match(r"OR(\d+)x(\d+)-([\d.]+)_(\d+)\.dat-(\d+)", name)
        if not m:
            continue
        M, N, alpha, _, var = m.groups()
        L = lit.get(name, {})
        lit_lb = num(L.get("dd_lcb2026_lb"))
        lit_ub = num(L.get("best_known_ub"))
        bob = num(meta.get(name, {}).get("Best known objective"))
        lb = max(x for x in (o.get("lb"), lit_lb, 0) if x is not None)
        ub = min(x for x in (o.get("ub_point"), o.get("ub_bound"), lit_ub, float("inf")) if x is not None)
        point = o.get("ub_point")
        better_point = point is not None and (lit_ub is None or point < lit_ub - 1e-6)
        first_point = point is not None and lit_ub is None
        better_lb = o.get("lb") is not None and (lit_lb is None or o["lb"] > lit_lb + 1e-6)
        proven = point is not None and lb >= point - 1e-6
        tally["instances"] += 1
        tally["better_point"] += better_point
        tally["first_point"] += first_point
        tally["better_lb"] += better_lb
        tally["proven_optimal"] += proven
        rows.append(
            dict(
                name=name, m=M, n=N, alpha=alpha, variant=var, bobilib=bob, lit_lb=lit_lb, lit_ub=lit_ub,
                our_lb=o.get("lb"), our_point=point, our_lp_bound=o.get("ub_bound"), best_lb=lb,
                best_ub=None if ub == float("inf") else ub,
                gap=None if ub in (0, float("inf")) else (ub - lb) / ub,
                lit_gap=None if not (lit_lb and lit_ub) else (lit_ub - lit_lb) / lit_ub,
            )
        )
    if out:
        with open(out, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=list(rows[0]))
            w.writeheader()
            w.writerows(rows)
    for k, v in tally.items():
        print(f"{k}: {v}")
    by = collections.defaultdict(list)
    for r in rows:
        if r["gap"] is not None:
            by[(r["variant"], r["alpha"])].append((r["gap"], r["lit_gap"]))
    for k in sorted(by):
        g = sorted(x for x, _ in by[k])
        lg = sorted(y for _, y in by[k] if y is not None)
        med = lambda v: v[len(v) // 2] if v else float("nan")
        print(f"variant -{k[0]} alpha {k[1]}: {len(g)} instances, median gap ours+lit {med(g):.4f}, literature alone {med(lg):.4f}")


if __name__ == "__main__":
    main()
