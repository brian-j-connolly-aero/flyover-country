"""Tests for the clique partition certificates (prover cert_prove.rs/.py, checker
scripts/check_clique_certificates.py). Everything is compared with brute force over all 3^n
labelled partitions of small random graphs, which shares no code with either program.

(A) The part-size reduction of the appendix (Proposition "Part-size reduction"): on random graphs
    G with complement H, for every labelled partition, inside_G = sum C(s_i,2) - |E(H)| + cut_H;
    relabelling parts keeps both counts; and for every k, w3(G) > k holds iff every sorted size
    vector s with b(s) = k + |E(H)| - sum C(s_i,2) >= 0 has min cut_H > b(s).
(B) The symmetry rule (equal-size parts opened in index order) loses no minimum, and the leaf bound
    never exceeds the smallest cut of a completion of a random partial placement.
(C) End to end through instance files written in BOBILib's clique format (random graphs, k near
    w3): the prover writes a certificate exactly when brute force says w3(G) > k, and the checker
    passes exactly then.
(D) Forged certificates for false claims (trees proved for a smaller budget, or a bare root leaf)
    are all rejected.
(E) Mutated instance files and mutated certificates of a real instance are rejected (or, for a
    mutated certificate, still a valid proof).
(F) The case list of the 30 certificates equals the table in the appendix.

usage: python3 -I cert_selftest.py CERT_PROVE_BIN WORKDIR [TRIALS]
"""
import gzip
import itertools
import json
import os
import random
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cert_prove  # noqa: E402
import cliquelib as cl  # noqa: E402

BOBILIB = os.path.dirname(os.path.dirname(HERE))
CHECKER = os.path.join(BOBILIB, "scripts", "check_clique_certificates.py")
CERTS = os.path.join(BOBILIB, "results", "clique", "certificates")
INST = os.path.join(BOBILIB, "data", "collection", "interdiction", "clique-interdiction", "clique")
APPENDIX = {40: [(14, 13, 13, 6), (14, 14, 12, 5), (15, 13, 12, 4), (15, 14, 11, 2), (16, 12, 12, 1), (16, 13, 11, 0)],
            50: [(17, 17, 16, 6), (18, 16, 16, 5), (18, 17, 15, 4), (19, 16, 15, 2), (18, 18, 14, 1), (19, 17, 14, 0)],
            60: [(20, 20, 20, 5), (21, 20, 19, 4), (21, 21, 18, 2), (22, 19, 19, 2), (22, 20, 18, 1)]}


def c2(t):
    return t * (t - 1) // 2


def random_graph(rng, n, p):
    pairs = list(itertools.combinations(range(n), 2))
    eg = [e for e in pairs if rng.random() < p]
    seen = set(eg)
    return eg, [e for e in pairs if e not in seen]


def brute(n, eg, eh):
    """All labelled partitions: returns (w3, {labelled sizes: min cut_H}) and checks the identity."""
    w3, mincut = None, {}
    for f in itertools.product(range(3), repeat=n):
        s = (f.count(0), f.count(1), f.count(2))
        ins = sum(f[u] == f[v] for u, v in eg)
        cut = sum(f[u] != f[v] for u, v in eh)
        assert ins == sum(map(c2, s)) - len(eh) + cut, "counting identity"
        w3 = ins if w3 is None else min(w3, ins)
        mincut[s] = min(mincut.get(s, cut), cut)
    return w3, mincut


def sorted_sizes(n):
    return [(a, b, n - a - b) for a in range(n + 1) for b in range(min(a, n - a) + 1) if n - a - b <= b]


def test_reduction(rng, trials):
    graphs = decisions = 0
    for _ in range(trials):
        n = rng.randint(3, 9)
        eg, eh = random_graph(rng, n, rng.choice([0.3, 0.5, 0.7, 0.85, 0.95]))
        w3, mincut = brute(n, eg, eh)
        for s, c in mincut.items():  # relabelling parts keeps the minimum
            assert mincut[tuple(sorted(s, reverse=True))] == c
        for k in range(-1, c2(n) + 1):
            red = all(mincut[s] > k + len(eh) - sum(map(c2, s)) for s in sorted_sizes(n)
                      if k + len(eh) - sum(map(c2, s)) >= 0)
            assert (w3 > k) == red, (n, eg, k)
            decisions += 1
        graphs += 1
    return f"(A) reduction: {graphs} random graphs (n = 3..9), {decisions} values of k, 0 mismatches"


def canonical(f, cap):
    first = {}
    for p in f:
        first.setdefault(p, len(first))
    return all(not (cap[p - 1] == cap[p] and first.get(p - 1, 99) > first.get(p, 99)) for p in (1, 2))


