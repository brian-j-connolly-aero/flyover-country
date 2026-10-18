"""Exactly repair a nearly valid recession direction on a small support:
rows the direction leaves (nearly) tight become exact equalities, and the
direction is replaced by its projection onto their rational null space.
Writes the repaired direction for verify_ray.py.

usage: python3 -I repair_ray.py INSTANCE.mps.gz INSTANCE.aux RAY.json OUT.json
"""
import json
import sys
from fractions import Fraction

sys.path.insert(0, __import__("os").path.dirname(__file__))
from verify_ray import read  # noqa: E402


def nullspace(rows, n):
    """Rational basis of {z : rows . z = 0} by Gaussian elimination."""
    m = [r[:] for r in rows]
    piv, r = [], 0
    for c in range(n):
        p = next((i for i in range(r, len(m)) if m[i][c] != 0), None)
        if p is None:
            continue
        m[r], m[p] = m[p], m[r]
        inv = 1 / m[r][c]
        m[r] = [x * inv for x in m[r]]
        for i in range(len(m)):
            if i != r and m[i][c] != 0:
                f = m[i][c]
                m[i] = [a - f * b for a, b in zip(m[i], m[r])]
        piv.append(c)
        r += 1
    free = [c for c in range(n) if c not in piv]
    basis = []
    for f in free:
        z = [Fraction(0)] * n
        z[f] = Fraction(1)
        for i, c in enumerate(piv):
            z[c] = -m[i][f]
        basis.append(z)
    return basis


def main():
    mps, aux, ray, out = sys.argv[1:5]
    ll_obj, ll_rows, kind, cols, rhs, ranges, lb, ub = read(mps, aux)
    d = {k: Fraction(v) for k, v in json.load(open(ray)).items()}
    names = sorted(d)
    idx = {j: i for i, j in enumerate(names)}
    rows = {}
    for j in names:
        for r, a in cols.get(j, {}).items():
            if r in ll_rows:
                rows.setdefault(r, [Fraction(0)] * len(names))[idx[j]] = a
    scale = max(abs(v) for v in d.values())
    tight = [vec for r, vec in rows.items() if abs(sum(a * d[j] for a, j in zip(vec, names))) <= Fraction(1, 10**9) * scale]
    basis = nullspace(tight, len(names))
    # Project d onto span(basis) by solving the normal equations exactly.
    dv = [d[j] for j in names]
    G = [[sum(a * b for a, b in zip(u, v)) for v in basis] for u in basis]
    h = [sum(a * b for a, b in zip(u, dv)) for u in basis]
    k = len(basis)
    A = [G[i] + [h[i]] for i in range(k)]
    for c in range(k):
        p = next(i for i in range(c, k) if A[i][c] != 0)
        A[c], A[p] = A[p], A[c]
        A[c] = [x / A[c][c] for x in A[c]]
        for i in range(k):
            if i != c and A[i][c] != 0:
                f = A[i][c]
                A[i] = [a - f * b for a, b in zip(A[i], A[c])]
    z = [A[i][k] for i in range(k)]
    new = [sum(z[i] * basis[i][t] for i in range(k)) for t in range(len(names))]
    json.dump({j: float(v) if v.denominator <= 10**6 else str(v) for j, v in zip(names, new)}, open(out, "w"))
    print(f"{len(tight)} tight rows, null space dimension {k}, repaired direction written")


if __name__ == "__main__":
    main()
