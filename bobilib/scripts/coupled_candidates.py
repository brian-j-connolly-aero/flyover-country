"""List the candidates for `bobi coupled`, smallest first.

usage: python3 -I scripts/coupled_candidates.py COLLECTION_JSON RAY_TSV DATA_DIR [MAX_VARS]

Candidates are BOBILib instances with status `open` and at least one
coupling constraint (collection.json), minus those whose follower is
unbounded for every leader choice (`unbounded` in the output of `bobi ray`,
RAY_TSV) and those with more than MAX_VARS variables (default 100000).
Prints one .mps.gz path per line (starting with DATA_DIR), sorted by
number of variables, and the counts on stderr.
"""
import glob
import json
import os
import sys


def main():
    meta_path, ray_path, data_dir, *rest = sys.argv[1:]
    max_vars = int(rest[0]) if rest else 100000
    meta = json.load(open(meta_path))
    unbounded = set()
    for line in open(ray_path):
        t = line.rstrip("\n").split("\t")
        if len(t) >= 2 and "unbounded" in t[1]:
            unbounded.add(t[0])
    paths = {}
    for p in glob.glob(os.path.join(data_dir, "collection", "general-bilevel", "**", "*.mps.gz"), recursive=True):
        paths[os.path.basename(p).removesuffix(".mps.gz")] = p
    open_coupled = []
    for name, v in meta.items():
        if v["Status"] != "open" or v["Number of coupling constraints"] <= 0:
            continue
        nv = v["Number of upper-level variables"] + v["Number of lower-level variables"]
        open_coupled.append((nv, name))
    open_coupled.sort()
    n_unb = n_big = n_missing = 0
    for nv, name in open_coupled:
        if name in unbounded:
            n_unb += 1
        elif nv > max_vars:
            n_big += 1
        elif name not in paths:
            n_missing += 1
            print(f"no instance file for {name}", file=sys.stderr)
        else:
            print(paths[name])
    kept = len(open_coupled) - n_unb - n_big - n_missing
    print(f"{len(open_coupled)} open with coupling rows: {n_unb} follower unbounded, "
          f"{n_big} over {max_vars} variables, {n_missing} missing, {kept} kept", file=sys.stderr)


main()
