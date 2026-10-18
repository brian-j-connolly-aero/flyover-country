"""Step (c): lower bounds.

Arithmetic (exact integers, every instance):
  Mantel:  m - k > floor(n^2/4)  =>  every remainder G - D (|D| <= k) has a triangle, value >= 3.
  Brouwer (1981, Thm 2, complement form; Tyomkyn-Uzzell 2015, Thm 1.1): a K4-free graph on
  n >= 7 vertices with at least t_3(n) - floor(n/3) + 2 edges is 3-partite (3-colourable,
  parts may be empty). If m - k reaches that, a K4-free remainder G - D would be 3-partite, so D
  would contain every edge inside the parts of some 3-partition: |D| >= w3(G), the fewest edges
  of G inside the parts of a 3-partition. So w3(G) > k gives value >= 4.

Exact w3(G) > k (density-0.9 instances). For a partition with part sizes s,
  inside_G(P) = sum_i C(s_i, 2) - inside_Gbar(P) = sum_i C(s_i, 2) - |Ebar| + cut_Gbar(P),
since every pair inside a part is an edge or a non-edge. So inside_G(P) <= k needs
  cut_Gbar(P) <= b(s) := k + |Ebar| - sum_i C(s_i, 2),
and b(s) < 0 rules a size vector out at once. The script enumerates every size vector
s_1 >= s_2 >= s_3 >= 0 with b(s) >= 0 and asks two solvers, on two models, whether the
complement can be split with those sizes cutting at most b(s) of its edges:
  * CP-SAT: feasibility model, parts of equal size ordered by their smallest vertex.
  * SCIP: linear model (c_uv >= x_ui - x_vi), no symmetry breaking, sum c <= b(s).
Both must report infeasible. Logs go to LOGDIR.

usage: python3 -I lb.py SUBSET LOGDIR [instances...] > lb.jsonl
"""
import json
import os
import random
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402

THREADS = 2
TIME = 3600.0


def c2(s):
    return s * (s - 1) // 2


def size_vectors(n, budget_of):
    out = []
    for s1 in range(n + 1):
        for s2 in range(min(s1, n - s1) + 1):
            s3 = n - s1 - s2
            if s3 > s2:
                continue
            b = budget_of((s1, s2, s3))
            if b >= 0:
                out.append(((s1, s2, s3), b))
    return out


def cpsat_feasible(n, ebar, sizes, b, log_path):
    from ortools.sat.python import cp_model

    md = cp_model.CpModel()
    x = [[md.NewBoolVar(f"x{v}_{i}") for i in range(3)] for v in range(n)]
    for v in range(n):
        md.AddExactlyOne(x[v])
    for i in range(3):
        md.Add(sum(x[v][i] for v in range(n)) == sizes[i])
    cut = []
    for (u, v) in ebar:
        c = md.NewBoolVar(f"c{u}_{v}")
        for i in range(3):
            # same part on both ends, or the pair is cut
            md.AddBoolOr([x[u][i].Not(), x[v][i], c])
            md.AddBoolOr([x[v][i].Not(), x[u][i], c])
        cut.append(c)
    md.Add(sum(cut) <= b)
    # Parts with equal sizes are interchangeable: order them by their smallest vertex.
    # Vertex v may open part j (> i, same size) only if part i already holds a vertex < v.
    for i in range(3):
        for j in range(i + 1, 3):
            if sizes[i] != sizes[j]:
                continue
            for v in range(n):
                md.AddBoolOr([x[v][j].Not()] + [x[u][i] for u in range(v)])
    s = cp_model.CpSolver()
    s.parameters.num_workers = THREADS
    s.parameters.max_time_in_seconds = TIME
    s.parameters.log_search_progress = True
    s.parameters.log_to_stdout = False
    with open(log_path, "w") as fh:
        s.log_callback = lambda line: fh.write(line + "\n")
        st = s.Solve(md)
    name = s.StatusName(st)
    sol = None
    if name in ("FEASIBLE", "OPTIMAL"):
        sol = [next(i for i in range(3) if s.Value(x[v][i])) for v in range(n)]
    return name, round(s.WallTime(), 2), sol


