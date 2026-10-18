"""Check that a BOBILib instance's high point relaxation (HPR) is exactly a
given single-level MIP, read independently of the Rust code.

usage: python3 -I compare_hpr.py BILEVEL.mps.gz BILEVEL.aux MIP.mps.gz

The HPR keeps every row of both levels, every bound and integrality
requirement, and the upper-level objective. If it equals MIP (same
variables, rows, coefficients, sides, bounds, integrality and objective, up
to the objective's sense), then MIP's optimum is a lower bound on the
bilevel optimum (BOBILib's upper level minimises), and a bilevel feasible
point attaining it is optimal. The AUX file is only used to confirm that it
names no variables or rows the MPS lacks.
"""
import gzip
import re
import sys
from fractions import Fraction


def read_mps(path):
    """Return (objective sense, objective, rows, bounds, integer set)."""
    opener = gzip.open if path.endswith(".gz") else open
    kind, coefs, rhs, ranges = {}, {}, {}, {}
    obj, lb, ub, integer = {}, {}, {}, set()
    obj_row, sense, sec, in_int = None, 1, None, False
    for line in opener(path, "rt"):
        if line.startswith("*") or not line.strip():
            continue
        t = line.split()
        if not line[0].isspace():
            sec = t[0]
            if sec == "OBJSENSE" and len(t) > 1:
                sense = -1 if t[1].upper().startswith("MAX") else 1
            continue
        if sec == "OBJSENSE":
            sense = -1 if t[0].upper().startswith("MAX") else 1
        elif sec == "ROWS":
            if t[0] == "N":
                obj_row = obj_row or t[1]
            else:
                kind[t[1]] = t[0]
                coefs[t[1]] = {}
        elif sec == "COLUMNS":
            if len(t) >= 3 and t[1].strip("'") == "MARKER":
                in_int = t[2].strip("'") == "INTORG"
                continue
            obj.setdefault(t[0], Fraction(0))
            if in_int:
                integer.add(t[0])
            for i in range(1, len(t) - 1, 2):
                v = Fraction(t[i + 1])
                if t[i] == obj_row:
                    obj[t[0]] += v
                elif v != 0:
                    coefs[t[i]][t[0]] = coefs[t[i]].get(t[0], Fraction(0)) + v
        elif sec in ("RHS", "RANGES"):
            rest = t[1:] if len(t) % 2 == 1 else t
            for i in range(0, len(rest) - 1, 2):
                if rest[i] != obj_row:
                    (rhs if sec == "RHS" else ranges)[rest[i]] = Fraction(rest[i + 1])
        elif sec == "BOUNDS":
            ty = t[0]
            needs = ty not in ("FR", "MI", "PL", "BV")
            col = t[2] if len(t) >= 3 else t[1]
            val = Fraction(t[3]) if needs and len(t) >= 4 else None
            if ty in ("UP", "UI"):
                ub[col] = val
                if val < 0 and lb.get(col, Fraction(0)) == 0:
                    lb[col] = None
            elif ty in ("LO", "LI"):
                lb[col] = val
            elif ty == "FX":
                lb[col] = ub[col] = val
            elif ty == "FR":
                lb[col] = ub[col] = None
            elif ty == "MI":
                lb[col] = None
            elif ty == "PL":
                ub[col] = None
            elif ty == "BV":
                lb[col], ub[col] = Fraction(0), Fraction(1)
                integer.add(col)
            if ty in ("UI", "LI"):
                integer.add(col)
    rows = {}
    for r, k in kind.items():
        b = rhs.get(r, Fraction(0))
        lo, hi = {"L": (None, b), "G": (b, None), "E": (b, b)}[k]
        if r in ranges:
            R = ranges[r]
            if k == "L":
                lo = b - abs(R)
            elif k == "G":
                hi = b + abs(R)
            else:
                lo, hi = (b, b + R) if R > 0 else (b + R, b)
        rows[r] = (lo, hi, coefs[r])
    bounds = {}
    for c in obj:
        # Integer columns with no upper bound default to 1 in some readers and
        # to infinity in others; compare what was written, not a default.
        bounds[c] = (lb.get(c, Fraction(0)), ub[c] if c in ub else ("default" if c in integer else None))
    return sense, obj, rows, bounds, integer


def main():
    bil, aux, mip = sys.argv[1:4]
    s1, o1, r1, b1, i1 = read_mps(bil)
    s2, o2, r2, b2, i2 = read_mps(mip)
    txt = open(aux).read()
    aux_vars = [l.split()[0] for l in re.search(r"@VARSBEGIN(.*)@VARSEND", txt, re.S).group(1).strip().splitlines()]
    aux_rows = re.search(r"@CONSTRSBEGIN(.*)@CONSTRSEND", txt, re.S).group(1).split()
    problems = []
    if set(aux_vars) - set(o1) or set(aux_rows) - set(r1):
        problems.append("AUX names unknown variables or rows")
    if set(o1) != set(o2):
        problems.append(f"variables differ: {len(set(o1) ^ set(o2))} names")
    if set(r1) != set(r2):
        problems.append(f"rows differ: {len(set(r1) ^ set(r2))} names")
    flip = s1 * s2  # the objectives must agree after converting both to minimisation
    for c in set(o1) & set(o2):
        if s1 * o1[c] != s2 * o2[c]:
            problems.append(f"objective of {c}: {o1[c]} vs {o2[c]}")
            break
    for r in set(r1) & set(r2):
        if r1[r] != r2[r]:
            problems.append(f"row {r} differs")
            break
    for c in set(b1) & set(b2):
        if b1[c] != b2[c]:
            problems.append(f"bounds of {c}: {b1[c]} vs {b2[c]}")
            break
    if i1 != i2:
        problems.append(f"integrality differs on {len(i1 ^ i2)} columns")
    print(f"{bil}: {len(o1)} vars, {len(r1)} rows, {len(aux_vars)} follower vars, {len(aux_rows)} follower rows; "
          f"senses {s1}/{s2}{' (flipped)' if flip < 0 else ''}")
    print("HPR == MIP" if not problems else "DIFFERENT: " + "; ".join(problems))
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
