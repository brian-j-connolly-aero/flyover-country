"""Summarise the state of every BOBILib instance that BOBILib itself does not
list as optimal: what the literature already settles, what this work adds,
and what remains open. Writes results/summary.json and prints a table.

usage: python3 -I summarize.py --meta collection.json --mkip-lit mkip_literature_status.csv
           --runs runs --results results [--miblp-lit miblp_literature_status.csv]
           [--lbc-optimal optimal.csv]

Inputs (all optional except --meta; missing ones are skipped):
  results/kip/summary.csv         KIP optima from Weninger and Fukasawa (verified files)
  results/ss/proofs.csv            our subset-sum optima (bobi ss-solve, verified files)
  runs/ss_*.tsv                    further local subset-sum runs (bobi ss-solve)
  runs/or_compare.csv              OR bounds vs literature (scripts/compare_or.py)
  results/clique/summary.csv      clique interdiction certificates
  results/general/hpr_optimal.csv general points meeting the HPR bound
  runs/general.jsonl               general feasible points (bobi general)
  runs/ray_verify.jsonl            exact unbounded-follower certificates
"""
import argparse
import collections
import csv
import glob
import json
import os
import re


def load_csv(path):
    return list(csv.DictReader(open(path))) if os.path.exists(path) else []


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--meta", required=True)
    ap.add_argument("--mkip-lit")
    ap.add_argument("--miblp-lit")
    ap.add_argument("--lbc-optimal", help="solution_files/optimal.csv from Lozano et al.'s repository")
    ap.add_argument("--runs", default="runs")
    ap.add_argument("--results", default="results")
    a = ap.parse_args()
    meta = json.load(open(a.meta))

    def family(v):
        if v["Class"] == "interdiction":
            return {"knapsack": "knapsack", "multidimensional knapsack": "multidim-knapsack", "edge clique": "clique",
                    "generalized": "generalized", "network": "network", "assignment": "assignment"}[v["Type"]]
        return "general-" + ("mixed" if v["Class"] == "mixed integer" else "pure")

    status = {}  # name -> (bobilib, outcome, source)
    for k, v in meta.items():
        status[k] = {"family": family(v), "bobilib": v["Status"], "outcome": None, "source": None}

    # Literature: KIP optima (Weninger-Fukasawa), imkp (Fischetti et al. 2019), OR optima (Lozano et al.).
    for r in load_csv(f"{a.results}/kip/summary.csv"):
        if r.get("literature_optimum") not in ("", None) and r["name"] in status and status[r["name"]]["bobilib"] != "optimal":
            status[r["name"]].update(outcome="optimal", source="literature (Weninger-Fukasawa 2025)")
    if a.mkip_lit:
        for r in load_csv(a.mkip_lit):
            n = r["name"]
            if n in status and status[n]["bobilib"] != "optimal" and r.get("proven_optimal_in_literature") == "True":
                src = "literature (Fischetti et al. 2019)" if r.get("set") == "imkp" else "literature (Lozano et al. 2026)"
                status[n].update(outcome="optimal", source=src)
    if a.lbc_optimal:
        # Lozano, Bergman and Cire's optimal solution files (decision diagrams), any family.
        for r in load_csv(a.lbc_optimal):
            n = re.sub(r"(\.dd_\d+)?[-.]res\.json$", "", re.sub(r"^dd_", "", r["name"]))
            if n in status and status[n]["bobilib"] != "optimal" and status[n]["outcome"] is None:
                status[n].update(outcome="optimal", source="literature (Lozano et al. 2026)")
    if a.miblp_lit:
        for r in load_csv(a.miblp_lit):
            n = r["instance"]
            if n in status and status[n]["bobilib"] != "optimal" and status[n]["outcome"] is None and r["closed_in_literature"] == "True":
                src = r["lit_best_lb_src"].split("-")[0]
                src = {"F17": "Fischetti et al. 2017", "LBC": "Lozano et al. 2026", "VLH": "Vasquez et al. 2025", "BOBILib": "an earlier BOBILib release"}.get(src, src)
                status[n].update(outcome="optimal", source=f"literature ({src})")
    # This work: subset-sum optima (committed proofs, then local runs).
    for r in load_csv(f"{a.results}/ss/proofs.csv"):
        n = r["name"]
        if n in status and status[n]["outcome"] is None:
            status[n].update(outcome="optimal", source="this work (robust subset-sum search)")
    for p in glob.glob(f"{a.runs}/ss_*.tsv"):
        for line in open(p):
            t = line.rstrip("\n").split("\t")
            if len(t) >= 3 and t[1] == "OPTIMAL" and t[0] in status and status[t[0]]["outcome"] is None:
                status[t[0]].update(outcome="optimal", source="this work (robust subset-sum search)")
    # This work: OR closures and first points.
    for r in load_csv(f"{a.runs}/or_compare.csv"):
        n = r["name"]
        if n not in status or status[n]["outcome"]:
            continue
        f = lambda s: float(s) if s not in ("", None, "None") else None
        point, lb = f(r["our_point"]), f(r["best_lb"])
        if point is not None and lb is not None and point <= lb + 1e-6:
            status[n].update(outcome="optimal", source="this work (certified point meets best lower bound)")
        elif point is not None and status[n]["bobilib"] == "open":
            status[n].update(outcome="feasible", source="this work (first certified point)" if f(r["lit_ub"]) is None else "this work (certified point)")
    # This work: clique certificates.
    for r in load_csv(f"{a.results}/clique/summary.csv"):
        if r.get("status") == "optimal" and r["name"] in status and status[r["name"]]["outcome"] is None:
            status[r["name"]].update(outcome="optimal", source="this work (Mantel/Brouwer + partitions)")
    # This work: unbounded followers (no bilevel feasible point).
    p = f"{a.runs}/ray_verify.jsonl"
    if os.path.exists(p):
        for line in open(p):
            if not line.startswith("{"):
                continue
            d = json.loads(line)
            n = d["instance"].replace(".mps.gz", "")
            # A later exact check (after repair) upgrades an earlier floating one.
            if n in status and status[n]["bobilib"] == "open" and (d["verified"] or status[n]["outcome"] is None):
                status[n].update(outcome="no bilevel feasible point",
                                 source="this work (recession ray, exact)" if d["verified"] else "this work (recession ray, floating point)")
    # This work: general points meeting the HPR (MIPLIB) bound.
    for r in load_csv(f"{a.results}/general/hpr_optimal.csv"):
        if r["name"] in status and status[r["name"]]["outcome"] is None:
            status[r["name"]].update(outcome="optimal", source="this work (point meets the MIPLIB optimum of the HPR)")
    # This work: general feasible points.
    p = f"{a.runs}/general.jsonl"
    if os.path.exists(p):
        for line in open(p):
            if not line.startswith("{"):
                continue
            d = json.loads(line)
            n = d.get("name")
            if n in status and status[n]["outcome"] is None and d.get("verified") and status[n]["bobilib"] == "open":
                status[n].update(outcome="feasible", source="this work (HPR, follower, tie-break)")

    rows = collections.defaultdict(collections.Counter)
    for n, s in status.items():
        if s["bobilib"] == "optimal":
            continue
        rows[s["family"]]["not optimal in BOBILib"] += 1
        key = (s["outcome"] or "unchanged") + (" / " + s["source"] if s["source"] else "")
        rows[s["family"]][key] += 1
    os.makedirs(a.results, exist_ok=True)
    json.dump({"per_instance": status, "per_family": {k: dict(v) for k, v in rows.items()}}, open(f"{a.results}/summary.json", "w"), indent=1)
    for fam in sorted(rows):
        print(f"== {fam}")
        for k, v in sorted(rows[fam].items(), key=lambda kv: -kv[1]):
            print(f"   {v:5d}  {k}")


if __name__ == "__main__":
    main()