def leaf_bound(n, eh, cap, place):
    """The checker's leaf bound, written again: place[v] in {0,1,2} or None, in vertex order."""
    adj = [[] for _ in range(n)]
    for u, v in eh:
        adj[u].append(v)
        adj[v].append(u)
    size = [sum(1 for p in place if p == q) for q in range(3)]
    cut = sum(1 for u, v in eh if place[u] is not None and place[v] is not None and place[u] != place[v])
    lb = cut
    for u in range(n):
        if place[u] is None:
            lb += min(sum(1 for v in adj[u] if place[v] is not None and place[v] != p)
                      for p in range(3) if size[p] < cap[p])
    return lb


def test_symmetry_and_bound(rng, trials):
    checks = 0
    for _ in range(trials):
        n = rng.randint(3, 8)
        eg, eh = random_graph(rng, n, rng.choice([0.5, 0.8, 0.9]))
        for cap in sorted_sizes(n):
            allf = [f for f in itertools.product(range(3), repeat=n) if tuple(f.count(q) for q in range(3)) == cap]
            cuts = {f: sum(f[u] != f[v] for u, v in eh) for f in allf}
            assert min(cuts[f] for f in allf if canonical(f, cap)) == min(cuts.values())
            for _ in range(4):
                d = rng.randint(0, n)
                f = rng.choice(allf)
                place = list(f[:d]) + [None] * (n - d)
                best = min(cuts[g] for g in allf if list(g[:d]) == list(f[:d]))
                assert leaf_bound(n, eh, cap, place) <= best
                checks += 1
    return f"(B) symmetry rule and leaf bound: {trials} random graphs (n = 3..8), {checks} partial placements, 0 failures"


def write_instance(path, name, n, eg, k):
    """An instance in BOBILib's clique format (names as in BOBILib's files)."""
    nonedges = [e for e in itertools.combinations(range(n), 2) if e not in set(eg)]
    rows = ["KL"] + [f"C{u},{v}" for u, v in nonedges] + [f"E{u},{v}" for u, v in eg] + [f"I{u},{v}" for u, v in eg]
    col = []
    for u, v in eg:
        col += [(f"w{u},{v}", "KL", 1), (f"w{u},{v}", f"I{u},{v}", 1)]
    for u, v in eg:
        col += [(f"y{u},{v}", f"E{u},{v}", -1), (f"y{u},{v}", f"I{u},{v}", 1)]
    for i in range(n):
        col.append((f"x{i}", "cost", 1))
        col += [(f"x{i}", f"C{u},{v}", 1) for u, v in nonedges if i in (u, v)]
        col += [(f"x{i}", f"E{u},{v}", 1) for u, v in eg if i in (u, v)]
    out = [f"NAME          {name}", "ROWS", " N  cost"] + [f" L  {r}" for r in rows] + ["COLUMNS",
           "    MARK0000  'MARKER'                 'INTORG'"]
    out += [f"    {c:<9} {r:<30} {a}" for c, r, a in col] + ["    MARK0001  'MARKER'                 'INTEND'", "RHS"]
    out += [f"    rhs       KL  {k}"] + [f"    rhs       {r}  1" for r in rows[1:]] + ["BOUNDS"]
    cols = list(dict.fromkeys(c for c, _, _ in col))
    out += [f" BV bnd       {c}" for c in cols] + ["ENDATA"]
    with gzip.open(os.path.join(path, name + ".mps.gz"), "wt") as fh:
        fh.write("\n".join(out) + "\n")
    fvars = [f"y{u},{v} 0.0" for u, v in eg] + [f"x{i} -1." for i in range(n)]
    aux = ["@NUMVARS", str(len(fvars)), "@NUMCONSTRS", str(len(rows) - 1), "@VARSBEGIN"] + fvars + ["@VARSEND",
           "@CONSTRSBEGIN"] + rows[1:] + ["@CONSTRSEND", "@NAME", name, "@MPS", name + ".mps"]
    with open(os.path.join(path, name + ".aux"), "w") as fh:
        fh.write("\n".join(aux) + "\n")


def check(idir, cdir, names, brouwer=True):
    flag = [] if brouwer else ["--no-brouwer"]
    r = subprocess.run([sys.executable, "-I", CHECKER, *flag, idir, cdir, *names], capture_output=True, text=True,
                       timeout=600)
    return r.returncode == 0 and r.stdout.rstrip().endswith("RESULT PASS"), r.stdout


def prove_raw(binary, hfile, s, b):
    out = subprocess.run([binary, hfile, *map(str, s), str(b), "1", "1"], capture_output=True, text=True,
                         check=True, timeout=600).stdout.split("\n")
    return None if out[0].startswith("FOUND") else (out[0].split()[1:], out[2].split()[1])


