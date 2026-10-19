"""Write the branch-and-bound certificates for w3(G) > k (density-0.9 clique instances).

For every instance where Brouwer's theorem applies (n >= 7 and m - k >= t_3(n) - floor(n/3) + 2)
and every size vector s1 >= s2 >= s3 >= 0 with b(s) = k + |E(H)| - sum C(s_i, 2) >= 0, where H
is the complement of G, run cert_prove (cert_prove.rs) on H and store its tree. The format is
described in results/clique/certificates/README.md; the checker is
scripts/check_clique_certificates.py, which shares no code with this file or with cliquelib.

usage: python3 -I cert_prove.py CERT_PROVE_BIN WORKDIR OUTDIR [TRIES] [NAME ...]
"""
import gzip
import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def c2(t):
    return t * (t - 1) // 2


def cases(n, k, nb):
    out = []
    for s1 in range(n, -1, -1):
        for s2 in range(min(s1, n - s1), -1, -1):
            s3 = n - s1 - s2
            if s3 > s2:
                continue
            b = k + nb - c2(s1) - c2(s2) - c2(s3)
            if b >= 0:
                out.append(((s1, s2, s3), b))
    return out


def prove(inst, binary, work, outdir, tries=1, seed=1, log=None):
    """Write OUTDIR/NAME.cert.jsonl.gz and return True, or return False if some case has a split
    (then w3(G) <= k and no certificate exists). Brouwer's hypothesis is not tested here."""
    n, k, nb = inst.n, inst.k, len(inst.nonedges)
    g = os.path.join(work, inst.name + ".H.txt")
    with open(g, "w") as fh:
        fh.write(f"{n} {nb}\n")
        fh.writelines(f"{u} {v}\n" for u, v in inst.nonedges)
    cs = cases(n, k, nb)
    head = dict(format="bobilib-clique-w3-tree", version=1, instance=inst.name, n=n, m=inst.m, k=k,
                nonedges=nb, cases=[[*s, b] for s, b in cs])
    lines = [json.dumps(head, separators=(",", ":"))]
    for s, b in cs:
        t0 = time.time()
        out = subprocess.run([binary, g, *map(str, s), str(b), str(tries), str(seed)], capture_output=True,
                             text=True, check=True, timeout=3600).stdout.split("\n")
        if out[0].startswith("FOUND"):
            return False
        order = [inst.xs[int(v)] for v in out[0].split()[1:]]
        nodes, leaves = map(int, out[1].split()[1:])
        tree = out[2].split()[1]
        assert len(tree) == nodes
        lines.append(json.dumps(dict(sizes=list(s), budget=b, order=order, nodes=nodes, leaves=leaves,
                                     tree=tree), separators=(",", ":")))
        if log:
            print(json.dumps(dict(inst=inst.name, sizes=s, budget=b, nodes=nodes, leaves=leaves,
                                  sec=round(time.time() - t0, 2))), file=log, flush=True)
    with gzip.GzipFile(os.path.join(outdir, inst.name + ".cert.jsonl.gz"), "wb", mtime=0) as fh:
        fh.write(("\n".join(lines) + "\n").encode())
    return True


def main():
    binary, work, outdir = sys.argv[1:4]
    tries = int(sys.argv[4]) if len(sys.argv) > 4 else 1
    names = set(sys.argv[5:])
    os.makedirs(work, exist_ok=True)
    os.makedirs(outdir, exist_ok=True)
    paths = sorted(cl.instance_paths("clique"), key=lambda p: cl.sort_key(os.path.basename(p)))
    for p in paths:
        inst = cl.load(p)
        if names and inst.name not in names:
            continue
        if not (inst.n >= 7 and inst.m - inst.k >= cl.turan(inst.n, 3) - inst.n // 3 + 2):
            continue
        if not prove(inst, binary, work, outdir, tries, log=sys.stdout):
            raise SystemExit(f"{inst.name}: a split exists, no certificate")


if __name__ == "__main__":
    main()
