"""Step (d), lower-bound side: a cutting-plane LP for tau4(G), the fewest edges meeting every K4.

Every vertex set S (s = |S|, e(S) edges of G inside S) gives a row that holds for every K4
transversal D, because D cap E(S) is a K4 transversal of G[S]:
  * Turan:   |D cap E(S)| >= e(S) - t_3(s), since a K4-free graph on s vertices has at most
             t_3(s) edges. For a K4 this is the covering row (6 - 5 = 1).
  * Brouwer: for s >= 7, |D cap E(S)| >= min(w3(G[S]), e(S) - t_3(s) + floor(s/3) - 1).
             If G[S] - D keeps at least t_3(s) - floor(s/3) + 2 edges it is 3-partite
             (Brouwer 1981, Thm 2), so D holds every inside edge of some 3-partition of S;
             otherwise D removed the rest. w3(G[S]) is computed exactly by w3_batch.
The LP starts from every K4 and every maximal clique (>= 5 vertices, where both rows coincide)
and separates violated rows by local search on the weights 1 - x_e, seeded from every
maximal clique and every set already found (in Brouwer mode, both with the Turan objective and
with the Brouwer one, which overestimates the violation where the w3 cap binds).

Any y >= 0 on the rows gives the bound y.b + sum_e min(0, 1 - (A^T y)_e) on tau4 (Lagrangian
of the box 0 <= x <= 1). The bound is recomputed in exact rationals from the floating-point
duals, so a final bound > k proves value 4. Otherwise the run records how far the LP got.

usage: python3 -I k4lp.py INSTANCE_NAME ROUNDS {turan|brouwer} W3_BIN WORKDIR > out.json
"""
import json
import os
import subprocess
import sys
import time
from fractions import Fraction

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402
import highspy  # noqa: E402
from k4cover import k4s, maximal_cliques  # noqa: E402


def t3_gain(s):
    """t_3(s + 1) - t_3(s): the new vertex joins a smallest part."""
    return s - s // 3


def bonus(s, brouwer):
    """What Brouwer adds to the Turan row at most (it is capped by w3)."""
    return s // 3 - 1 if brouwer and s >= 7 else 0


class Rows:
    def __init__(self, inst, brouwer, w3_bin, work):
        self.inst, self.brouwer, self.w3_bin, self.work = inst, brouwer, w3_bin, work
        self.eidx = {e: i for i, e in enumerate(inst.edges)}
        self.A = inst.adjacency()
        self.rows = []  # (sorted vertex tuple, edge indices, rhs)
        self.seen = set()

    def edges_in(self, S):
        return [self.eidx[(u, v)] for i, u in enumerate(S) for v in S[i + 1:] if self.A[u] >> v & 1]

    def rhs_many(self, sets):
        """Right-hand sides for sorted vertex tuples (exact w3 via w3_batch when needed)."""
        rhs = [len(self.edges_in(S)) - cl.turan(len(S), 3) for S in sets]
        big = [i for i, S in enumerate(sets) if len(S) >= 7]
        if not self.brouwer or not big:
            return rhs
        path = os.path.join(self.work, f"{self.inst.name}.w3in.txt")
        with open(path, "w") as fh:
            fh.write(f"{self.inst.n} {self.inst.m}\n")
            fh.writelines(f"{u} {v}\n" for u, v in self.inst.edges)
            fh.write(f"{len(big)}\n")
            for i in big:
                cap = max(rhs[i] + bonus(len(sets[i]), True), 0)
                fh.write(f"{len(sets[i])} {cap} {' '.join(map(str, sets[i]))}\n")
        out = subprocess.run([self.w3_bin, path], check=True, capture_output=True, text=True).stdout.split()
        w3cap = [int(v) for v in out[0::2]]
        assert len(w3cap) == len(big)
        # w3 >= e(S) - t_3(s) always, so the Brouwer row never falls below the Turan row.
        for i, w in zip(big, w3cap):
            rhs[i] = max(rhs[i], w)
        return rhs

    def add(self, sets, x=None):
        sets = [tuple(sorted(S)) for S in sets]
        sets = [S for S in dict.fromkeys(sets) if S not in self.seen]
        if not sets:
            return []
        added = []
        for S, rhs in zip(sets, self.rhs_many(sets)):
            e = self.edges_in(S)
            if rhs <= 0 or (x is not None and rhs - x[e].sum() <= 1e-6):
                continue
            self.seen.add(S)
            self.rows.append((S, e, rhs))
            added.append(S)
        return added


