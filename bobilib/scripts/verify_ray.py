"""Exact check that a BOBILib instance's follower is unbounded for every
leader choice, from a recession direction written by `bobi ray --out`.

usage: python3 -I verify_ray.py INSTANCE.mps.gz INSTANCE.aux RAY.json

The direction d (follower variables only) is rationalised and checked in
exact arithmetic against the instance read independently of the Rust code:
  * every follower row a.x <= hi has a.d <= 0, a.x >= lo has a.d >= 0
    (equality rows need a.d = 0), counting only follower variables, since
    leader variables enter follower rows as constants;
  * d_j >= 0 if follower variable j has a finite lower bound, d_j <= 0 if
    it has a finite upper bound;
  * the follower objective e.d < 0 (BOBILib followers minimise).
Then for every leader choice x the follower's region is either empty or
contains rays along d, and with rational data its mixed-integer hull has
the same recession cone (Meyer 1974), so the follower has no optimal
solution: the instance has no bilevel feasible point.
"""
import gzip
import json
import re
import sys
from fractions import Fraction


def read(mps, aux):
    txt = open(aux).read()
    ll_obj = {}
    for line in re.search(r"@VARSBEGIN(.*)@VARSEND", txt, re.S).group(1).strip().splitlines():
        t = line.split()
        ll_obj[t[0]] = Fraction(t[1]) if len(t) > 1 else Fraction(0)
    ll_rows = set(re.search(r"@CONSTRSBEGIN(.*)@CONSTRSEND", txt, re.S).group(1).split())
    kind, cols, rhs, ranges = {}, {}, {}, {}
    lb, ub, integer = {}, {}, set()
    obj_row = None
    sec, in_int = None, False
    for line in gzip.open(mps, "rt"):
        if line.startswith("*") or not line.strip():
            continue
        t = line.split()
        if not line[0].isspace():
            sec = t[0]
            continue
        if sec == "ROWS":
            if t[0] == "N":
                obj_row = obj_row or t[1]
            else:
                kind[t[1]] = t[0]
        elif sec == "COLUMNS":
            if len(t) >= 3 and t[1].strip("'") == "MARKER":
                in_int = t[2].strip("'") == "INTORG"
                continue
            cols.setdefault(t[0], {})
            if in_int:
                integer.add(t[0])
            for i in range(1, len(t), 2):
                if t[i] != obj_row:
                    cols[t[0]][t[i]] = Fraction(t[i + 1])
        elif sec in ("RHS", "RANGES"):
            rest = t[1:] if len(t) % 2 == 1 else t
            for i in range(0, len(rest), 2):
                (rhs if sec == "RHS" else ranges)[rest[i]] = Fraction(rest[i + 1])
        elif sec == "BOUNDS":
            ty = t[0]
            needs = ty not in ("FR", "MI", "PL", "BV")
            col = t[2] if (needs and len(t) == 4) or (not needs and len(t) >= 3) else t[1]
            val = Fraction(t[-1]) if needs or len(t) == 4 else None
            if ty in ("UP", "UI"):
                ub[col] = val
                if val < 0 and lb.get(col, Fraction(0)) == 0:
                    lb[col] = None
            elif ty in ("LO", "LI"):
                lb[col] = val
            elif ty == "FX":
                lb[col] = ub[col] = val
            elif ty == "FR":
                lb[col] = None
                ub[col] = None
            elif ty == "MI":
                lb[col] = None
            elif ty == "PL":
                ub[col] = None
            elif ty == "BV":
                lb[col], ub[col] = Fraction(0), Fraction(1)
    return ll_obj, ll_rows, kind, cols, rhs, ranges, lb, ub


def main():
    mps, aux, ray = sys.argv[1:4]
    ll_obj, ll_rows, kind, cols, rhs, ranges, lb, ub = read(mps, aux)
    d = {k: Fraction(v) if isinstance(v, str) else Fraction(v).limit_denominator(10**6) for k, v in json.load(open(ray)).items()}
    problems = []
    for j, v in d.items():
        if j not in ll_obj:
            problems.append(f"{j} is not a follower variable")
            continue
        lo = lb.get(j, Fraction(0))
        hi = ub.get(j)
        if lo is not None and v < 0:
            problems.append(f"{j}: d={v} < 0 under lower bound {lo}")
        if hi is not None and v > 0:
            problems.append(f"{j}: d={v} > 0 under upper bound {hi}")
    # Row activity along d, follower variables only.
    act = {r: Fraction(0) for r in ll_rows}
    for j, v in d.items():
        for r, a in cols.get(j, {}).items():
            if r in act:
                act[r] += a * v
    for r in ll_rows:
        k, a = kind[r], act[r]
        has_hi = k in ("L", "E") or (k == "G" and r in ranges)
        has_lo = k in ("G", "E") or (k == "L" and r in ranges)
        if k == "E" and r in ranges:
            has_lo = has_hi = True
        if has_hi and a > 0:
            problems.append(f"row {r} ({k}): a.d = {a} > 0")
        if has_lo and a < 0:
            problems.append(f"row {r} ({k}): a.d = {a} < 0")
    obj = sum(ll_obj[j] * v for j, v in d.items() if j in ll_obj)
    ok = not problems and obj < 0
    print(json.dumps({"instance": mps.split("/")[-1], "verified": ok, "objective_along_d": float(obj), "support": len(d), "problems": problems[:5]}))


if __name__ == "__main__":
    main()
