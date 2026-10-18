"""Step (e): the small sets bcpins / plusbcpins (n <= 15), which BOBILib lists as optimal.

For each instance this computes what the arguments used on 'clique' give:
  * lower bound: Turan (m - k > t_r(n) gives value >= r + 1) or Brouwer (n >= 2r + 1,
    m - k >= t_r(n) - floor(n/r) + 2 and w_r(G) > k give value >= r + 1), where w_r(G) is the
    fewest edges inside the parts of an r-partition (parts may be empty) and "w_r(G) <= k?"
    is decided by CP-SAT;
  * upper bound: w_r(G) <= k gives value <= r;
and then the exact value: when the bounds meet that is it; otherwise, from the lower bound t
upwards, CP-SAT decides whether some D with |D| <= k meets every K_{t+1} of G (the decision
version; proving the optimisation version from scratch is slow even at n = 12 because the
hitting-set LP is weak). Compared with BOBILib's value. The r-partition for the upper bound
comes from rcut_tabu and is recounted here.

usage: python3 -I small_exact.py SUBSET TABU_BIN WORKDIR > out.jsonl
"""
import itertools
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402
from ortools.sat.python import cp_model  # noqa: E402


def solve(md, seconds=300):
    s = cp_model.CpSolver()
    s.parameters.num_workers = 1
    s.parameters.max_time_in_seconds = seconds
    st = s.Solve(md)
    return s, s.StatusName(st)


def cliques_of_size(A, q):
    n = len(A)
    return [S for S in itertools.combinations(range(n), q)
            if all(A[u] >> v & 1 for u, v in itertools.combinations(S, 2))]


def hits_all(inst, q):
    """Is there D with |D| <= k meeting every q-clique of G? Returns (status, D)."""
    eidx = {e: i for i, e in enumerate(inst.edges)}
    md = cp_model.CpModel()
    x = [md.NewBoolVar("") for _ in inst.edges]
    for S in cliques_of_size(inst.adjacency(), q):
        md.AddBoolOr([x[eidx[e]] for e in itertools.combinations(S, 2)])
    md.Add(sum(x) <= inst.k)
    s, st = solve(md)
    if st in ("FEASIBLE", "OPTIMAL"):
        D = [inst.edges[i] for i in range(inst.m) if s.Value(x[i])]
        assert len(D) <= inst.k and not cl.has_clique(inst.adjacency(D), q)
        return "FEASIBLE", D
    return st, None


def partition_within(inst, r):
    """Decide whether some r-partition (parts may be empty) has at most k edges inside its parts.
    Returns the CP-SAT status and, when feasible, the inside-edge count of the partition found.
    The decision version is used because proving the optimum of max r-cut is slow at n = 15."""
    md = cp_model.CpModel()
    x = [[md.NewBoolVar("") for _ in range(r)] for _ in range(inst.n)]
    for v in range(inst.n):
        md.AddExactlyOne(x[v])
    md.Add(x[0][0] == 1)
    same = []
    for (u, v) in inst.edges:
        z = md.NewBoolVar("")
        for i in range(r):
            md.AddBoolOr([x[u][i].Not(), x[v][i].Not(), z])
        same.append(z)
    md.Add(sum(same) <= inst.k)
    s, st = solve(md)
    if st in ("FEASIBLE", "OPTIMAL"):
        part = [next(i for i in range(r) if s.Value(x[v][i])) for v in range(inst.n)]
        inside = len(cl.inside_edges(inst, part))
        assert inside <= inst.k
        return "FEASIBLE", inside
    return st, None


def tabu_partition(inst, r, binary, work):
    """An r-partition from rcut_tabu (stops once at most k edges are inside); recounted here."""
    g = os.path.join(work, inst.name + ".txt")
    with open(g, "w") as fh:
        fh.write(f"{inst.n} {inst.m}\n")
        fh.writelines(f"{u} {v}\n" for u, v in inst.edges)
    out = subprocess.run([binary, g, str(r), "20000", "20", "1", str(inst.k)], check=True,
                         capture_output=True, text=True).stdout.strip().splitlines()[-1].split()
    part = [int(t) for t in out[2:]]
    return len(cl.inside_edges(inst, part)), part


def main():
    sub, binary, work = sys.argv[1:4]
    os.makedirs(work, exist_ok=True)
    meta = cl.load_meta()
    paths = sorted(cl.instance_paths(sub), key=lambda p: cl.sort_key(os.path.basename(p)))
    for p in paths:
        inst = cl.load(p)
        n, m, k = inst.n, inst.m, inst.k
        rest = m - k
        omega = len(cl.max_clique(inst.adjacency()))
        lb, lb_why, brouwer = 1, "trivial", None
        for r in range(1, omega):
            if rest > cl.turan(n, r):
                lb, lb_why = r + 1, f"Turan t_{r}"
            elif n >= 2 * r + 1 and rest >= cl.turan(n, r) - n // r + 2:
                # Brouwer applies at r; it gives r + 1 only if no r-partition has <= k inside.
                brouwer = (r, partition_within(inst, r)[0])
                if brouwer[1] == "INFEASIBLE":
                    lb, lb_why = r + 1, f"Brouwer r={r}"
                break
            else:
                break
        inside, part = tabu_partition(inst, lb, binary, work)
        ub, ub_why = omega, "no deletion"
        if inside <= k:
            D = cl.inside_edges(inst, part)
            ub, ub_why = len(cl.max_clique(inst.adjacency(D))), f"{lb}-partition, {inside} inside"
        # Exact value: the arguments meet, or decide level by level above the proven bound.
        exact, trail = (lb, []) if ub == lb else (None, [])
        if exact is None:
            for t in range(lb, ub):
                st, _ = hits_all(inst, t + 1)
                trail.append((t, st))
                if st == "FEASIBLE":
                    exact = t
                    break
                if st != "INFEASIBLE":
                    break
            else:
                exact = ub
        bob = meta[inst.name]["Best known objective"]
        print(json.dumps(dict(inst=inst.name, n=n, m=m, k=k, omega_G=omega, exact=exact, trail=trail,
                              bobilib=bob, bobilib_status=meta[inst.name]["Status"],
                              consistent=exact == bob, arg_lb=lb, arg_lb_why=lb_why, brouwer=brouwer,
                              arg_ub=ub, arg_ub_why=ub_why, settled_by_args=lb == ub, notes=inst.notes)),
              flush=True)


if __name__ == "__main__":
    main()
