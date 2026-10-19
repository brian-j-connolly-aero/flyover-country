"""Check the branch-and-bound certificates for w3(G) > k on BOBILib's density-0.9 clique instances.

usage: python3 -I check_clique_certificates.py [--no-brouwer] INSTANCE_DIR CERT_DIR [NAME ...]
(default NAMEs normalClique{40,50,60}-90-{1..10}; reads NAME.mps[.gz], NAME.aux, NAME.cert.jsonl.gz;
--no-brouwer, for tests only, skips the assertion of item 2)

A PASS proves, for each instance, in exact integer arithmetic and from the files alone:
1. Model. Read as BOBILib intends (the leader minimises the MPS objective; the follower owns the
   AUX columns and minimises the AUX objective subject to the AUX rows), the file is edge clique
   interdiction on a graph G = (V, E): all columns are binary (BV); V = the follower columns x
   with follower cost -1 and leader cost +1, all other costs are 0; no RANGES or other sections;
   the rows are exactly one leader row, sum of all leader columns w <= k, and follower rows: for
   each vertex pair uv exactly one of x_u + x_v <= 1 (uv not in E) or x_u + x_v - y <= 1 (uv in E,
   with its own column y); for each such y exactly one row w + y <= 1, with its own w. So
   n = |V|, m = |E| and k come from the rows; names serve only as labels.
2. Arithmetic: n >= 7 and m - k >= t_3(n) - floor(n/3) + 2, the hypothesis of Brouwer's 1981
   Theorem 2 in complemented form (the theorem itself is NOT checked here).
3. w3(G) > k: every partition of V into 3 parts (empty parts allowed) has more than k edges of G
   inside parts. Let H be the complement of G (the x_u + x_v <= 1 rows), h = |E(H)|. A partition
   with part sizes s has inside_G = sum C(s_i,2) - h + cut_H (a pair inside a part is an edge of G
   or of H, and h - cut_H edges of H lie inside parts). Relabel parts so that s1 >= s2 >= s3; with
   b(s) = k + h - sum C(s_i,2), inside_G <= k iff cut_H <= b(s). The script enumerates every such
   s. If b(s) < 0 there is nothing to show (cut_H >= 0); otherwise the certificate must hold a
   tree for exactly (s, b(s)), and the script checks it:
   * the tree fixes an order v_0..v_{n-1} of V; a node at depth d places v_0..v_{d-1};
   * a branch node ('1', needs d < n) gets one child per part p = 0, 1, 2 allowed for v_d: part p
     holds fewer than s_p vertices and, if it is empty and s_{p-1} = s_p, part p-1 is not empty;
   * a leaf ('0') needs  cut_H(placed) + sum over unplaced u of min over parts p with room of
     |placed H-neighbours of u outside p|  >  b(s), a lower bound on cut_H of every completion
     (each unplaced u ends in a part with room now; the edges counted for u join u to placed
     vertices, so they differ for different u and from the edges among placed vertices).
   Symmetry: relabelling equal-size parts so that their first vertices in the order increase keeps
   sizes and cut, and such a canonical partition never uses a forbidden empty part. So every
   canonical partition with sizes s reaches a leaf and has cut_H > b(s), hence every partition.
Trusted: Python's integers, fractions (to read numbers), json, gzip, and this file. No project
code is imported. Run without -O (the checks are assert statements).
"""
import gzip
import json
import os
import sys
import time
from fractions import Fraction


def num(t):
    q = Fraction(t)
    assert q.denominator == 1, f"non-integer number {t}"
    return q.numerator


def c2(t):
    return t * (t - 1) // 2