def brute_masks(n, eg, eh):
    """w3(G) and min cut_H for each labelled size vector, over all 3^n labelled partitions (bitmasks)."""
    pc = [bin(x).count("1") for x in range(1 << n)]
    e = {}
    for name, edges in (("G", eg), ("H", eh)):
        adj = [0] * n
        for u, v in edges:
            adj[u] |= 1 << v
            adj[v] |= 1 << u
        cnt = [0] * (1 << n)
        for mask in range(1, 1 << n):
            low = mask & -mask
            rest = mask ^ low
            cnt[mask] = cnt[rest] + pc[adj[low.bit_length() - 1] & rest]
        e[name] = cnt
    eG, eH, h, full = e["G"], e["H"], len(eh), (1 << n) - 1
    w3, mincut = None, {}
    for a in range(1 << n):
        rest = full ^ a
        b = rest
        while True:
            c = rest ^ b
            ins = eG[a] + eG[b] + eG[c]
            assert ins == c2(pc[a]) + c2(pc[b]) + c2(pc[c]) - h + (h - eH[a] - eH[b] - eH[c])
            s = (pc[a], pc[b], pc[c])
            cut = h - eH[a] - eH[b] - eH[c]
            w3 = ins if w3 is None else min(w3, ins)
            if cut < mincut.get(s, 1 << 30):
                mincut[s] = cut
            if b == 0:
                break
            b = (b - 1) & rest
    return w3, mincut


def test_end_to_end(rng, binary, work, trials):
    idir, cdir = os.path.join(work, "inst"), os.path.join(work, "cert")
    for d in (idir, cdir):
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
    st = dict(true=0, false=0, forged=0, cases=0, brouwer=0, nodes=0)
    for t in range(trials):
        n = rng.randint(7, 12)
        eg, eh = random_graph(rng, n, rng.uniform(0.6, 0.97))
        w3, mincut = brute_masks(n, eg, eh)
        k = max(0, w3 + rng.choice([-3, -2, -1, -1, -1, 0, 0, 0, 1]))
        name = f"selftest{t}"
        write_instance(idir, name, n, eg, k)
        inst = cl.load(os.path.join(idir, name + ".mps.gz"))
        assert sorted(inst.edges) == sorted(eg) and inst.k == k
        made = cert_prove.prove(inst, binary, work, cdir, tries=3)
        assert made == (w3 > k), (name, w3, k)
        ok, log = check(idir, cdir, [name], brouwer=False)
        assert ok == (w3 > k), (name, w3, k, log)
        q, r = divmod(n, 3)
        if len(eg) - k >= c2(n) - r * c2(q + 1) - (3 - r) * c2(q) - n // 3 + 2:  # Brouwer's hypothesis holds
            assert check(idir, cdir, [name])[0] == ok
            st["brouwer"] += 1
        cs = cert_prove.cases(n, k, len(eh))
        st["true" if w3 > k else "false"] += 1
        st["cases"] += len(cs)
        if w3 > k:
            with gzip.open(os.path.join(cdir, name + ".cert.jsonl.gz"), "rt") as fh:
                st["nodes"] += sum(json.loads(x)["nodes"] for x in list(fh)[1:])
            continue
        # (D) forge: each case whose claim is false gets a tree proved for a smaller budget.
        hfile = os.path.join(work, name + ".H.txt")
        lines = []
        for s, b in cs:
            c = mincut[s]
            order, tree = prove_raw(binary, hfile, s, min(b, c - 1)) if c > 0 else (list(range(n)), "0")
            lines.append(dict(sizes=list(s), budget=b, order=[f"x{v}" for v in order], nodes=len(tree),
                              leaves=tree.count("0"), tree=tree))
        head = dict(format="bobilib-clique-w3-tree", version=1, instance=name, n=n, m=len(eg), k=k,
                    nonedges=len(eh), cases=[c["sizes"] + [c["budget"]] for c in lines])
        with gzip.open(os.path.join(cdir, name + ".cert.jsonl.gz"), "wt") as fh:
            fh.write("\n".join(json.dumps(x) for x in [head] + lines) + "\n")
        assert not check(idir, cdir, [name], brouwer=False)[0], ("forgery accepted", name)
        st["forged"] += 1
    return (f"(C) end to end: {trials} random instance files (n = 7..12, k near w3), {st['cases']} cases; "
            f"w3 > k in {st['true']} (certificate written, {st['nodes']} tree nodes, checker PASS), w3 <= k in "
            f"{st['false']} (no certificate, checker FAIL), as brute force says; {st['brouwer']} of them also "
            f"satisfy Brouwer's hypothesis and give the same verdict without --no-brouwer\n"
            f"(D) forgeries: {st['forged']} forged certificates for false claims, all rejected")


