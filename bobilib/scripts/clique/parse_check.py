"""Step (a): parse every instance of a clique-interdiction subset, check its structure row by row, and
re-evaluate BOBILib's own solution file (budget, residual maximum clique, follower optimality).

usage: python3 -I parse_check.py [clique|bcpins|plusbcpins] > out.jsonl
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def main():
    sub = sys.argv[1] if len(sys.argv) > 1 else "clique"
    meta = cl.load_meta()
    paths = sorted(cl.instance_paths(sub), key=lambda p: cl.sort_key(os.path.basename(p)))
    for p in paths:
        t0 = time.time()
        try:
            inst = cl.load(p)
        except (AssertionError, KeyError, ValueError) as e:
            print(json.dumps(dict(inst=os.path.basename(p), structure_error=str(e))), flush=True)
            continue
        md = meta.get(inst.name, {})
        out = dict(inst=inst.name, n=inst.n, m=inst.m, nonedges=len(inst.nonedges), k=inst.k,
                   density=round(2 * inst.m / (inst.n * (inst.n - 1)), 4),
                   bobilib_status=md.get("Status"), bobilib_value=md.get("Best known objective"),
                   omega_G=len(cl.max_clique(inst.adjacency())))
        sp = os.path.join(cl.SOL_DIR, sub, inst.name + ".res.json")
        if os.path.exists(sp):
            s = json.load(open(sp))
            val = dict(s["upper_level_decisions"])
            val.update(s["lower_level_decisions"])
            removed = [e for e in inst.edges if val.get(inst.ws[e], 0) > 0.5]
            A = inst.adjacency(removed)
            clique = [i for i in range(inst.n) if val.get(inst.xs[i], 0) > 0.5]
            om = cl.max_clique(A)
            out.update(sol_obj=s.get("objective_value"), sol_removed=len(removed),
                       sol_clique=len(clique), sol_clique_valid=cl.is_clique(A, clique),
                       sol_residual_omega=len(om), sol_viol=inst.max_violation(val)[0])
        out["sec"] = round(time.time() - t0, 2)
        print(json.dumps(out), flush=True)


if __name__ == "__main__":
    main()
