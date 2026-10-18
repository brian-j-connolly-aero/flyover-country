"""Independent check of subset-sum knapsack interdiction (KIP) lower bounds:
for every leader deletion within the budget B, some subset of the surviving
items has a sum in [lo, C] (C the follower's capacity; lo = C by default).
Then the bilevel optimum is at least lo. When the search reaches a leaf that
fails, it has found a deletion within B that keeps every sum out of [lo, C],
so the answer is exact either way.

It shares no code with the Rust solver (`bobi ss-solve`). It parses the MPS
and AUX files itself, checks that the instance is a KIP whose profits equal
its follower weights, and runs a plain depth-first search over how many
copies of each weight the leader deletes (the cheapest copies first). Items
the leader can delete for free are taken as deleted; items heavier than C
never fit. The search has only two rules:

  * direct: the decided survivors already reach a sum in [lo, C];
  * node table: the online game in which the follower plays the undecided
    classes one at a time, each after seeing the leader's deletion in that
    class, and must end with a residual that the decided survivors can
    complete to a total in [lo, C]. In threshold form (the largest budget at
    which the follower still wins from residual t),
        thr(t) = min over r (P(r) + max_{j <= n - r, j v <= t} thr'(t - j v)),
    capped at B, with base thr(t) = B if some decided sum s has
    t - (C - lo) <= s <= t, else -1. The node is safe if thr(C) is at least
    the remaining budget. P(r) is the cost of the r cheapest copies.

`selftest` compares the search with brute force over every subset of items
on small random instances (this also checks the reductions above).

Usage:
  python3 -I ss_check.py selftest
  python3 -I ss_check.py INSTANCE.mps.gz INSTANCE.aux [BUDGET|- [LO]]
"""
import gzip
import sys
from itertools import product

import numpy as np


def parse(mps_path, aux_path):
    rows = {}
    cols = {}
    rhs = {}
    bounds = {}
    sec = None
    opener = gzip.open if mps_path.endswith(".gz") else open
    with opener(mps_path, "rt", encoding="latin-1") as fh:
        for line in fh:
            if line.startswith("*") or not line.strip():
                continue
            if not line[0].isspace():
                sec = line.split()[0]
                continue
            t = line.split()
            if sec == "ROWS":
                rows[t[1]] = t[0]
            elif sec == "COLUMNS":
                if "'MARKER'" in t:
                    continue
                d = cols.setdefault(t[0], {})
                for i in range(1, len(t), 2):
                    d[t[i]] = float(t[i + 1])
            elif sec == "RHS":
                for i in range(1, len(t), 2):
                    rhs[t[i]] = float(t[i + 1])
            elif sec == "BOUNDS":
                bounds.setdefault(t[2], []).append((t[0], float(t[3]) if len(t) > 3 else None))
    fobj = {}
    fcons = []
    sec = None
    with open(aux_path) as fh:
        for line in fh:
            t = line.split()
            if not t:
                continue
            if t[0].startswith("@"):
                sec = t[0]
                continue
            if sec == "@VARSBEGIN":
                fobj[t[0]] = float(t[1])
            elif sec == "@CONSTRSBEGIN":
                fcons.append(t[0])
    return rows, cols, rhs, bounds, fobj, fcons


def kip_data(mps_path, aux_path):
    rows, cols, rhs, bounds, fobj, fcons = parse(mps_path, aux_path)
    yvars = set(fobj)
    xvars = [c for c in cols if c not in yvars]
    # Leader variables binary; follower variables binary, or integer with
    # lower bound 0 (the linking row x + y <= 1 caps them at 1).
    assert all(bounds.get(x) == [("BV", None)] for x in xvars), "leader binary"
    assert all(bounds.get(y) in ([("BV", None)], [("LI", 0.0)]) for y in yvars), "follower 0/1"
    lrows = [r for r, ty in rows.items() if ty != "N" and r not in fcons]
    assert len(lrows) == 1, f"one leader row, got {lrows}"
    lrow = lrows[0]
    assert rows[lrow] == "L"
    # The follower's knapsack row: the follower constraint with every y.
    krows = [r for r in fcons if sum(1 for y in yvars if r in cols[y]) == len(yvars)]
    assert len(krows) == 1, krows
    krow = krows[0]
    assert rows[krow] == "L"
    assert all(krow not in cols[x] for x in xvars)
    assert all(lrow not in cols[y] for y in yvars)
    B = rhs.get(lrow, 0.0)
    C = rhs.get(krow, 0.0)
    items = []
    for y in yvars:
        links = [r for r in cols[y] if r in fcons and r != krow]
        assert len(links) == 1, (y, links)
        r = links[0]
        assert rows[r] == "L" and rhs.get(r) == 1.0 and cols[y][r] == 1.0
        xs = [x for x in xvars if r in cols[x]]
        assert len(xs) == 1 and cols[xs[0]][r] == 1.0, (r, xs)
        # Nothing else in the linking row.
        assert sum(1 for c in cols if r in cols[c]) == 2
        x = xs[0]
        w = cols[y][krow]
        p = -fobj[y]
        cost = cols[x].get(lrow, 0.0)
        assert w == int(w) and cost == int(cost) and w >= 0 and cost >= 0
        assert p == w, "subset-sum instance: profit equals weight"
        items.append((int(w), int(cost)))
    # Every follower constraint is the knapsack row or a linking row.
    assert len(fcons) == 1 + len(yvars)
    return items, int(B), int(C)


