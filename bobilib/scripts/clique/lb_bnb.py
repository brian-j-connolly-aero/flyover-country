"""Third check of w3(G) > k on the instances where Brouwer's theorem applies, with no MIP/CP
solver: the exact search split_bnb.rs on the complement graph, for every size vector that
lb.py enumerates (same reduction, independent search code).

usage: python3 -I lb_bnb.py SUBSET SPLIT_BNB_BIN WORKDIR > lb_bnb.jsonl
"""
import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402
from lb import c2, size_vectors  # noqa: E402


def main():
    sub, binary, work = sys.argv[1:4]
    os.makedirs(work, exist_ok=True)
    paths = sorted(cl.instance_paths(sub), key=lambda p: cl.sort_key(os.path.basename(p)))
    for p in paths:
        inst = cl.load(p)
        n, k, nb = inst.n, inst.k, len(inst.nonedges)
        if not (n >= 7 and inst.m - k >= cl.turan(n, 3) - n // 3 + 2):
            continue
        g = os.path.join(work, inst.name + ".complement.txt")
        with open(g, "w") as fh:
            fh.write(f"{n} {nb}\n")
            fh.writelines(f"{u} {v}\n" for u, v in inst.nonedges)
        rows, ok = [], True
        for sizes, b in size_vectors(n, lambda s: k + nb - sum(c2(t) for t in s)):
            t0 = time.time()
            out = subprocess.run([binary, g, *map(str, sizes), str(b)], capture_output=True, text=True,
                                 check=True).stdout.split("\n")
            found = out[0].startswith("FOUND")
            if found:
                part = [int(t) for t in out[0].split()[1:]]
                rows.append(dict(sizes=sizes, budget=b, found=True, inside_G=len(cl.inside_edges(inst, part))))
            else:
                assert out[0] == "NONE"
                rows.append(dict(sizes=sizes, budget=b, found=False, nodes=int(out[1].split()[1]),
                                 sec=round(time.time() - t0, 2)))
            ok &= not found
        print(json.dumps(dict(inst=inst.name, w3_gt_k_bnb=ok, size_vectors=rows)), flush=True)


if __name__ == "__main__":
    main()
