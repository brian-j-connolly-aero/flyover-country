"""Check a leader choice found by k4ls or ftabu, independently of the Rust code.

Rebuilds the deleted edge set D from the tool's output (k4ls: its "transversal" line; ftabu:
the edges whose ends land on equal or non-adjacent pattern vertices of its "best" map), and
reports |D|, whether every element of D is an edge of G, the exact maximum clique of G - D, an
exhaustive K4 test, and whether G - D is 3-colourable (exact backtracking). With --write and
|D| <= k it also writes the solution file and checks it against the instance rows.

usage: python3 -I check_transversal.py INSTANCE_NAME k4ls OUTPUT [--write]
       python3 -I check_transversal.py INSTANCE_NAME ftabu OUTPUT PATTERN [--write]
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def pattern(name):
    """Same patterns as ftabu.rs: vertex count and edge set."""
    cyc = lambda k: [(i, (i + 1) % k) for i in range(k)]  # noqa: E731
    if name == "k3":
        return 3, [(0, 1), (1, 2), (0, 2)]
    if name == "w5":
        return 6, cyc(5) + [(i, 5) for i in range(5)]
    if name == "w7":
        return 8, cyc(7) + [(i, 7) for i in range(7)]
    if name == "c7bar":
        return 7, [(i, (i + d) % 7) for i in range(7) for d in (2, 3)]
    if name == "moser":
        return 7, [(0, 1), (0, 2), (1, 2), (1, 3), (2, 3), (0, 4), (0, 5), (4, 5), (4, 6), (5, 6), (3, 6)]
    raise ValueError(name)


def three_colourable(A):
    """Exact DSatur-style backtracking: always colour the uncoloured vertex with the fewest
    colours left (ties: highest degree); a vertex with none left means backtrack."""
    n = len(A)
    deg = [cl.popcount(a) for a in A]
    cls = [0, 0, 0]  # bitsets of the vertices in each colour class

    def go(left):
        if not left:
            return True
        best = None
        L = left
        while L:
            v = (L & -L).bit_length() - 1
            L &= L - 1
            free = [c for c in range(3) if not A[v] & cls[c]]
            key = (len(free), -deg[v])
            if best is None or key < best[0]:
                best = (key, v, free)
                if not free:
                    return False
        _, v, free = best
        for c in free:
            cls[c] |= 1 << v
            if go(left & ~(1 << v)):
                return True
            cls[c] &= ~(1 << v)
        return False

    return go((1 << n) - 1)


def main():
    args = [a for a in sys.argv[1:] if a != "--write"]
    write = "--write" in sys.argv
    name, tool, path = args[0], args[1], args[2]
    inst = cl.Instance(os.path.join(cl.SET_DIR, "clique", name + ".mps.gz"))
    E = set(inst.edges)
    with open(path) as fh:
        lines = fh.read().splitlines()
    if tool == "k4ls":
        line = [x for x in lines if x.startswith("transversal ")][-1].split()
        D = [tuple(sorted(map(int, t.split("-")))) for t in line[2:]]
        claimed = int(line[1])
    else:
        r, fe = pattern(args[3])
        adj = {(i, j) for i, j in fe} | {(j, i) for i, j in fe}
        line = [x for x in lines if x.startswith("best ")][-1].split()
        claimed, f = int(line[1]), list(map(int, line[2:]))
        assert len(f) == inst.n and all(0 <= c < r for c in f)
        D = [(u, v) for (u, v) in inst.edges if (f[u], f[v]) not in adj]
    out = dict(inst=name, tool=tool, k=inst.k, claimed=claimed, deleted=len(D), distinct=len(set(D)),
               all_edges_of_G=all(e in E for e in D))
    H = inst.adjacency(D)
    omega = cl.max_clique(H)
    out["remainder_omega"] = len(omega)
    out["remainder_has_k4"] = cl.has_clique(H, 4)
    out["remainder_3_colourable"] = three_colourable(H)
    if write and len(set(D)) <= inst.k and out["all_edges_of_G"] and not out["remainder_has_k4"]:
        val, obj = cl.write_solution(inst, sorted(set(D)), omega, os.path.join(cl.RESULTS, name + ".res.json"))
        out["written_objective"] = obj
        out["max_violation"] = inst.max_violation(val)[0]
    print(json.dumps(out), flush=True)


if __name__ == "__main__":
    main()