def node_table(play, S, cap, C, lo=None):
    """thr(C) of the online game ending in A (bool array of 0..=C). For a
    band of targets [lo, C], A = residuals t with some s in S, t - (C - lo)
    <= s <= t: the follower then totals u + s in [lo, C]."""
    if lo is not None and lo < C:
        A = S.copy()
        for d in range(1, C - lo + 1):
            A[d:] |= S[: C + 1 - d]
        S = A
    thr = np.where(S, cap, -1).astype(np.int64)
    for v, costs in reversed(play):
        n = len(costs)
        P = [0]
        for c in costs:
            P.append(P[-1] + c)
        new = np.full(C + 1, cap, dtype=np.int64)
        for r in range(n + 1):
            if P[r] > cap:
                break
            M = thr.copy()
            for j in range(1, n - r + 1):
                d = j * v
                if d > C:
                    break
                M[d:] = np.maximum(M[d:], thr[: C + 1 - d])
            new = np.minimum(new, P[r] + M)
        thr = new
    return int(thr[C])


def add_copies(S, v, cnt, C):
    S = S.copy()
    for _ in range(cnt):
        if v > C:
            break
        T = S.copy()
        T[v:] |= S[: C + 1 - v]
        S = T
    return S


class Prover:
    def __init__(self, classes, B, C, lo=None):
        # classes: list of (value, sorted costs); search order ascending value.
        # Goal: some subset sum in [lo, C] (lo = C: exactly C).
        self.classes = sorted(classes)
        self.B = B
        self.C = C
        self.lo = C if lo is None else lo
        self.nodes = 0
        self.by_rule = {"direct": 0, "table": 0}
        m = len(self.classes)

        def ratio_key(i):
            v, costs = self.classes[i]
            # total cost per unit mass, ascending (cross-multiplied).
            return (sum(costs) / (v * len(costs)), v)

        self.play = [
            [self.classes[i] for i in sorted(range(k, m), key=ratio_key)] for k in range(m + 1)
        ]

    def safe(self, k, S, b):
        self.nodes += 1
        if S[self.lo : self.C + 1].any():
            self.by_rule["direct"] += 1
            return True
        if node_table(self.play[k], S, self.B, self.C, self.lo) >= b:
            self.by_rule["table"] += 1
            return True
        if k == len(self.classes):
            return False
        v, costs = self.classes[k]
        n = len(costs)
        spent = 0
        for r in range(n + 1):
            if r > 0:
                spent += costs[r - 1]
            if spent > b:
                break
            if not self.safe(k + 1, add_copies(S, v, n - r, self.C), b - spent):
                return False
        return True

    def run(self):
        S = np.zeros(self.C + 1, dtype=bool)
        S[0] = True
        return self.safe(0, S, self.B)


def classes_of(items, C):
    by = {}
    for w, c in items:
        if c == 0 or w == 0 or w > C:
            continue  # free (always deleted), worthless, or never fits
        by.setdefault(w, []).append(c)
    return [(w, sorted(cs)) for w, cs in by.items()]


def brute(items, B, C, lo=None):
    """Exhaustive: does every deletion within B leave a subset sum in [lo, C]?"""
    lo = C if lo is None else lo
    n = len(items)
    for X in product([0, 1], repeat=n):
        if sum(c for (w, c), x in zip(items, X) if x) > B:
            continue
        reach = 1
        for (w, c), x in zip(items, X):
            if not x:
                reach |= reach << w
        if not (reach >> lo) & ((1 << (C - lo + 1)) - 1):
            return False
    return True


def selftest(rounds=3000, seed=1):
    rng = np.random.default_rng(seed)
    safe_cases = 0
    table_prunes = 0
    for _ in range(rounds):
        n = int(rng.integers(1, 11))
        items = [(int(rng.integers(1, 9)), int(rng.integers(0, 8))) for _ in range(n)]
        total = sum(w for w, _ in items)
        C = int(rng.integers(0, total + 2))
        B = int(rng.integers(0, 25))
        lo = C - int(rng.integers(0, C // 2 + 1)) if rng.integers(0, 3) == 0 else C
        expect = brute(items, B, C, lo)
        p = Prover(classes_of(items, C), B, C, lo)
        got = p.run()
        # Branching to the leaves, the search is exact.
        assert got == expect, (items, B, C, lo, got, expect)
        safe_cases += got
        table_prunes += p.by_rule["table"]
    print(f"selftest ok: {rounds} rounds, {safe_cases} safe, {table_prunes} table prunes")


if __name__ == "__main__":
    if sys.argv[1] == "selftest":
        selftest()
    else:
        import time

        t0 = time.time()
        items, B, C = kip_data(sys.argv[1], sys.argv[2])
        if len(sys.argv) > 3 and sys.argv[3] != "-":
            B = int(sys.argv[3])  # budget override (experiments); "-" keeps the instance's
        lo = int(sys.argv[4]) if len(sys.argv) > 4 else C  # target band [lo, C]
        cl = classes_of(items, C)
        p = Prover(cl, B, C, lo)
        sys.setrecursionlimit(10000)
        ok = p.run()
        print(
            f"{sys.argv[1].split('/')[-1]}: n={len(items)} B={B} C={C} lo={lo} classes={len(cl)} "
            f"-> {'EVERY DELETION LEAVES A SUM IN [lo, C] (optimum >= lo)' if ok else 'NOT PROVEN'} "
            f"nodes={p.nodes} {p.by_rule} {time.time() - t0:.1f}s"
        )
