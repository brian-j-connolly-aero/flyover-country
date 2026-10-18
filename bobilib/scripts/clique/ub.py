"""Step (b): upper bounds. For each instance find an r-partition (r = 3, else 4) with at most k
edges inside its parts by tabu search (rcut_tabu), delete exactly those edges, compute the
remaining graph's maximum clique exactly, and write a BOBILib solution file.

The certificate is checked three ways here: the deleted set is within budget, the follower's
clique is a clique of the remaining graph, and an exhaustive search finds no clique one larger.
Every row and bound of the instance is checked by cliquelib.Instance.max_violation.

usage: python3 -I ub.py SUBSET GRAPH_DIR TABU_BIN OUT_DIR [ITERS RUNS] > ub.jsonl
"""
import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def tabu(binary, graph, r, iters, runs, seed, target):
    out = subprocess.run([binary, graph, str(r), str(iters), str(runs), str(seed), str(target)],
                         check=True, capture_output=True, text=True).stdout
    last = out.strip().splitlines()[-1].split()
    assert last[0] == "best"
    return int(last[1]), [int(t) for t in last[2:]]


def main():
    sub, gdir, binary, outdir = sys.argv[1:5]
    iters = int(sys.argv[5]) if len(sys.argv) > 5 else 200000
    runs = int(sys.argv[6]) if len(sys.argv) > 6 else 10
    os.makedirs(outdir, exist_ok=True)
    paths = sorted(cl.instance_paths(sub), key=lambda p: cl.sort_key(os.path.basename(p)))
    for p in paths:
        t0 = time.time()
        inst = cl.Instance(p)
        g = os.path.join(gdir, inst.name + ".txt")
        res = dict(inst=inst.name, n=inst.n, m=inst.m, k=inst.k)
        cert = None
        for r in (3, 4, 5):
            inside, part = tabu(binary, g, r, iters, runs, seed=7, target=-1)
            assert len(part) == inst.n and set(part) <= set(range(r))
            D = cl.inside_edges(inst, part)
            assert len(D) == inside, "tabu's count disagrees with a recount"
            res[f"best_inside_{r}"] = inside
            if cert is None and inside <= inst.k:
                cert = (r, part, D)
            if r >= 4 and cert is not None:
                break
        r, part, D = cert
        A = inst.adjacency(D)
        clique = cl.max_clique(A)
        w = len(clique)
        assert w <= r and cl.is_clique(A, clique) and not cl.has_clique(A, w + 1)
        path = os.path.join(outdir, inst.name + ".res.json")
        val, obj = cl.write_solution(inst, D, clique, path)
        viol, what = inst.max_violation(val)
        assert viol == 0.0, what
        res.update(r=r, deleted=len(D), residual_omega=w, objective=obj, max_violation=viol,
                   partition="".join(str(x) for x in part), sec=round(time.time() - t0, 1))
        print(json.dumps(res), flush=True)


if __name__ == "__main__":
    main()
