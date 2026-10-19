"""Tight check of a coupled solution file, independent of bobilib's Rust code.

Reads the MPS with HiGHS (highspy) and the AUX file itself.
1. rows, bounds, integrality of the file's point (max absolute violation);
2. phi* = follower optimum at the file's leader values (follower rows only),
   mip_rel_gap 0, mip_abs_gap 1e-9, feasibility tolerance 1e-9;
   slack = f(y_file) - dual bound;
3. tie-break: all rows, leader fixed, f(y) <= phi* + 1e-6 (the full tight
   allowance), minimise F; reports the proven bound and F_file - bound
   (what the leader could still gain inside the allowance).
usage: python3 -I scripts/coupled_tight_check.py TIME MPS AUX SOL
prints one TSV line.
"""
import gzip
import json
import os
import shutil
import sys
import tempfile
import time

import highspy
import numpy as np


def read_aux(path):
    ll, cons, sec = {}, [], None
    for line in open(path, encoding="latin-1"):
        line = line.strip()
        if not line:
            continue
        if line.startswith("@"):
            sec = line
            continue
        if sec == "@VARSBEGIN":
            p = line.split()
            ll[p[0]] = float(p[1])
        elif sec == "@CONSTRSBEGIN":
            cons.append(line.split()[0])
    return ll, set(cons)


def model(plain, tl, rel, absg):
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(plain)
    for k, v in (("threads", 1), ("time_limit", tl), ("mip_rel_gap", rel),
                 ("mip_abs_gap", absg), ("primal_feasibility_tolerance", 1e-9),
                 ("mip_feasibility_tolerance", 1e-9)):
        h.setOptionValue(k, v)
    return h


def main():
    tl = float(sys.argv[1])
    mps, aux, solp = sys.argv[2:5]
    tmp = tempfile.mkdtemp(dir=os.environ.get("PYCHECK_TMP"))
    plain = os.path.join(tmp, "m.mps")
    t0 = time.time()
    try:
        with gzip.open(mps, "rb") as s, open(plain, "wb") as d:
            shutil.copyfileobj(s, d)
        ll, cons = read_aux(aux)
        sol = json.load(open(solp))
        vals = dict(sol["upper_level_decisions"])
        vals.update(sol["lower_level_decisions"])
        h0 = model(plain, tl, 0.0, 1e-9)
        lp = h0.getLp()
        names = list(lp.col_names_)
        rnames = list(lp.row_names_)
        n = len(names)
        integ = list(lp.integrality_) if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * n
        is_int = np.array([t == highspy.HighsVarType.kInteger for t in integ])
        is_ll = np.array([nm in ll for nm in names])
        x = np.array([float(vals.get(nm, 0.0)) for nm in names])
        c = np.array(lp.col_cost_)
        F = float(c @ x + lp.offset_)
        fcoef = np.array([ll.get(nm, 0.0) for nm in names])
        f_sol = float(fcoef @ x)
        # 1. violations
        A = lp.a_matrix_
        act = np.zeros(len(rnames))
        for j in range(n):
            for k in range(A.start_[j], A.start_[j + 1]):
                act[A.index_[k]] += A.value_[k] * x[j]
        lo, hi = np.array(lp.row_lower_), np.array(lp.row_upper_)
        cl, cu = np.array(lp.col_lower_), np.array(lp.col_upper_)
        viol = max(
            float(np.max(np.maximum(lo - act, 0.0), initial=0.0)),
            float(np.max(np.maximum(act - hi, 0.0), initial=0.0)),
            float(np.max(np.maximum(cl - x, 0.0), initial=0.0)),
            float(np.max(np.maximum(x - cu, 0.0), initial=0.0)),
            float(np.max(np.abs(x - np.round(x)) * is_int, initial=0.0)),
        )
        inf = highspy.kHighsInf
        # 2. follower optimum at the file's leader values
        f = model(plain, tl, 0.0, 1e-9)
        for j in range(n):
            if not is_ll[j]:
                v = round(x[j]) if is_int[j] else x[j]
                f.changeColBounds(j, v, v)
            f.changeColCost(j, fcoef[j])
        for i, rn in enumerate(rnames):
            if rn not in cons:
                f.changeRowBounds(i, -inf, inf)
        f.changeObjectiveOffset(0.0)
        f.run()
        fs = str(f.getModelStatus()).replace("HighsModelStatus.k", "")
        fi = f.getInfo()
        phi = fi.objective_function_value
        phib = fi.mip_dual_bound if np.any(is_int & is_ll) else phi
        slack = f_sol - phib
        # 3. tie-break inside the full allowance
        g = model(plain, tl, 1e-9, 1e-9)
        for j in range(n):
            if not is_ll[j]:
                v = round(x[j]) if is_int[j] else x[j]
                g.changeColBounds(j, v, v)
        idx = np.nonzero(is_ll & (fcoef != 0))[0].astype(np.int32)
        g.addRow(-inf, phib + 1e-6, len(idx), idx, fcoef[idx])
        g.run()
        gs = str(g.getModelStatus()).replace("HighsModelStatus.k", "")
        gi = g.getInfo()
        gb = gi.mip_dual_bound if np.any(is_int) else gi.objective_function_value
        if gs != "Optimal":
            gb = float("nan")
        print("\t".join(str(s) for s in [
            os.path.basename(solp).replace(".res.json", ""),
            repr(F), repr(sol.get("objective_value")), f"{viol:.2e}",
            fs, repr(phi), repr(phib), repr(f_sol), f"{slack:.3e}",
            gs, repr(gb), f"{F - gb:.3e}", f"{time.time() - t0:.1f}s"]))
    finally:
        shutil.rmtree(tmp)


main()
