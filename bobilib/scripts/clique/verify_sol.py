"""Check BOBILib solution files the way BOBILib's feasibility checker does, without the graph:
  1. every bound, integrality and row of the MPS holds at the file's point;
  2. with the leader's variables fixed to the file's values, the follower's problem (its rows
     and objective from the AUX file, read generically) is solved to optimality by SCIP, and its
     optimum must equal the file's lower-level objective;
  3. the upper-level objective at the file's point equals the file's objective_value.
Since the leader minimises sum x and the follower minimises -sum x, the follower's optimal
points all give the same leader objective, so optimistic and pessimistic readings agree.

usage: python3 -I verify_sol.py SUBSET SOLUTION_DIR > verify.jsonl
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402
from pyscipopt import Model, quicksum  # noqa: E402


def follower_optimum(inst, val):
    md = Model()
    md.hideOutput()
    md.setParam("parallel/maxnthreads", 1)
    md.setParam("limits/time", 3600)
    fol = [name for name in inst.order if name in inst.lvars]
    x = {}
    for name in fol:
        v = inst.vars[name]
        x[name] = md.addVar(lb=v["lb"], ub=v["ub"], vtype="B" if v["int"] else "C", name=name)
    for r, row in inst.rows.items():
        if r not in inst.lrows:
            continue
        terms, const = [], 0.0
        for name, v in inst.vars.items():
            a = v["coefs"].get(r)
            if a is None:
                continue
            if name in x:
                terms.append(a * x[name])
            else:
                const += a * val[name]
        lhs = quicksum(terms) + const
        if row["sense"] == "L":
            md.addCons(lhs <= row["rhs"])
        elif row["sense"] == "G":
            md.addCons(lhs >= row["rhs"])
        else:
            md.addCons(lhs == row["rhs"])
    md.setObjective(quicksum(inst.lvars[n] * x[n] for n in fol), "minimize")
    md.optimize()
    assert md.getStatus() == "optimal", md.getStatus()
    return md.getObjVal()


def main():
    sub, sdir = sys.argv[1], sys.argv[2]
    paths = sorted(cl.instance_paths(sub), key=lambda p: cl.sort_key(os.path.basename(p)))
    for p in paths:
        inst = cl.load(p)
        sp = os.path.join(sdir, inst.name + ".res.json")
        if not os.path.exists(sp):
            continue
        t0 = time.time()
        s = json.load(open(sp))
        val = {name: 0.0 for name in inst.vars}
        val.update(s["upper_level_decisions"])
        val.update(s["lower_level_decisions"])
        assert set(val) == set(inst.vars), "solution names differ from the instance's"
        viol, what = inst.max_violation(val)
        upper = sum(v["obj"] * val[n] for n, v in inst.vars.items())
        lower = sum(c * val[n] for n, c in inst.lvars.items())
        opt = follower_optimum(inst, val)
        ok = viol <= 1e-9 and abs(opt - lower) <= 1e-6 and abs(upper - s["objective_value"]) <= 1e-9
        print(json.dumps(dict(inst=inst.name, objective=s["objective_value"], upper=upper, lower=lower,
                              follower_opt=opt, max_violation=viol, worst=what, budget_used=sum(
                                  val[n] for n in inst.ws.values()), k=inst.k, ok=ok,
                              sec=round(time.time() - t0, 1))), flush=True)


if __name__ == "__main__":
    main()