def separate(inst, x, seeds, max_new, brouwer):
    """Local search for S maximising sum_{E(S)} (1 - x_e) - t_3(s) + bonus(s): the violation of
    the row when its w3 cap does not bind (an upper bound on it otherwise)."""
    n = inst.n
    W = np.zeros((n, n))
    for i, (u, v) in enumerate(inst.edges):
        W[u, v] = W[v, u] = 1.0 - x[i]
    found = {}
    for seed in seeds:
        S = np.zeros(n, dtype=bool)
        S[list(seed)] = True
        s = int(S.sum())
        inner = W[:, S].sum(axis=1)  # weight from each vertex into S
        F = inner[S].sum() / 2 - cl.turan(s, 3) + bonus(s, brouwer)
        for _ in range(4 * n):
            ga = -t3_gain(s) + bonus(s + 1, brouwer) - bonus(s, brouwer)
            gr = t3_gain(s - 1) + bonus(s - 1, brouwer) - bonus(s, brouwer)
            add = np.where(S, -np.inf, inner + ga)
            rem = np.where(S, -inner + gr, -np.inf) if s > 4 else np.full(n, -np.inf)
            va, vr = add.argmax(), rem.argmax()
            if max(add[va], rem[vr]) <= 1e-9:
                break
            if add[va] >= rem[vr]:
                S[va] = True
                inner += W[:, va]
                F += add[va]
                s += 1
            else:
                S[vr] = False
                inner -= W[:, vr]
                F += rem[vr]
                s -= 1
        if F > 1e-6:
            key = tuple(np.flatnonzero(S))
            found[key] = max(found.get(key, 0.0), F)
    return [S for S, _ in sorted(found.items(), key=lambda kv: -kv[1])[:max_new]]


def exact_bound(inst, rows, y):
    """Lagrangian bound from duals y >= 0, in exact rational arithmetic."""
    col = [Fraction(0)] * inst.m
    val = Fraction(0)
    for (S, e, rhs), w in zip(rows.rows, y):
        w = Fraction(max(0.0, float(w)))
        if w == 0:
            continue
        val += w * rhs
        for j in e:
            col[j] += w
    return val + sum(min(Fraction(0), 1 - c) for c in col)


def main():
    name, rounds, mode, w3_bin, work = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5]
    assert mode in ("turan", "brouwer")
    os.makedirs(work, exist_ok=True)
    inst = cl.Instance(os.path.join(cl.SET_DIR, "clique", name + ".mps.gz"))
    t0 = time.time()
    rows = Rows(inst, mode == "brouwer", w3_bin, work)
    quads = k4s(inst)
    cliques = [tuple(c) for c in maximal_cliques(inst.adjacency(), 5)]
    rows.add(quads)
    rows.add(cliques)
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("threads", 1)
    inf = highspy.kHighsInf
    h.addVars(inst.m, np.zeros(inst.m), np.ones(inst.m))
    h.changeColsCost(inst.m, np.arange(inst.m, dtype=np.int32), np.ones(inst.m))

    def push(new_rows):
        lo = np.array([r[2] for r in new_rows], dtype=float)
        starts, idx = [], []
        for r in new_rows:
            starts.append(len(idx))
            idx.extend(r[1])
        h.addRows(len(new_rows), lo, np.full(len(new_rows), inf), len(idx), np.array(starts, dtype=np.int32),
                  np.array(idx, dtype=np.int32), np.ones(len(idx)))

    push(rows.rows)
    history, grown = [], []
    out = dict(inst=name, mode=mode, n=inst.n, m=inst.m, k=inst.k, k4=len(quads), maximal_cliques_ge5=len(cliques))
    for rnd in range(rounds + 1):
        h.run()
        assert h.getModelStatus() == highspy.HighsModelStatus.kOptimal, h.getModelStatus()
        sol = h.getSolution()
        x = np.array(sol.col_value)
        bound = exact_bound(inst, rows, sol.row_dual)
        history.append(dict(round=rnd, rows=len(rows.rows), lp=h.getInfo().objective_function_value,
                            exact_bound=float(bound), sec=round(time.time() - t0, 1)))
        print(json.dumps(history[-1]), file=sys.stderr, flush=True)
        if bound > inst.k or rnd == rounds:
            break
        # Turan-violated sets first (their Brouwer row can only be stronger), then sets that only
        # the Brouwer row might cut off.
        cand = separate(inst, x, cliques + grown, 3000, False)
        if mode == "brouwer":
            cand += separate(inst, x, cliques + grown, 3000, True)
        new = rows.add(cand, x)
        if not new:
            break
        grown.extend(new)
        push(rows.rows[-len(new):])
    sizes = [len(r[0]) for r in rows.rows if len(r[0]) > 4]
    out.update(history=history, final_lp=history[-1]["lp"], final_exact_bound=history[-1]["exact_bound"],
               proves_value_4=bool(bound > inst.k), row_sizes=[min(sizes), max(sizes)], sec=round(time.time() - t0, 1))
    if bound > inst.k:
        out["exact_bound_fraction"] = f"{bound.numerator}/{bound.denominator}"
    print(json.dumps(out), flush=True)


if __name__ == "__main__":
    main()