def read_mps(path):
    rows, cols, rhs, bv, obj, sec = {}, {}, {}, set(), None, None
    with (gzip.open if path.endswith(".gz") else open)(path, "rt", encoding="latin-1") as fh:
        for line in fh:
            t = line.split()
            if not t or line[0] == "*":
                continue
            if not line[0].isspace():
                sec = t[0]
                assert sec in ("NAME", "ROWS", "COLUMNS", "RHS", "BOUNDS", "ENDATA"), f"section {sec}"
            elif sec == "ROWS":
                assert (t[0] == "N" and obj is None) or (t[0] == "L" and t[1] not in rows), f"row {t}"
                if t[0] == "N":
                    obj = t[1]
                else:
                    rows[t[1]] = {}
            elif sec == "COLUMNS" and t[1] != "'MARKER'":
                c = cols.setdefault(t[0], {})
                for r, a in zip(t[1::2], t[2::2]):
                    assert (r == obj or r in rows) and r not in c, f"entry {t[0]} {r}"
                    c[r] = num(a)
                    if r != obj and c[r]:
                        rows[r][t[0]] = c[r]
            elif sec == "RHS":
                for r, a in zip(t[1::2], t[2::2]):
                    assert r in rows and r not in rhs, f"rhs {r}"
                    rhs[r] = num(a)
            elif sec == "BOUNDS":
                assert t[0] == "BV" and t[2] in cols and t[2] not in bv, f"bound {t}"
                bv.add(t[2])
    assert bv == set(cols), "every column must be binary (BV)"
    return rows, {c: e.get(obj, 0) for c, e in cols.items()}, rhs


def read_aux(path):
    lvars, lrows, blk, head = {}, set(), None, {}
    with open(path, encoding="latin-1") as fh:
        for t in filter(None, map(str.split, fh)):
            if t[0][0] == "@":
                blk = t[0]
            elif blk == "@VARSBEGIN":
                assert t[0] not in lvars
                lvars[t[0]] = num(t[1])
            elif blk == "@CONSTRSBEGIN":
                assert t[0] not in lrows
                lrows.add(t[0])
            else:
                assert blk in ("@NUMVARS", "@NUMCONSTRS", "@NAME", "@MPS") and blk not in head, f"aux {blk}"
                head[blk] = t[0]
    assert num(head["@NUMVARS"]) == len(lvars) and num(head["@NUMCONSTRS"]) == len(lrows)
    return lvars, lrows


def read_graph(mps, aux):
    """Check the model of item 1; return (sorted V, H as {v: set of H-neighbours}, m, k)."""
    (rows, cost, rhs), (lvars, lrows) = read_mps(mps), read_aux(aux)
    assert set(lvars) <= set(cost) and lrows <= set(rows)
    X = sorted(c for c in lvars if lvars[c] == -1)
    xs, Y, W = set(X), {c for c in lvars if lvars[c] == 0}, set(cost) - set(lvars)
    assert len(X) + len(Y) == len(lvars), "follower costs must be -1 or 0"
    assert all(cost[c] == (1 if c in xs else 0) for c in cost), "leader cost: +1 on x, 0 elsewhere"
    lead = [r for r in rows if r not in lrows]
    assert len(lead) == 1 and rows[lead[0]] == {w: 1 for w in W}, "leader row must be sum w <= k"
    H, pairs, yE, yI, wI = {x: set() for x in X}, set(), set(), set(), set()
    for r in lrows:
        c = rows[r]
        px = tuple(sorted(v for v in c if v in xs and c[v] == 1))
        py = [v for v in c if v in Y]
        assert rhs.get(r, 0) == 1, f"rhs of {r}"
        if len(c) == 2 and len(py) == 1 and set(c) - Y <= W and set(c.values()) == {1}:  # w + y <= 1
            w = (set(c) - Y).pop()
            assert py[0] not in yI and w not in wI, f"row {r} repeats a column"
            yI.add(py[0])
            wI.add(w)
            continue
        assert len(px) == 2 and px not in pairs, f"row {r}: unexpected shape or repeated vertex pair"
        pairs.add(px)
        if len(c) == 2:  # x_u + x_v <= 1: uv is an edge of H
            H[px[0]].add(px[1])
            H[px[1]].add(px[0])
        else:  # x_u + x_v - y <= 1: uv is an edge of G
            assert len(c) == 3 and len(py) == 1 and c[py[0]] == -1 and py[0] not in yE, f"row {r}"
            yE.add(py[0])
    n = len(X)
    assert len(pairs) == n * (n - 1) // 2, "every vertex pair needs exactly one row"
    assert yE == yI == Y and wI == W, "each y needs one edge row and one w + y row; each w one"
    return X, H, len(yE), rhs.get(lead[0], 0)


