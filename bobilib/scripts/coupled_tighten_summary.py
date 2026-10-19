"""Fold the strict re-derivation of points into a folder's summary table.

usage: python3 -I scripts/coupled_tighten_summary.py SUMMARY.csv TIGHTEN.jsonl CERTIFY_TIGHT.tsv

SUMMARY.csv: results/general-coupled/summary.csv or results/general/points.csv.
TIGHTEN.jsonl: the output of scripts/tighten_coupled.sh (`bobi coupled-tighten`),
one JSON line per re-derived file. CERTIFY_TIGHT.tsv: `bobi certify --tight` on
every file of the folder. Rows of re-derived files get the new point's
objectives (and, where the table has those columns, errors and free_leader);
every row with a solution file (status `point`, and `verified` True where the
table has that column) gets `tightened` (all-fixed / linking-fixed, empty when
the file was kept), `follower_slack` (the point's follower value minus the
proven bound on the follower's optimum, absolute) and `tight_certified`
(yes / no). SUMMARY.csv is rewritten in place.
"""
import csv
import json
import sys


def certify_lines(path):
    out = {}
    for line in open(path):
        p = line.rstrip("\n").split("\t")
        if len(p) < 3:
            continue
        d = {"status": p[1]}
        for f in p[2:]:
            if "=" in f:
                k, v = f.split("=", 1)
                d[k] = v.split(" ")[0]
        out[p[0]] = d
    return out


def main():
    summ, tj, ct = sys.argv[1:4]
    tight = {}
    for line in open(tj):
        if line.startswith("{"):
            d = json.loads(line)
            tight[d["name"]] = d
    cert = certify_lines(ct)
    with open(summ, newline="") as f:
        rows = list(csv.DictReader(f))
    cols = list(rows[0].keys())
    for c in ("tightened", "follower_slack", "tight_certified"):
        if c not in cols:
            cols.append(c)
    for r in rows:
        n = r["name"]
        for c in ("tightened", "follower_slack", "tight_certified"):
            r.setdefault(c, "")
        if r["status"] != "point" or r.get("verified", "True") != "True":
            continue
        if n in tight:
            t = tight[n]
            assert t["status"] == "point", n
            c = cert[n]
            assert abs(float(c["upper"]) - t["upper"]) <= 1e-9 * max(1.0, abs(t["upper"])), n
            new = dict(upper_obj=repr(t["upper"]), lower_obj=repr(t["lower"]),
                       rel_error=c["rel_error"], tie_rel_error=c["tie_rel_error"],
                       max_violation=repr(t["max_violation"]),
                       free_leader=str(t["mode"] == "linking-fixed"),
                       tightened=t["mode"])
            r.update({k: v for k, v in new.items() if k in cols})
        c = cert.get(n)
        assert c is not None, f"no certify --tight line for {n}"
        r["follower_slack"] = c["slack"]
        r["tight_certified"] = "yes" if c["status"] == "VERIFIED" else "no"
    with open(summ, "w", newline="") as f:
        w = csv.DictWriter(f, cols, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    pts = [r for r in rows if r["status"] == "point" and r.get("verified", "True") == "True"]
    print(f"{len(pts)} points, {sum(r['tight_certified'] == 'yes' for r in pts)} tight-certified, "
          f"{sum(bool(r['tightened']) for r in pts)} re-derived")


main()
