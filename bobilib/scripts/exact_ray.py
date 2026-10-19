"""Find an exact recession certificate for a BOBILib follower, independently
of the Rust code, and write it for verify_ray.py with rational strings.

usage: python3 -I exact_ray.py INSTANCE.mps.gz INSTANCE.aux OUT.json
           [--milp] [--bound M] [--time-limit SECONDS]

A certificate is a direction d on the follower variables with
  * a_F.d <= 0 on follower rows a.x <= hi, a_F.d >= 0 on rows a.x >= lo
    (both on equality rows), counting follower columns only;
  * d_j >= 0 where y_j has a finite lower bound, d_j <= 0 where it has a
    finite upper bound;
  * f.d < 0 for the follower objective f (BOBILib followers minimise).

Stage 1 (exact, no solver).  Every unit direction d = s e_j (s = +1 or -1,
s f_j < 0) is checked exactly: it is a certificate iff y_j is unbounded in
direction s and every follower row containing y_j allows the change s a_rj.
Such a direction is integral, so for every leader choice x, every feasible
follower point y gives feasible points y + t s e_j (t = 0, 1, 2, ...; any
t >= 0 when y_j is continuous) with objective f.y + t s f_j -> -infinity:
no optimal follower response exists and the Meyer (1974) argument is not
needed.  The column with the most negative s f_j is written (ties: first
continuous column in file order, then first integer column).

Stage 2 (fallback, or forced with --milp).  A HiGHS MILP minimises f.d over
integer d in [-M, M] with the row and sign conditions and f.d <= -1.  The
result is checked exactly; rows the floating solution violates by rounding
get a strict margin (a_F.d <= -1 or >= 1, possible because d is integral)
and the MILP is solved again, up to five rounds.
"""
import argparse
import gzip
import json
import os
import sys
from fractions import Fraction

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from verify_ray import read  # noqa: E402


def integer_columns(mps):
    """Names of the columns inside MARKER INTORG ... INTEND blocks."""
    ints, sec, in_int = set(), None, False
    for line in gzip.open(mps, "rt"):
        if line.startswith("*") or not line.strip():
            continue
        t = line.split()
        if not line[0].isspace():
            sec = t[0]
            continue
        if sec == "COLUMNS":
            if len(t) >= 3 and t[1].strip("'") == "MARKER":
                in_int = t[2].strip("'") == "INTORG"
            elif in_int:
                ints.add(t[0])
    return ints


def row_sides(r, kind, ranges):
    k = kind[r]
    has_hi = k in ("L", "E") or (k == "G" and r in ranges)
    has_lo = k in ("G", "E") or (k == "L" and r in ranges)
    return has_hi, has_lo


def exact_problems(d, inst):
    """Exact rational check of d; returns (list of problems, f.d)."""
    ll_obj, ll_rows, kind, cols, rhs, ranges, lb, ub = inst
    problems = []
    for j, v in d.items():
        if j not in ll_obj:
            problems.append(("var", j))
            continue
        if lb.get(j, Fraction(0)) is not None and v < 0:
            problems.append(("lb", j))
        if ub.get(j) is not None and v > 0:
            problems.append(("ub", j))
    act = {}
    for j, v in d.items():
        for r, a in cols.get(j, {}).items():
            if r in ll_rows:
                act[r] = act.get(r, Fraction(0)) + a * v
    for r, a in act.items():
        has_hi, has_lo = row_sides(r, kind, ranges)
        if has_hi and a > 0:
            problems.append(("hi", r))
        if has_lo and a < 0:
            problems.append(("lo", r))
    obj = sum((ll_obj[j] * v for j, v in d.items() if j in ll_obj), Fraction(0))
    return problems, obj


def unit_directions(inst, ints):
    """All exact unit certificates (s f_j, integer?, file position, j, s)."""
    ll_obj, ll_rows, kind, cols, rhs, ranges, lb, ub = inst
    order = {j: i for i, j in enumerate(cols)}
    found = []
    for j, f in ll_obj.items():
        for s in (1, -1):
            if s * f >= 0:
                continue
            if s > 0 and ub.get(j) is not None:
                continue
            if s < 0 and lb.get(j, Fraction(0)) is not None:
                continue
            ok = True
            for r, a in cols.get(j, {}).items():
                if r not in ll_rows:
                    continue
                has_hi, has_lo = row_sides(r, kind, ranges)
                if (has_hi and s * a > 0) or (has_lo and s * a < 0):
                    ok = False
                    break
            if ok:
                found.append((s * f, j in ints, order.get(j, len(order)), j, s))
    found.sort()
    return found


