"""Structure of BOBILib's general and assignment instances, read from the instance files.

usage: python3 -I scripts/instance_structure.py --meta collection.json --collection data/collection
           [--out results/notes/instance_structure.csv] [--jobs 2] [NAME ...]

For every instance of the families general-pure, general-mixed and assignment (BOBILib's
"Class"/"Type" in collection.json), reads NAME.mps.gz and NAME.aux and writes one CSV row:

  instance, collection, family     name, BOBILib sub-collection (directory), family
  cols, rows                       variables; rows other than N rows
  leader_vars, follower_vars       follower variables = the AUX @VARS list
  leader_rows, follower_rows       follower rows = the AUX @CONSTRS list
  leader_vars_prefix               1 if the leader variables are the first leader_vars columns
  follower_vars_prefix             1 if the follower variables are the first follower_vars columns
  leader_rows_prefix               1 if the leader rows are the first leader_rows rows
  coupling_rows                    leader rows with a nonzero on a follower variable
  linking_vars                     leader variables with a nonzero in a follower row
  continuous_linking_vars          of those, the continuous ones (not in an INTORG block and
                                   without a BV, BI, LI or UI bound)
  fobj_exact                       follower variables whose AUX cost is exactly -c_j (c: the MPS objective)
  fobj_rounded                     ... equal to -c_j only after rounding to 6 significant digits
  fobj_other                       ... neither
  leader_obj_on_leader_vars        leader variables with a nonzero MPS objective coefficient
  meta_coupling, meta_continuous_linking   BOBILib's "Number of coupling constraints" and
                                   "Number of continuous linking variables", for comparison

Numbers are compared as Python floats parsed from the files. Shares no code with the Rust crate.
At most --jobs processes (default 2).
"""
import argparse
import csv
import gzip
import multiprocessing
import os
import sys

FIELDS = ["instance", "collection", "family", "cols", "rows", "leader_vars", "follower_vars", "leader_rows",
          "follower_rows", "leader_vars_prefix", "follower_vars_prefix", "leader_rows_prefix", "coupling_rows",
          "linking_vars", "continuous_linking_vars", "fobj_exact", "fobj_rounded", "fobj_other",
          "leader_obj_on_leader_vars", "meta_coupling", "meta_continuous_linking"]


def family(v):
    if v["Class"] == "interdiction":
        return "assignment" if v["Type"] == "assignment" else None
    return "general-" + ("mixed" if v["Class"] == "mixed integer" else "pure")


def read_aux(path):
    fvars, fcons, sec = {}, set(), None
    with open(path, encoding="latin-1") as fh:
        for line in fh:
            s = line.strip()
            if not s:
                continue
            if s.startswith("@"):
                sec = {"@VARSBEGIN": "V", "@CONSTRSBEGIN": "C", "@CONSTRBEGIN": "C"}.get(s)
                continue
            if sec == "V":
                t = s.split()
                fvars[t[0]] = float(t[1])
            elif sec == "C":
                fcons.add(s.split()[0])
    return fvars, fcons


def scan(job):
    name, base, coll, fam, meta = job
    fvars, fcons = read_aux(base + ".aux")
    obj = None
    rorder = []
    corder, seen = [], set()
    cobj = {}
    integer = set()
    coupling, linking = set(), set()
    sec, intblock = None, False
    with gzip.open(base + ".mps.gz", "rt", encoding="latin-1") as fh:
        for line in fh:
            if not line.strip() or line[0] == "*":
                continue
            if not line[0].isspace():
                sec = line.split()[0]
                continue
            t = line.split()
            if sec == "ROWS":
                if t[0] == "N":
                    if obj is None:
                        obj = t[1]
                else:
                    rorder.append(t[1])
            elif sec == "COLUMNS":
                if len(t) >= 3 and t[1] == "'MARKER'":
                    intblock = t[2] == "'INTORG'"
                    continue
                c = t[0]
                if c not in seen:
                    seen.add(c)
                    corder.append(c)
                    if intblock:
                        integer.add(c)
                isf = c in fvars
                for i in range(1, len(t) - 1, 2):
                    r = t[i]
                    if r == obj:
                        cobj[c] = float(t[i + 1])
                    elif isf:
                        if r not in fcons:
                            coupling.add(r)
                    elif r in fcons:
                        linking.add(c)
            elif sec == "BOUNDS":
                if t[0] in ("BV", "BI", "LI", "UI"):
                    integer.add(t[2])
    leader = [c for c in corder if c not in fvars]
    lrows = [r for r in rorder if r not in fcons]
    nl, nf = len(leader), len(fvars)
    ex = rd = ot = 0
    for v, f in fvars.items():
        target = -cobj.get(v, 0.0)
        if f == target:
            ex += 1
        elif target != 0 and float("%.6g" % target) == f:
            rd += 1
        else:
            ot += 1
    lead_set = set(leader)
    return dict(
        instance=name, collection=coll, family=fam, cols=len(corder), rows=len(rorder), leader_vars=nl,
        follower_vars=nf, leader_rows=len(lrows), follower_rows=len(rorder) - len(lrows),
        leader_vars_prefix=int(set(corder[:nl]) == lead_set),
        follower_vars_prefix=int(set(corder[:nf]) == set(fvars)),
        leader_rows_prefix=int(set(rorder[:len(lrows)]) == set(lrows)),
        coupling_rows=len(coupling), linking_vars=len(linking),
        continuous_linking_vars=sum(1 for c in linking if c not in integer),
        fobj_exact=ex, fobj_rounded=rd, fobj_other=ot,
        leader_obj_on_leader_vars=sum(1 for c in leader if cobj.get(c, 0.0) != 0),
        meta_coupling=meta.get("Number of coupling constraints", ""),
        meta_continuous_linking=meta.get("Number of continuous linking variables", ""))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--meta", required=True)
    ap.add_argument("--collection", required=True)
    ap.add_argument("--out", default="results/notes/instance_structure.csv")
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("names", nargs="*")
    a = ap.parse_args()
    meta = __import__("json").load(open(a.meta))
    paths = {}
    for root, _, names in os.walk(a.collection):
        for n in names:
            if n.endswith(".mps.gz"):
                paths[n[: -len(".mps.gz")]] = os.path.join(root, n[: -len(".mps.gz")])
    jobs = []
    for k, v in meta.items():
        fam = family(v)
        if fam is None or (a.names and k not in a.names):
            continue
        base = paths.get(k)
        if base is None or not os.path.exists(base + ".aux"):
            sys.exit(f"instance_structure: no files for {k}")
        jobs.append((k, base, os.path.basename(os.path.dirname(base)), fam, v))
    jobs.sort(key=lambda j: os.path.getsize(j[1] + ".mps.gz"))
    with multiprocessing.Pool(max(1, min(a.jobs, 2))) as pool:
        rows = list(pool.imap(scan, jobs, chunksize=4))
    rows.sort(key=lambda r: (r["family"], r["collection"], r["instance"]))
    with open(a.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=FIELDS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    print(f"instance_structure: {len(rows)} instances -> {a.out}")


if __name__ == "__main__":
    main()
