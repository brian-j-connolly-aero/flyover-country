"""Re-check every solution file in results/clique against its instance, from scratch.

For each <name>.res.json: every variable of the instance is present, the full assignment
violates no bound, integrality or row (cliquelib.Instance.max_violation), the leader deletes
at most k edges, the follower's x is a clique of G minus the deleted edges, the upper
objective (sum x) equals the file's objective value, that clique is maximum (exact branch
and bound, plus an exhaustive test that no clique one larger exists), and for an "optimal"
file the value equals the proved lower bound in summary.csv.

usage: python3 -I recheck.py > recheck.jsonl
"""
import csv
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def main():
    with open(os.path.join(cl.RESULTS, "summary.csv")) as fh:
        summary = {r["name"]: r for r in csv.DictReader(fh)}
    bad = 0
    for p in cl.instance_paths("clique"):
        inst = cl.load(p)
        with open(os.path.join(cl.RESULTS, inst.name + ".res.json")) as fh:
            sol = json.load(fh)
        val = {**sol["upper_level_decisions"], **sol["lower_level_decisions"]}
        missing = sorted(set(inst.vars) - set(val))
        extra = sorted(set(val) - set(inst.vars))
        viol, what = inst.max_violation(val)
        D = [e for e in inst.edges if val[inst.ws[e]] > 0.5]
        C = [i for i in range(inst.n) if val[inst.xs[i]] > 0.5]
        H = inst.adjacency(D)
        omega = len(cl.max_clique(H))
        row = summary[inst.name]
        d = dict(inst=inst.name, k=inst.k, deleted=len(D), clique=len(C), objective=sol["objective_value"],
                 status=sol["status"], max_violation=viol, worst=what, missing=len(missing), extra=len(extra),
                 clique_ok=cl.is_clique(H, C), omega=omega, no_larger=not cl.has_clique(H, omega + 1),
                 lower_bound=int(row["lower_bound"]), upper_bound=int(row["upper_bound"]))
        d["ok"] = (not missing and not extra and viol == 0.0 and len(D) <= inst.k and d["clique_ok"]
                   and len(C) == omega == sol["objective_value"] == d["upper_bound"] and d["no_larger"]
                   and (sol["status"] != "optimal" or d["lower_bound"] == omega))
        bad += not d["ok"]
        print(json.dumps(d), flush=True)
    print(f"{bad} failures", file=sys.stderr)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