def check_tree(X, H, sizes, b, cert):
    """Verify one tree (item 3); return (nodes, leaves)."""
    n, order, tree = len(X), cert["order"], cert["tree"]
    assert sorted(order) == X, "order is not a permutation of V"
    pos = {v: i for i, v in enumerate(order)}
    adj = [[pos[u] for u in H[v]] for v in order]  # vertices renamed to their place in the order
    cnt, placed, size, cap = [[0, 0, 0] for _ in range(n)], [0] * n, [0, 0, 0], list(sizes)
    it, leaves = iter(tree), [0]

    def put(v, p, s):
        size[p] += s
        for u in adj[v]:
            cnt[u][p] += s
            placed[u] += s

    def node(d, cut):
        c = next(it, "end of tree string")
        if c == "0":
            room = [p for p in range(3) if size[p] < cap[p]]
            lb = cut + sum(min(placed[u] - cnt[u][p] for p in room) for u in range(d, n))
            assert lb > b, f"leaf at depth {d} has bound {lb} <= {b}"
            leaves[0] += 1
            return
        assert c == "1" and d < n, f"bad node {c!r} at depth {d}"
        for p in range(3):
            if size[p] < cap[p] and not (p > 0 and size[p] == 0 and cap[p - 1] == cap[p] and size[p - 1] == 0):
                add = placed[d] - cnt[d][p]
                put(d, p, 1)
                node(d + 1, cut + add)
                put(d, p, -1)

    node(0, 0)
    assert next(it, None) is None, "tree string has nodes left over"
    assert (cert["nodes"], cert["leaves"]) == (len(tree), leaves[0]), "node or leaf count"
    return len(tree), leaves[0]


def check_instance(idir, cdir, name, brouwer):
    mps = os.path.join(idir, name + ".mps.gz")
    X, H, m, k = read_graph(mps if os.path.exists(mps) else mps[:-3], os.path.join(idir, name + ".aux"))
    n, h = len(X), sum(map(len, H.values())) // 2
    q, r = divmod(n, 3)
    t3 = c2(n) - r * c2(q + 1) - (3 - r) * c2(q)
    margin = m - k - (t3 - n // 3 + 2)
    assert (n >= 7 and margin >= 0) or not brouwer, "Brouwer's hypothesis fails"
    sorted_sizes = [(s1, s2, n - s1 - s2) for s1 in range(n + 1) for s2 in range(min(s1, n - s1) + 1)
                    if n - s1 - s2 <= s2]
    want = {s: k + h - sum(map(c2, s)) for s in sorted_sizes if k + h - sum(map(c2, s)) >= 0}
    with gzip.open(os.path.join(cdir, name + ".cert.jsonl.gz"), "rt") as fh:
        head, certs = json.loads(fh.readline()), [json.loads(x) for x in fh]
    assert [head[f] for f in ("instance", "n", "m", "k", "nonedges")] == [name, n, m, k, h], "header"
    got = {tuple(c["sizes"]): c for c in certs}
    assert len(got) == len(certs) and set(got) == set(want), "case list differs from the reduction"
    print(f"{name}: n={n} m={m} k={k} h={h} t3={t3} Brouwer margin {margin}; {len(sorted_sizes)} "
          f"sorted size vectors, {len(want)} with b(s) >= 0", flush=True)
    res = []
    for s in sorted(want, key=lambda s: (want[s], s)):
        assert got[s]["budget"] == want[s], f"budget of {s}"
        res.append(check_tree(X, H, s, want[s], got[s]))
        print(f"  sizes {s} b={want[s]}: tree OK, {res[-1][0]} nodes, {res[-1][1]} leaves")
    return k, res


def main():
    if not __debug__:
        sys.exit("the checks are assert statements: run without -O")
    args = sys.argv[1:]
    brouwer = args[0] != "--no-brouwer"
    idir, cdir, *names = args if brouwer else args[1:]
    names = names or [f"normalClique{n}-90-{i}" for n in (40, 50, 60) for i in range(1, 11)]
    t0, res, bad = time.time(), [], []
    for name in names:
        try:
            ti = time.time()
            k, r = check_instance(idir, cdir, name, brouwer)
            res += r
            print(f"  PASS: w3(G) > k = {k} ({time.time() - ti:.2f} s)")
        except Exception as e:  # any error at all fails the instance
            bad.append(name)
            print(f"{name}: FAIL {type(e).__name__}: {e}")
    print(f"instances passed {len(names) - len(bad)}/{len(names)}; cases certified {len(res)}; tree nodes "
          f"{sum(r[0] for r in res)}, leaves {sum(r[1] for r in res)}; time {time.time() - t0:.1f} s")
    print("RESULT", "PASS" if not bad else "FAIL " + " ".join(bad))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
