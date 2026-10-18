"""Combine the runs into results/clique/summary.csv and set each solution file's status.

usage: python3 -I summarize.py RUNS_DIR
reads RUNS_DIR/{parse_clique,ub,lb,lb_bnb}.jsonl and, when present, RUNS_DIR/k4cover_*.json
"""
import csv
import glob
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def jsonl(path):
    with open(path) as fh:
        return {d["inst"]: d for d in map(json.loads, fh)}


def main():
    runs = sys.argv[1]
    parse = jsonl(os.path.join(runs, "parse_clique.jsonl"))
    ub = jsonl(os.path.join(runs, "ub.jsonl"))
    lb = jsonl(os.path.join(runs, "lb.jsonl"))
    bnb = jsonl(os.path.join(runs, "lb_bnb.jsonl"))
    cover = {}
    for f in glob.glob(os.path.join(runs, "k4cover_*.json")):
        with open(f) as fh:
            for line in fh:
                d = json.loads(line)
                cover[d["inst"]] = d
    meta = cl.load_meta()
    rows = []
    for name in sorted(parse, key=cl.sort_key):
        p, u, b = parse[name], ub[name], lb[name]
        density = 0.7 if p["density"] < 0.8 else 0.9
        lo, why = 1, ""
        if b["mantel_ok"]:
            lo, why = 3, f"Mantel: m-k={b['remaining']} > floor(n^2/4)={b['mantel']}"
        if b.get("lb4_proved") and bnb.get(name, {}).get("w3_gt_k_bnb"):
            nvec = len(b["size_vectors"])
            assert nvec == len(bnb[name]["size_vectors"])
            lo, why = 4, (f"Brouwer 1981 Thm 2: m-k={b['remaining']} >= t3(n)-floor(n/3)+2={b['brouwer_need']}; "
                          f"w3(G) > k proved by CP-SAT, SCIP and split_bnb over {nvec} part-size vectors")
        hi = u["residual_omega"]
        cert = f"{u['r']}-partition with {u['deleted']} inside edges <= k; residual max clique {hi}"
        if hi > 3 and name in cover and cover[name].get("transversal", 10**9) <= p["k"]:
            c = cover[name]
            assert not c["remainder_has_k4"]
            hi = c["remainder_omega"]
            cert = f"K4-transversal of {c['transversal']} edges <= k (CP-SAT); residual max clique {hi}"
        assert lo <= hi
        status = "optimal" if lo == hi else "open"
        rows.append(dict(name=name, n=p["n"], density=density, m=p["m"], k=p["k"],
                         bobilib_value=int(p["bobilib_value"]), lower_bound=lo, upper_bound=hi,
                         status=status, lb_argument=why, ub_certificate=cert))
        sp = os.path.join(cl.RESULTS, name + ".res.json")
        with open(sp) as fh:
            sol = json.load(fh)
        assert sol["objective_value"] == hi, f"{name}: solution file disagrees with the bound"
        sol["status"] = "optimal" if status == "optimal" and sol["objective_value"] == lo else "feasible"
        sol["difficulty"] = meta[name].get("Difficulty")
        order = ["name", "bilevel_type", "status", "difficulty", "objective_value",
                 "upper_level_decisions", "lower_level_decisions"]
        with open(sp, "w") as fh:
            json.dump({key: sol[key] for key in order}, fh, indent=1)
    with open(os.path.join(cl.RESULTS, "summary.csv"), "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0]), lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    opt = sum(r["status"] == "optimal" for r in rows)
    print(f"{len(rows)} instances, {opt} optimal, {len(rows) - opt} open")


if __name__ == "__main__":
    main()
