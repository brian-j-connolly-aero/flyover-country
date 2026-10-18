"""Step (d): the exact question behind value 3 on a density-0.7 instance.

By Mantel the value is at least 3, and it is exactly 3 iff some D with |D| <= k meets every K4
of G (an edge K4-transversal): G - D is then K4-free, so its maximum clique is 3. A K4-free
graph need not be 3-partite at these edge counts (m - k is far below Brouwer's threshold), so
the fewest inside edges of a 3-partition (w3) only bounds the transversal number tau4 from above.

This script enumerates every K4 of G and
  * solves the LP relaxation of min |D| s.t. |D cap E(Q)| >= 1 for every K4 Q, optionally with
    the valid clique rows |D cap E(Q)| >= C(q,2) - t_3(q) for the maximal cliques Q of G
    (a K_q keeps at most t_3(q) edges when K4-free), giving a lower bound on tau4;
  * runs CP-SAT on the exact set-cover model (minimise |D|), hinted with the best
    3-partition's inside edges, to look for a transversal within budget (|D| <= k).

usage: python3 -I k4cover.py INSTANCE_NAME PARTITION SECONDS LOGFILE > out.json
PARTITION is a digit string (part of each vertex) or an rcut_tabu output file.
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def k4s(inst):
    A = inst.adjacency()
    out = []
    for a in range(inst.n):
        Pa = A[a] >> (a + 1) << (a + 1)
        while Pa:
            b = (Pa & -Pa).bit_length() - 1
            Pa &= Pa - 1
            Pb = Pa & A[b]
            while Pb:
                c = (Pb & -Pb).bit_length() - 1
                Pb &= Pb - 1
                Pc = Pb & A[c]
                while Pc:
                    d = (Pc & -Pc).bit_length() - 1
                    Pc &= Pc - 1
                    out.append((a, b, c, d))
    return out


def maximal_cliques(A, minsize):
    """Bron-Kerbosch with pivoting; yields maximal cliques with at least minsize vertices."""
    def rec(R, P, X):
        if not P and not X:
            if len(R) >= minsize:
                yield R
            return
        u = max(cl_bits(P | X), key=lambda w: cl.popcount(P & A[w]))
        cand = P & ~A[u]
        while cand:
            v = (cand & -cand).bit_length() - 1
            cand &= cand - 1
            yield from rec(R + [v], P & A[v], X & A[v])
            P &= ~(1 << v)
            X |= 1 << v

    def cl_bits(S):
        while S:
            v = (S & -S).bit_length() - 1
            S &= S - 1
            yield v

    yield from rec([], (1 << len(A)) - 1, 0)


def lp_bound(inst, eidx, quads, cliques):
    import numpy as np
    from scipy.optimize import linprog
    from scipy.sparse import csr_matrix

    rows, cols, rhs = [], [], []
    r = 0
    for q in quads:
        for i in range(4):
            for j in range(i + 1, 4):
                rows.append(r)
                cols.append(eidx[(q[i], q[j])])
        rhs.append(1.0)
        r += 1
    for Q in cliques:
        Q = sorted(Q)
        for i in range(len(Q)):
            for j in range(i + 1, len(Q)):
                rows.append(r)
                cols.append(eidx[(Q[i], Q[j])])
        rhs.append(float(len(Q) * (len(Q) - 1) // 2 - cl.turan(len(Q), 3)))
        r += 1
    A = csr_matrix((-np.ones(len(rows)), (rows, cols)), shape=(r, inst.m))
    res = linprog(np.ones(inst.m), A_ub=A, b_ub=-np.array(rhs), bounds=(0, 1), method="highs")
    assert res.status == 0, res.message
    return res.fun


def main():
    name, partition, seconds, log_path = sys.argv[1], sys.argv[2], float(sys.argv[3]), sys.argv[4]
    inst = cl.Instance(os.path.join(cl.SET_DIR, "clique", name + ".mps.gz"))
    eidx = {e: i for i, e in enumerate(inst.edges)}
    t0 = time.time()
    quads = k4s(inst)
    out = dict(inst=name, n=inst.n, m=inst.m, k=inst.k, k4_count=len(quads))
    A = inst.adjacency()
    big = list(maximal_cliques(A, 5))
    out["maximal_cliques_ge5"] = len(big)
    out["lp_k4"] = lp_bound(inst, eidx, quads, [])
    out["lp_k4_cliques"] = lp_bound(inst, eidx, quads, big)
    if os.path.exists(partition):
        # rcut_tabu output: the last line is "best <inside> <p_0> ... <p_{n-1}>".
        with open(partition) as fh:
            partition = "".join(fh.read().strip().splitlines()[-1].split()[2:])
    part = [int(c) for c in partition]
    hint = {eidx[e] for e in cl.inside_edges(inst, part)}
    out["hint_size"] = len(hint)

    from ortools.sat.python import cp_model

    md = cp_model.CpModel()
    x = [md.NewBoolVar(f"d{u},{v}") for (u, v) in inst.edges]
    for (a, b, c, d) in quads:
        md.AddBoolOr([x[eidx[p]] for p in ((a, b), (a, c), (a, d), (b, c), (b, d), (c, d))])
    for Q in big:
        Q = sorted(Q)
        md.Add(sum(x[eidx[(Q[i], Q[j])]] for i in range(len(Q)) for j in range(i + 1, len(Q)))
               >= len(Q) * (len(Q) - 1) // 2 - cl.turan(len(Q), 3))
    # No hard budget row: the hint (a 3-partition's inside edges) stays feasible and LNS works from it.
    md.Minimize(sum(x))
    for i in range(inst.m):
        md.AddHint(x[i], i in hint)
    s = cp_model.CpSolver()
    s.parameters.num_workers = 2
    s.parameters.max_time_in_seconds = seconds
    s.parameters.log_search_progress = True
    s.parameters.log_to_stdout = False
    with open(log_path, "w") as fh:
        s.log_callback = lambda line: fh.write(line + "\n")
        st = s.Solve(md)
    out["cpsat_status"] = s.StatusName(st)
    out["cpsat_bound"] = s.BestObjectiveBound()
    if st in (cp_model.OPTIMAL, cp_model.FEASIBLE):
        D = [inst.edges[i] for i in range(inst.m) if s.Value(x[i])]
        H = inst.adjacency(D)
        out["transversal"] = len(D)
        out["remainder_has_k4"] = cl.has_clique(H, 4)
        out["remainder_omega"] = len(cl.max_clique(H))
        out["deleted"] = [f"{u},{v}" for (u, v) in D]
        if len(D) <= inst.k and not out["remainder_has_k4"]:
            # Within budget: this leader choice is a better certificate than any 4-partition.
            clique = cl.max_clique(H)
            val, _ = cl.write_solution(inst, D, clique, os.path.join(cl.RESULTS, name + ".res.json"))
            assert inst.max_violation(val)[0] == 0.0
    out["sec"] = round(time.time() - t0, 1)
    print(json.dumps(out), flush=True)


if __name__ == "__main__":
    main()