def milp_direction(inst, bound, time_limit):
    import highspy
    import numpy as np

    ll_obj, ll_rows, kind, cols, rhs, ranges, lb, ub = inst
    names = sorted(ll_obj)
    idx = {j: i for i, j in enumerate(names)}
    rows = sorted(ll_rows)
    ridx = {r: i for i, r in enumerate(rows)}
    entries = [[] for _ in rows]
    for j in names:
        for r, a in cols.get(j, {}).items():
            if r in ridx:
                entries[ridx[r]].append((idx[j], float(a)))
    margin_hi, margin_lo = set(), set()
    for rnd in range(5):
        h = highspy.Highs()
        h.setOptionValue("output_flag", False)
        h.setOptionValue("threads", 1)
        h.setOptionValue("time_limit", float(time_limit))
        inf = highspy.kHighsInf
        for j in names:
            lo = 0.0 if lb.get(j, Fraction(0)) is not None else -bound
            hi = 0.0 if ub.get(j) is not None else bound
            h.addVar(lo, hi)
        h.changeColsCost(len(names), np.arange(len(names), dtype=np.int32), np.array([float(ll_obj[j]) for j in names]))
        h.changeColsIntegrality(len(names), np.arange(len(names), dtype=np.int32), np.array([highspy.HighsVarType.kInteger] * len(names)))
        for r, ent in zip(rows, entries):
            if not ent:
                continue
            has_hi, has_lo = row_sides(r, kind, ranges)
            lo = (1.0 if r in margin_lo else 0.0) if has_lo else -inf
            hi = (-1.0 if r in margin_hi else 0.0) if has_hi else inf
            h.addRow(lo, hi, len(ent), np.array([e[0] for e in ent], dtype=np.int32), np.array([e[1] for e in ent]))
        h.addRow(-inf, -1.0, len(names), np.arange(len(names), dtype=np.int32), np.array([float(ll_obj[j]) for j in names]))
        h.run()
        sol = h.getSolution().col_value
        if not len(sol) or h.getModelStatus() not in (highspy.HighsModelStatus.kOptimal, highspy.HighsModelStatus.kTimeLimit):
            return None, f"MILP status {h.modelStatusToString(h.getModelStatus())}"
        d = {j: Fraction(round(sol[idx[j]])) for j in names if round(sol[idx[j]]) != 0}
        problems, obj = exact_problems(d, inst)
        if not problems and obj < 0:
            return d, f"MILP round {rnd + 1}"
        if not problems:
            return None, f"MILP round {rnd + 1}: no direction with f.d < 0 (status {h.modelStatusToString(h.getModelStatus())})"
        for kind_, r in problems:
            if kind_ == "hi":
                margin_hi.add(r)
            elif kind_ == "lo":
                margin_lo.add(r)
            else:
                return None, f"MILP solution breaks a bound: {kind_} {r}"
    return None, "MILP: no exact direction after 5 rounds"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mps")
    ap.add_argument("aux")
    ap.add_argument("out")
    ap.add_argument("--milp", action="store_true", help="skip stage 1")
    ap.add_argument("--bound", type=float, default=100.0)
    ap.add_argument("--time-limit", type=float, default=300.0)
    args = ap.parse_args()
    inst = read(args.mps, args.aux)
    d, how = None, None
    if not args.milp:
        found = unit_directions(inst, integer_columns(args.mps))
        if found:
            val, is_int, _, j, s = found[0]
            d = {j: Fraction(s)}
            how = f"unit direction ({len(found)} unit certificates exist; {'integer' if is_int else 'continuous'} column)"
    if d is None:
        d, how = milp_direction(inst, args.bound, args.time_limit)
    if d is None:
        print(json.dumps({"instance": os.path.basename(args.mps), "found": False, "reason": how}))
        sys.exit(1)
    problems, obj = exact_problems(d, inst)
    assert not problems and obj < 0
    json.dump({j: str(v) for j, v in sorted(d.items())}, open(args.out, "w"))
    print(json.dumps({"instance": os.path.basename(args.mps), "found": True, "method": how, "support": len(d), "objective_along_d": str(obj)}))


if __name__ == "__main__":
    main()
