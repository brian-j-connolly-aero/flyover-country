"""Write each instance's graph G as a plain edge list ("n m", then "u v" lines) for rcut_tabu.

usage: python3 -I export_graphs.py SUBSET OUTDIR
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cliquelib as cl  # noqa: E402


def main():
    sub, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    for p in cl.instance_paths(sub):
        inst = cl.Instance(p)
        with open(os.path.join(out, inst.name + ".txt"), "w") as fh:
            fh.write(f"{inst.n} {inst.m}\n")
            fh.writelines(f"{u} {v}\n" for u, v in inst.edges)


if __name__ == "__main__":
    main()
