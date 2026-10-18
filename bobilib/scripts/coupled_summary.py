"""Summarise `bobi coupled` runs into a CSV and print tallies.

usage: python3 -I scripts/coupled_summary.py COLLECTION_JSON OUT.csv RUN.jsonl...

Each instance keeps its most informative line: a point beats a finished
search without one, which beats a timeout or crash, which beats "not
attempted" (lines `scripts/coupled.sh` writes once its batch budget is
spent). Among equals the later line wins, so a re-run with other settings
can be appended to the same file or passed after it.
"""
import collections
import csv
import json
import os
import sys

RANK = {"point": 3, "no_bilevel_point": 2, "hpr_infeasible": 2, "none": 1, "error": 0, "not_attempted": -1}


def name_of(d):
    if "name" in d:
        return d["name"]
    if "file" in d:
        return os.path.basename(d["file"]).removesuffix(".mps.gz")
    return None


def main():
    meta_path, out_path, *runs = sys.argv[1:]
    meta = json.load(open(meta_path))
    best = {}
    for path in runs:
        for line in open(path):
            line = line.strip()
            if not line:
                continue
            d = json.loads(line)
            name = name_of(d)
            if name is None:
                continue
            d["name"] = name
            d.setdefault("status", "error" if "error" in d else "none")
            old = best.get(name)
            if old is not None and RANK.get(d["status"], 0) < RANK.get(old["status"], 0):
                continue
            best[name] = d
    fields = [
        "name", "vars", "follower_vars", "rows", "coupling_rows", "linking_vars",
        "integer_linking_vars", "bobilib_status", "bobilib_best", "first_point", "status",
        "method", "secs_to_point", "secs", "time_limit", "upper_obj", "lower_obj",
        "rel_error", "tie_rel_error", "max_violation", "free_leader", "methods_tried",
    ]
    rows = []
    tally = collections.Counter()
    by_method = collections.Counter()
    tried = collections.Counter()
    found = collections.Counter()
    first = 0

    def size(kv):
        m = meta.get(kv[0], {})
        nv = m.get("Number of upper-level variables", 0) + m.get("Number of lower-level variables", 0)
        return (kv[1].get("vars", nv), kv[0])

    for name, d in sorted(best.items(), key=size):
        m = meta.get(name, {})
        row = {k: d.get(k, "") for k in fields}
        if row["vars"] == "" and m:
            row["vars"] = m["Number of upper-level variables"] + m["Number of lower-level variables"]
            row["follower_vars"] = m["Number of lower-level variables"]
            row["coupling_rows"] = m["Number of coupling constraints"]
            row["linking_vars"] = m["Number of linking variables"]
        row["bobilib_status"] = m.get("Status", "")
        row["bobilib_best"] = m.get("Best known objective", "")
        is_first = d["status"] == "point" and row["bobilib_best"] in ("n.a.", "", None)
        row["first_point"] = "yes" if is_first else ("no" if d["status"] == "point" else "")
        row["methods_tried"] = "; ".join(f"{r['method']}: {r['end']}" for r in d.get("reports", []))
        rows.append(row)
        tally[d["status"]] += 1
        first += is_first
        if d["status"] == "point":
            by_method[d.get("method")] += 1
        for r in d.get("reports", []):
            tried[r["method"]] += 1
            found[r["method"]] += bool(r.get("found"))
    with open(out_path, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} instances:", dict(tally))
    print("points by method:", dict(by_method))
    print("first feasible points (BOBILib lists no objective):", first)
    print("method runs (instances where it ran / found the point):",
          {k: f"{tried[k]}/{found[k]}" for k in tried})


main()