def scip_feasible(n, ebar, sizes, b, log_path):
    from pyscipopt import Model, quicksum

    md = Model()
    md.setLogfile(log_path)
    md.hideOutput(True)  # quiet on stdout; the log file still gets everything
    md.setParam("limits/time", TIME)
    md.setParam("parallel/maxnthreads", 1)
    x = [[md.addVar(vtype="B", name=f"x{v}_{i}") for i in range(3)] for v in range(n)]
    for v in range(n):
        md.addCons(quicksum(x[v]) == 1)
    for i in range(3):
        md.addCons(quicksum(x[v][i] for v in range(n)) == sizes[i])
    cut = []
    for (u, v) in ebar:
        c = md.addVar(vtype="B", name=f"c{u}_{v}")
        for i in range(3):
            md.addCons(c >= x[u][i] - x[v][i])
            md.addCons(c >= x[v][i] - x[u][i])
        cut.append(c)
    md.addCons(quicksum(cut) <= b)
    md.setObjective(quicksum(cut), "minimize")
    t0 = time.time()
    md.optimize()
    st = md.getStatus()
    best = md.getObjVal() if md.getNSols() else None
    return st, best, round(time.time() - t0, 2)


def main():
    sub, logdir = sys.argv[1], sys.argv[2]
    only = set(sys.argv[3:])
    os.makedirs(logdir, exist_ok=True)
    paths = sorted(cl.instance_paths(sub), key=lambda p: cl.sort_key(os.path.basename(p)))
    for p in paths:
        inst = cl.Instance(p)
        if only and inst.name not in only:
            continue
        n, m, k = inst.n, inst.m, inst.k
        nb = len(inst.nonedges)
        rest = m - k
        brouwer_need = cl.turan(n, 3) - n // 3 + 2
        res = dict(inst=inst.name, n=n, m=m, k=k, nonedges=nb, remaining=rest,
                   mantel=n * n // 4, mantel_ok=rest > n * n // 4,
                   turan3=cl.turan(n, 3), turan3_ok=rest > cl.turan(n, 3),
                   brouwer_need=brouwer_need, brouwer_ok=(n >= 7 and rest >= brouwer_need))
        assert cl.turan(n, 2) == n * n // 4
        # Brouwer states Theorem 2 for the complement: alpha(G') <= t and e(G') <= T(n,t) + q - 2,
        # q = floor(n/t), T(n,t) = ((q+1)t - n) C(q,2) + (n - qt) C(q+1,2). Translate it here.
        q = n // 3
        T = ((q + 1) * 3 - n) * c2(q) + (n - q * 3) * c2(q + 1)
        assert T + cl.turan(n, 3) == c2(n) and c2(n) - (T + q - 2) == brouwer_need
        res["brouwer_T"] = T
        # The reduction rests on one counting identity; spot-check it on random partitions.
        rng = random.Random(inst.name)
        for _ in range(200):
            part = [rng.randrange(3) for _ in range(n)]
            sizes = [part.count(i) for i in range(3)]
            cut = sum(part[u] != part[v] for (u, v) in inst.nonedges)
            assert len(cl.inside_edges(inst, part)) == sum(c2(t) for t in sizes) - nb + cut
        if res["brouwer_ok"]:
            vecs = size_vectors(n, lambda s: k + nb - sum(c2(t) for t in s))
            rows = []
            all_cp, all_scip = True, True
            for sizes, b in vecs:
                tag = f"{inst.name}_{sizes[0]}-{sizes[1]}-{sizes[2]}"
                cst, ctime, sol = cpsat_feasible(n, inst.nonedges, sizes, b, os.path.join(logdir, tag + ".cpsat.log"))
                sst, sbest, stime = scip_feasible(n, inst.nonedges, sizes, b,
                                                  os.path.join(logdir, tag + ".scip.log"))
                if sol is not None:
                    # A counterexample would refute the bound: recount it directly on G.
                    res.setdefault("counterexample", []).append(dict(sizes=sizes, inside=len(cl.inside_edges(inst, sol))))
                cp_ok = cst == "INFEASIBLE"
                scip_ok = sst == "infeasible" and sbest is None
                all_cp &= cp_ok
                all_scip &= scip_ok
                rows.append(dict(sizes=sizes, budget=b, cpsat=cst, cpsat_sec=ctime, scip=sst,
                                 scip_solution=sbest, scip_sec=stime))
            res.update(size_vectors=rows, w3_gt_k_cpsat=all_cp, w3_gt_k_scip=all_scip,
                       lb4_proved=bool(all_cp and all_scip))
        print(json.dumps(res), flush=True)


if __name__ == "__main__":
    main()