def test_mutations(rng, work):
    name = "normalClique40-90-1"
    idir, cdir = os.path.join(work, "mut_inst"), os.path.join(work, "mut_cert")
    for d in (idir, cdir):
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
    with gzip.open(os.path.join(INST, name + ".mps.gz"), "rt", encoding="latin-1") as fh:
        mps = fh.read()
    with open(os.path.join(INST, name + ".aux"), encoding="latin-1") as fh:
        aux = fh.read()
    shutil.copy(os.path.join(CERTS, name + ".cert.jsonl.gz"), cdir)
    row = mps.split("\n")[5].split()[1]  # the first C row
    variants = {
        "budget k+1": (mps.replace("rhs       KL                            175", "rhs       KL  176"), aux),
        "budget k-1": (mps.replace("rhs       KL                            175", "rhs       KL  174"), aux),
        f"row {row} deleted": ("\n".join(x for x in mps.split("\n") if row not in x.split()),
                               "\n".join(x for x in aux.split("\n") if x.strip() != row).replace(
                                   "@NUMCONSTRS\n1482", "@NUMCONSTRS\n1481")),
        "a y coefficient -1 -> -2": (mps.replace("E0,1                           -1", "E0,1  -2", 1), aux),
        "RANGES section added": (mps.replace("BOUNDS", "RANGES\n    rng       KL  1\nBOUNDS"), aux),
        "a follower cost -1 -> -2": (mps, aux.replace("\nx0  -1.", "\nx0  -2.")),
        "a bound BV -> UP": (mps.replace(" BV bnd       x0 ", " UP bnd       x0  1"), aux),
    }
    out = []
    ok, _ = check(INST, cdir, [name])
    assert ok
    for what, (m, a) in variants.items():
        assert (m, a) != (mps, aux), what
        with gzip.open(os.path.join(idir, name + ".mps.gz"), "wt") as fh:
            fh.write(m)
        with open(os.path.join(idir, name + ".aux"), "w") as fh:
            fh.write(a)
        ok, log = check(idir, cdir, [name])
        assert not ok, what
        out.append(what)
    with gzip.open(os.path.join(CERTS, name + ".cert.jsonl.gz"), "rt") as fh:
        head, *certs = [json.loads(x) for x in fh]
    kinds = ["one tree character flipped", "last tree character dropped", "a leaf appended",
             "two of the first 8 order entries swapped", "a budget raised by 1", "a case deleted"]
    tally = {kind: [0, 0] for kind in kinds}
    for t in range(60):
        cs = json.loads(json.dumps(certs))
        c = rng.choice(cs)
        kind = t % 6
        if kind == 0:
            i = rng.randrange(len(c["tree"]))
            c["tree"] = c["tree"][:i] + "10"[int(c["tree"][i])] + c["tree"][i + 1:]
        elif kind == 1:
            c["tree"] = c["tree"][:-1]
        elif kind == 2:
            c["tree"] += "0"
        elif kind == 3:
            i, j = rng.sample(range(8), 2)  # among the first vertices, where the tree branches
            c["order"][i], c["order"][j] = c["order"][j], c["order"][i]
        elif kind == 4:
            c["budget"] += 1
        else:
            cs.remove(c)
        with gzip.open(os.path.join(cdir, name + ".cert.jsonl.gz"), "wt") as fh:
            fh.write("\n".join(json.dumps(x) for x in [head] + cs) + "\n")
        ok, _ = check(INST, cdir, [name])
        tally[kinds[kind]][ok] += 1
    return (f"(E) instance mutations rejected: {len(out)}/{len(variants)} ({'; '.join(out)})\n"
            f"    certificate mutations (rejected/still a valid proof): "
            + "; ".join(f"{kind} {r}/{a}" for kind, (r, a) in tally.items()))


def test_appendix():
    seen = {}
    for f in sorted(os.listdir(CERTS)):
        if f.endswith(".cert.jsonl.gz"):
            with gzip.open(os.path.join(CERTS, f), "rt") as fh:
                head = json.loads(fh.readline())
            seen.setdefault(head["n"], set()).add(tuple(sorted(map(tuple, head["cases"]))))
    for n, cases in APPENDIX.items():
        assert seen[n] == {tuple(sorted(cases))}, n
    return f"(F) case lists of {sum(len(v) for v in seen.values())} distinct groups equal the appendix table"


def main():
    binary, work = sys.argv[1:3]
    trials = int(sys.argv[3]) if len(sys.argv) > 3 else 300
    os.makedirs(work, exist_ok=True)
    rng = random.Random(20261019)
    for line in (test_reduction(rng, trials), test_symmetry_and_bound(rng, trials // 3),
                 test_end_to_end(rng, binary, work, trials // 3), test_mutations(rng, work), test_appendix()):
        print(line, flush=True)
    print("SELFTEST PASS")


if __name__ == "__main__":
    main()
