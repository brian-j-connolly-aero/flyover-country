#!/usr/bin/env python3
"""Build the data-package index in bobilib/results/.

Writes, from the result files under bobilib/results/ and BOBILib's instance
metadata (collection.json from bobilib.org):

  results/instances.csv        one row per BOBILib instance (2,654)
  results/bobilib_update.solu  status changes and improved values, .solu format
  results/README.md            the package index (IJOC repository style)
  results/SHA256SUMS           sha256 of every other file in results/

usage:
  python3 -I paper/scripts/build_data_package.py [--meta collection.json]
      [--mkip-lit mkip_literature_status.csv] [--miblp-lit miblp_literature_status.csv]
      [--lbc-optimal optimal.csv] [--results DIR]

Per-instance outcomes come from results/summary.json (scripts/summarize.py)
and are cross-checked against the per-folder tables. Without --meta, the
BOBILib columns (collection, status, best known value) are read back from the
existing results/instances.csv, which is the metadata snapshot of the package.
Published optima that are not stored under results/ (imkp, generalized and
MIPLIB-derived closures) come from the literature tables named by --mkip-lit,
--miblp-lit and --lbc-optimal (the ones scripts/summarize.py reads); without
them they are read back from the existing instances.csv as well. KIP and OR
literature values always come from results/kip/summary.csv and
results/or/compare.csv.

Every count the paper quotes for the package is asserted below.
"""
import argparse
import collections
import csv
import hashlib
import io
import json
import math
import os
import re
import sys
import tarfile

HERE = os.path.dirname(os.path.abspath(__file__))
BOBILIB = os.path.dirname(os.path.dirname(HERE))
SNAPSHOT = "2026-10-18"  # date of the BOBILib metadata snapshot (collection.json)

FAMILY_ORDER = ["knapsack", "multidim-knapsack", "clique", "network", "generalized", "assignment",
                "general-pure", "general-mixed"]

# Literature sources: summary.json label -> citation key used in instances.csv / README.
LIT_KEYS = {
    "literature (Weninger-Fukasawa 2025)": "WeningerFukasawa2025",
    "literature (Lozano et al. 2026)": "LozanoBergmanCire2026",
    "literature (Fischetti et al. 2019)": "FischettiEtAl2019",
    "literature (Fischetti et al. 2017)": "FischettiEtAl2017",
    "literature (Vasquez et al. 2025)": "VasquezLozanoVanHoeve2025",
    "literature (an earlier BOBILib release)": "BOBILibEarlierRelease",
}
LIT_METHOD = {
    "WeningerFukasawa2025": "combinatorial branch and bound (published optimum)",
    "LozanoBergmanCire2026": "decision-diagram network relaxation (published optimum)",
    "FischettiEtAl2019": "branch and cut with interdiction cuts (published optimum)",
    "FischettiEtAl2017": "MIX++ branch and cut (published optimum)",
    "VasquezLozanoVanHoeve2025": "decision-diagram reformulation (published optimum)",
    "BOBILibEarlierRelease": "listed optimal in an earlier BOBILib release",
}
BOBILIB_KEY = "ThuraufEtAl2026"
REFERENCES = [
    ("ThuraufEtAl2026", "J. Thürauf, T. Kleinert, I. Ljubić, T. Ralphs and M. Schmidt, BOBILib: Bilevel "
     "Optimization (Benchmark) Instance Library, Mathematical Programming Computation 18(3) (2026) "
     "819-876, doi:10.1007/s12532-025-00294-y; https://bobilib.org"),
    ("BOBILibEarlierRelease", "an earlier release of the BOBILib metadata, which listed the instance as optimal"),
    ("WeningerFukasawa2025", "N. Weninger and R. Fukasawa, A fast combinatorial algorithm for the bilevel "
     "knapsack problem with interdiction constraints, Mathematical Programming 210 (2025) 847-879, "
     "doi:10.1007/s10107-024-02133-9"),
    ("LozanoBergmanCire2026", "L. Lozano, D. Bergman and A. A. Cire, Network relaxations for combinatorial bilevel "
     "optimization under linear interactions, Operations Research (2026), Articles in Advance, "
     "doi:10.1287/opre.2025.2457 (arXiv:2407.18193)"),
    ("FischettiEtAl2019", "M. Fischetti, I. Ljubić, M. Monaci and M. Sinnl, Interdiction games and "
     "monotonicity, with application to knapsack problems, INFORMS Journal on Computing 31(2) (2019) "
     "390-410, doi:10.1287/ijoc.2018.0831"),
    ("FischettiEtAl2017", "M. Fischetti, I. Ljubić, M. Monaci and M. Sinnl, A new general-purpose algorithm "
     "for mixed-integer bilevel linear programs, Operations Research 65(6) (2017) 1615-1637, "
     "doi:10.1287/opre.2017.1650"),
    ("VasquezLozanoVanHoeve2025", "S. Vásquez, L. Lozano and W.-J. van Hoeve, A single-level reformulation of "
     "binary bilevel programs using decision diagrams, Mathematical Programming (2025), online first, "
     "doi:10.1007/s10107-025-02294-1"),
]

# Hand-written notes that come from the text of REPORT.md or a results README
# rather than from a table (prose only, never a value used elsewhere).
EXTRA_NOTES = {
    "general20-30-20-20-20-2": "BOBILib's solution file has follower value 106899984.14 against the follower's "
                               "optimum 106899953 at the same leader choice (31 units, relative 2.9e-7) and row "
                               "violations up to 3.3e-4; BOBILib's checker accepts it (relative 1e-4 on the "
                               "follower's value, 1e-4 on the rows), bobi verify rejects it "
                               "(notes/general20-30-20-20-20-2.verify.tsv)",
    "timtab1_50_50": "HPR is exactly MIPLIB's timtab1 (optimum 764772), a lower bound; this point is 764774 "
                     "(general-coupled/README.md)",
    # Lefebvre and Schmidt's run files (notes/lefebvre_schmidt.md, sections 1 and 3).
    "ex10_0_100": "Lefebvre and Schmidt's (2025) MILP and MILP-First run files report the same optimum, 100 "
                  "(Optimal/Proved, about 624 s); their paper does not state it, so it is not counted as a published "
                  "optimum (notes/lefebvre_schmidt.md)",
    "ex10_50_50": "HPR is exactly MIPLIB's ex10 (scripts/compare_hpr.py), so MIPLIB's optimum 100 is a lower bound; "
                  "Lefebvre and Schmidt's (2025) MILP and MILP-First run files report Optimal/Proved 100, not verified "
                  "here, and no point is known here (notes/lefebvre_schmidt.md)",
    "highschool1-aigio_0_100": "Lefebvre and Schmidt's (2025) PADM run file lists a point of value 0 (Feasible/Proved, "
                               "results-PADM.csv lines 789 and 794), and their paper counts the instance among the open "
                               "ones where PADM found a feasible point; it cannot be bilevel feasible: the exact recession "
                               "direction here leaves the follower without an optimal response for every leader choice "
                               "(notes/lefebvre_schmidt.md)",
}

# Published points for general instances that are not in miblp_literature_status.csv: they appear only
# in the per-instance run files that Lefebvre and Schmidt publish with their 2025 preprint
# (henrilefebvre.com, open-science page of the paper: results/results-PADM.csv and results-MILP.csv,
# column best_obj). Used only to decide whether a point here is the first one anywhere.
EXTRA_PUBLISHED = {
    "rmine6_0_100": (-3.11093, "Lefebvre and Schmidt (2025), PADM run files"),
    "rmine6_50_50": (-3.11093, "Lefebvre and Schmidt (2025), PADM run files"),
    "ex10_0_100": (100, "Lefebvre and Schmidt (2025), MILP run files, reported there as optimal"),
    "neos-631710_0_100": (556, "Lefebvre and Schmidt (2025), MILP-First run file"),
}

# OR instances proven optimal in Lozano, Bergman and Cire's published data although the lower
# bound in their paper's table exceeds their point: their repository's branch-and-cut bound
# (results_solvers_plus_dd_Mar24_2026_mibs_bc_augmented.csv, bilevelsolver_plus_dd) equals it.
OR_BC_CLOSED = {
    "OR10x100-0.75_4.dat-0": (8813.0, "published point 8813; the published decision-diagram lower bound 8814 "
                              "comes from runs its authors flag as numerically inconsistent; their repository's "
                              "branch-and-cut bound is 8813, which closes the instance"),
}

# Authoritative counts (paper spec). The script fails if the data disagree.
EXPECT = dict(
    instances=2654, bobilib_optimal=1027, bobilib_infeasible=33, bobilib_feasible=1175, bobilib_open=419,
    unsolved=1594, literature=528, new_optimal=82, infeasible_bilevel=40, first_point=228, unchanged=716,
    settled=650, optimal_now=610, packing=1254,
)
CLASSES = ("exact", "numerical", "benchmark")  # certificate classes (i), (ii), (iii) of the appendix


def die(msg):
    sys.exit(f"build_data_package: {msg}")


def check(cond, msg):
    if not cond:
        die("assertion failed: " + msg)


def num(s):
    if s is None:
        return None
    if isinstance(s, (int, float)):
        return float(s)
    s = str(s).strip()
    if s in ("", "None", "n.a.", "nan"):
        return None
    m = re.match(r"^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?", s)
    return float(m.group(0)) if m else None


def fmt(v):
    """Full precision: integers without a decimal point, other values by repr (round trip)."""
    if v is None:
        return ""
    v = float(v)
    if v.is_integer() and abs(v) < 1e15:
        return str(int(v))
    return repr(v)


def natural(name):
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r"(\d+)", name)]


def load_csv(path, delimiter=","):
    with open(path, newline="") as f:
        return list(csv.DictReader(f, delimiter=delimiter))


def load_tsv_results(path):
    """bobi verify output: name -> result (PASS/FAIL); skips the summary line."""
    out = {}
    for r in load_csv(path, "\t"):
        if r.get("result") in ("PASS", "FAIL"):
            out[r["name"]] = r["result"]
    return out


def res_objectives(res):
    """Objective of every current BOBILib solution file under results/ (including archive members).

    The tolerance/ folders hold superseded files (benchmark-accepted points replaced by tight ones,
    general/README.md and general-coupled/README.md, "Strict tolerance"); they are not current."""
    files = {}
    for root, dirs, names in os.walk(res):
        dirs[:] = [d for d in dirs if d != "tolerance"]
        for n in names:
            p = os.path.join(root, n)
            rel = os.path.relpath(p, res)
            if n.endswith(".res.json"):
                d = json.load(open(p))
                files[rel] = (d["name"], d["objective_value"], d["status"])
            elif n.endswith(".tar.gz"):
                with tarfile.open(p) as t:
                    for m in t.getmembers():
                        if m.isfile() and m.name.endswith(".res.json"):
                            d = json.load(t.extractfile(m))
                            files[f"{rel}:{os.path.basename(m.name)}"] = (d["name"], d["objective_value"], d["status"])
    return files


def collection_of(name, v):
    tags = [t.strip() for t in v["Tags"].split(",")]
    if len(tags) >= 2 and not tags[-1].startswith("interdiction class"):
        return tags[-1]
    # 60 pure-integer instances carry no sub-collection tag; BOBILib stores them under denegre/.
    check(v["Class"] == "pure integer" and name.startswith("milp_"), f"no sub-collection for {name}")
    return "denegre"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--results", default=os.path.join(BOBILIB, "results"))
    ap.add_argument("--meta", help="BOBILib collection.json")
    ap.add_argument("--collection-dir", help="optional BOBILib collection/ directory to cross-check paths")
    ap.add_argument("--mkip-lit")
    ap.add_argument("--miblp-lit")
    ap.add_argument("--lbc-optimal")
    a = ap.parse_args()
    R = os.path.abspath(a.results)
    prev_path = os.path.join(R, "instances.csv")
    prev = {r["instance"]: r for r in load_csv(prev_path)} if os.path.exists(prev_path) else {}

    summary = json.load(open(os.path.join(R, "summary.json")))["per_instance"]

    # ---- BOBILib metadata --------------------------------------------------------------
    bob = {}
    if a.meta:
        meta = json.load(open(a.meta))
        for k, v in meta.items():
            bob[k] = dict(collection=collection_of(k, v), status=v["Status"], value=num(v["Best known objective"]))
            if v["Tags"].split(", ")[-1] == "or":  # follower rows used by the OR table: -0: m-1, -50: m//2, -100: 1
                m = re.match(r"OR(\d+)x\d+-[\d.]+_\d+\.dat-(\d+)$", k)
                rows, var = int(m.group(1)), int(m.group(2))
                fr = v["Number of lower-level constraints"] - v["Number of linking constraints"]
                check(fr == {0: rows - 1, 50: rows // 2, 100: 1}[var], f"follower rows of {k}")
        mode_meta = f"metadata {a.meta}"
    else:
        check(prev, "no --meta and no existing results/instances.csv to take the BOBILib columns from")
        for k, r in prev.items():
            bob[k] = dict(collection=r["collection"], status=r["bobilib_status"], value=num(r["bobilib_value"]))
        mode_meta = "BOBILib columns of the existing instances.csv"
    if a.collection_dir:
        for root, _, names in os.walk(a.collection_dir):
            for n in names:
                if n.endswith(".mps.gz"):
                    k = n[: -len(".mps.gz")]
                    check(bob[k]["collection"] == os.path.basename(root), f"collection of {k}")
    check(set(bob) == set(summary), "metadata and summary.json list different instances")
    for k, s in summary.items():
        check(bob[k]["status"] == s["bobilib"], f"BOBILib status of {k}")

    # ---- result tables -------------------------------------------------------------------
    objs = res_objectives(R)
    by_name = collections.defaultdict(dict)  # instance -> {folder-relative path: (objective, status)}
    for rel, (n, obj, st) in objs.items():
        by_name[n][rel] = (obj, st)
    kip = {r["name"]: r for r in load_csv(f"{R}/kip/summary.csv")}
    ss = {r["name"]: r for r in load_csv(f"{R}/ss/proofs.csv")}
    orc = {r["name"]: r for r in load_csv(f"{R}/or/compare.csv")}
    clq = {r["name"]: r for r in load_csv(f"{R}/clique/summary.csv")}
    net = {r["name"]: r for r in load_csv(f"{R}/network/summary.csv")}
    net_highs = {r["name"]: r for r in load_csv(f"{R}/network/highs_crosscheck.csv")}
    net_review = {r["name"]: r for r in load_csv(f"{R}/network/review.tsv", "\t")}
    hpr = {r["name"]: r for r in load_csv(f"{R}/general/hpr_optimal.csv")}
    gpts = {r["name"]: r for r in load_csv(f"{R}/general/points.csv")}
    cpl = {r["name"]: r for r in load_csv(f"{R}/general-coupled/summary.csv")}
    # bobi verify --tight over every current solution file (notes/tight_verify.md): folder -> name -> class
    tight = collections.defaultdict(dict)
    for r in load_csv(f"{R}/notes/tight_verify.tsv", "\t"):
        tight[r["folder"]][r["name"]] = r
    # Lefebvre and Schmidt's run files against this package (notes/lefebvre_schmidt.md): instance -> categories
    ls_notes = collections.defaultdict(list)
    for r in load_csv(f"{R}/notes/lefebvre_schmidt_discrepancies.tsv", "\t"):
        if r["category"] not in ls_notes[r["instance"]]:
            ls_notes[r["instance"]].append(r["category"])
    small_clique = {}
    for p in ("small_bcpins.jsonl", "small_plusbcpins.jsonl"):
        for line in open(f"{R}/clique/runs/{p}"):
            if line.startswith("{"):
                d = json.loads(line)
                small_clique[d["inst"]] = d
    # Case certificates of w3(G) > k (clique/certificates/check.log of scripts/check_clique_certificates.py):
    # instance -> number of part-size cases certified; only instances whose check passed.
    clique_certs, cur, ok_cases = {}, None, 0
    log = open(f"{R}/clique/certificates/check.log").read()
    check("\nRESULT PASS\n" in log, "clique case certificates: check.log has no RESULT PASS")
    for line in log.splitlines():
        m = re.match(r"(normalClique\S+): n=\d+ .* (\d+) with b\(s\) >= 0$", line)
        if m:
            cur, want_cases, ok_cases = m.group(1), int(m.group(2)), 0
        elif re.match(r"  sizes \(.*\) b=\d+: tree OK", line):
            ok_cases += 1
        elif line.startswith("  PASS: w3(G) > k") and cur:
            check(ok_cases == want_cases, f"case count of {cur}")
            clique_certs[cur] = ok_cases
            cur = None
    rays = {}
    for line in open(f"{R}/rays/ray_verify.jsonl"):
        if line.startswith("{"):
            d = json.loads(line)
            n = d["instance"].replace(".mps.gz", "")
            if d["verified"] or n not in rays:  # a later exact check supersedes an earlier failed one
                rays[n] = d
    verified = {}
    for p in ("or/verify_one_knapsack.tsv", "or/verify_several_knapsacks.tsv"):
        for n, res in load_tsv_results(f"{R}/{p}").items():
            verified[f"or/{n}.res.json"] = res == "PASS"
    for n, res in load_tsv_results(f"{R}/network/verify_ours.tsv").items():
        verified[f"network/{n}.res.json"] = res == "PASS"
    for n, res in load_tsv_results(f"{R}/clique/review/bobi_verify.tsv").items():
        verified[f"clique/{n}.res.json"] = res == "PASS"
    for rel, (n, _, _) in objs.items():
        top = rel.split("/")[0]
        if top in ("kip", "ss"):  # both folders' READMEs: every file passes bobi verify (re-run for this package)
            verified[rel] = True
        elif top == "general":
            verified[rel] = gpts.get(n, {}).get("verified") == "True"
        elif top == "general-coupled":
            verified[rel] = cpl.get(n, {}).get("status") == "point"
    for rel in objs:
        check(rel in verified, f"no verification record for {rel}")

    # ---- literature values -----------------------------------------------------------------
    lit = {}  # instance -> (value, note)
    for n, r in kip.items():
        if num(r["literature_optimum"]) is not None:
            lit[n] = (num(r["literature_optimum"]), "")
    for n, r in orc.items():
        if summary[n]["source"] == "literature (Lozano et al. 2026)":
            if n in OR_BC_CLOSED:
                lit[n] = OR_BC_CLOSED[n]
                continue
            check(num(r["lit_lb"]) == num(r["lit_ub"]), f"published OR optimum of {n}")
            lit[n] = (num(r["lit_ub"]), "")
    lit_sources = ["results/kip/summary.csv", "results/or/compare.csv"]
    # Who published the best OR point (compare.csv's lit_ub): Lozano, Bergman and Cire, whose
    # repository holds a solution file for it, or Lefebvre and Schmidt (value only).
    or_pub_src = {}
    # Lefebvre and Schmidt's values from runs stopped at the time limit (scripts/ls_mkip_points.py):
    # not counted as published points, as in their own analysis; noted where one would be the best.
    or_ls_timelimit = {}
    if a.mkip_lit:
        for r in load_csv(a.mkip_lit):
            tl = num(r.get("lefebvre_schmidt_ub_timelimit"))
            if r["set"] == "or" and tl is not None and (num(r["best_known_ub"]) is None or tl < num(r["best_known_ub"])):
                or_ls_timelimit[r["name"]] = tl
            if r["set"] != "or" or num(r["best_known_ub"]) is None:
                continue
            b, dd, ls_ = num(r["best_known_ub"]), num(r["dd_lcb2026_ub"]), num(r["lefebvre_schmidt_ub"])
            or_pub_src[r["name"]] = ("Lozano, Bergman and Cire (2026), solution file in their repository"
                                     if dd is not None and dd == b else
                                     "Lefebvre and Schmidt (2025), value only" if ls_ is not None and ls_ == b
                                     else "published")
    # Best published point of the general instances BOBILib lists as feasible or open.
    gen_pub = {}
    if a.miblp_lit:
        for r in load_csv(a.miblp_lit):
            if num(r["lit_best_ub"]) is not None:
                src = r["lit_best_ub_src"]
                who = ("Fischetti et al. (2017)" if src.startswith("F17") else
                       "Vasquez, Lozano and van Hoeve (2025)" if src.startswith("VLH") else
                       "Lozano, Bergman and Cire (2026)" if src.startswith("LBC") else
                       "an earlier BOBILib release" if src.startswith("BOBILib") else src)
                gen_pub[r["instance"]] = (num(r["lit_best_ub"]), who)
    else:  # read back from the existing instances.csv
        for n, p in prev.items():
            for x in p["notes"].split("; "):
                m = re.match(r"published point (\S+) \((.*?)\)", x)
                if m and p["family"].startswith("general-"):
                    gen_pub[n] = (num(m.group(1)), m.group(2))
    for n, v in EXTRA_PUBLISHED.items():
        gen_pub.setdefault(n, v)
    if a.mkip_lit:
        for r in load_csv(a.mkip_lit):
            if r["set"] == "imkp" and r["proven_optimal_in_literature"] == "True":
                lit.setdefault(r["name"], (num(r["flms2019_opt"]), ""))
        lit_sources.append(os.path.basename(a.mkip_lit))
    if a.miblp_lit:
        for r in load_csv(a.miblp_lit):
            if r["closed_in_literature"] == "True":
                val = num(r["lit_best_ub"])
                m = re.search(r"([-+\d.eE]+) within tolerance", r["lit_best_ub_src"])
                note = (f"published: {fmt(val)} with an exactly optimal follower ({m.group(1)} only within "
                        f"BOBILib's 1e-4 follower tolerance)") if m else ""
                lit.setdefault(r["instance"], (val, note))
        lit_sources.append(os.path.basename(a.miblp_lit))
    if a.lbc_optimal:
        for r in load_csv(a.lbc_optimal):
            n = r["name"]
            if n in summary and summary[n]["family"] == "generalized":
                lit.setdefault(n, (num(r["UB"]), ""))
        lit_sources.append(os.path.basename(a.lbc_optimal))
    for n, r in hpr.items():
        if summary[n]["source"] == "literature (an earlier BOBILib release)":
            lit.setdefault(n, (num(r["optimum"]), ""))
    for n, s in summary.items():
        if s["source"] in LIT_KEYS and s["bobilib"] != "optimal" and n not in lit:
            p = prev.get(n)
            check(p is not None and p["source"] == LIT_KEYS[s["source"]] and p["primal_value"] != "",
                  f"no published value for {n}: pass the literature tables (--mkip-lit, --miblp-lit, --lbc-optimal)")
            pnotes = [x for x in p["notes"].split("; ") if x.startswith("published: ")]
            lit[n] = (num(p["primal_value"]), pnotes[0] if pnotes else "")
            if "previous instances.csv" not in lit_sources:
                lit_sources.append("previous instances.csv")

    # ---- rows ------------------------------------------------------------------------------
    rows = []
    for n in summary:
        s, b = summary[n], bob[n]
        fam = s["family"]
        files = by_name.get(n, {})
        row = dict(instance=n, collection=b["collection"], family=fam, bobilib_status=b["status"],
                   bobilib_value=b["value"], new_status="", primal_value=None, dual_bound=None, source="",
                   method="", certificate="", verified="na", certificate_class="na", notes=[])
        notes = row["notes"]
        outcome, src = s["outcome"], s["source"]
        bval = b["value"]

        def cert(rel, ok=None):
            row["certificate"] = rel
            v = verified.get(rel) if ok is None else ok
            row["verified"] = "yes" if v else "no"

        def tight_class(rel):
            """numerical if the file passes bobi verify --tight, benchmark if only BOBILib's tolerance."""
            folder, _, member = rel.partition(":")
            folder = folder if member else os.path.dirname(rel)
            t = tight[folder][n]
            check(t["result"] == "PASS" or t["class"] == "benchmark-accepted", f"tight verification of {rel}")
            return "numerical" if t["class"] == "pass" else "benchmark"

        def tightened_note(r, folder):
            if r.get("tightened"):
                how = {"all-fixed": "every leader value kept", "linking-fixed": "linking variables kept"}[r["tightened"]]
                old = tight[f"{folder}/tolerance"][n]
                what = ("a benchmark-accepted point" if old["class"] == "benchmark-accepted" else
                        f"which already passed the tight test (follower slack {float(old['follower_slack']):.1e})")
                notes.append(f"re-derived with the follower within 1e-9 of its optimum (bobi coupled-tighten, {how}); "
                             f"the superseded file, {what}, is in {folder}/tolerance/")
            if r.get("tight_certified") == "no":
                notes.append("passes bobi verify --tight; bobi certify --tight gets no follower bound (the follower's "
                             "problem at these leader values is infeasible at its feasibility tolerance 1e-9, "
                             "general/README.md)")

        def own_file(prefix=None):
            """Our verified solution file for n (best objective), optionally within one folder."""
            c = [(obj, rel) for rel, (obj, _) in files.items() if prefix is None or rel.startswith(prefix)]
            return min(c) if c else (None, None)

        if b["status"] == "optimal":
            row["new_status"] = "bobilib_optimal"
            row["primal_value"] = row["dual_bound"] = bval
            row["source"] = BOBILIB_KEY
            row["method"] = "BOBILib's solver runs (MibS, MIX++)"
            obj, rel = own_file()
            if rel:
                check(abs(obj - bval) <= 1e-6 * max(1, abs(bval)), f"{n}: file value {obj} != BOBILib optimum {bval}")
                cert(rel)
            if fam == "knapsack":
                notes.append("optimum also published by Weninger and Fukasawa (2025); file rebuilt from their leader vector")
                if src:
                    notes.append("lower bound also re-proved by bobi ss-solve")
            elif fam == "network" and n in net:
                notes.append(f"re-derived: all {int(net[n]['attacks']):,} attacks enumerated, follower by exact tree DP")
            elif fam == "network" and n in net_highs:
                notes.append("re-derived: all attacks enumerated, follower solved by HiGHS at every attack")
            elif fam == "network" and net_review.get(n, {}).get("method") == "brute":
                notes.append("re-derived by brute force in the independent review (network/review.tsv)")
            elif fam == "network":
                notes.append("BOBILib's solution file passes bobi verify (network/verify_bobilib.tsv); optimality not re-derived")
            elif fam == "clique" and n in small_clique:
                d = small_clique[n]
                check(d["consistent"] and d["settled_by_args"] and d["exact"] == bval, f"small clique {n}")
                notes.append(f"re-derived: lower bound {d['arg_lb_why']}, upper bound {d['arg_ub_why']} edges "
                             f"(clique/runs/)")
        elif b["status"] == "infeasible":
            row["new_status"] = "bobilib_infeasible"
            row["source"] = BOBILIB_KEY
            row["method"] = "BOBILib's solver runs (MibS, MIX++)"
            if n in rays:
                check(rays[n]["verified"], f"ray of {n}")
                cert(f"rays/{n}.ray.json", True)
                notes.append(f"agrees: exact recession direction of the follower, f^T d = {rays[n]['objective_along_d']:g}")
        elif outcome == "optimal" and src in LIT_KEYS:
            key = LIT_KEYS[src]
            val, lnote = lit[n]
            row.update(new_status="optimal", primal_value=val, dual_bound=val, source=key, method=LIT_METHOD[key],
                       certificate_class="numerical")  # a published optimum, used as published
            if lnote:
                notes.append(lnote)
            obj, rel = own_file()
            if rel and abs(obj - val) <= 1e-6 * max(1, abs(val)):
                cert(rel)
                if fam == "knapsack":
                    notes.append("file rebuilt from the published leader vector, follower re-solved exactly")
            elif rel:
                notes.append(f"our verified point {fmt(obj)} ({rel}) is above the published optimum")
            if bval is None:
                notes.append("BOBILib lists no point")
            elif bval > val + 1e-6 * max(1, abs(val)):
                notes.append(f"BOBILib value {fmt(bval)} is {100 * (bval - val) / abs(val):.2f}% above the optimum")
            else:
                notes.append("BOBILib value already optimal")
        elif outcome == "optimal":  # this work
            row.update(new_status="optimal", source="this work")
            if fam == "knapsack":
                r = ss[n]
                val = num(r["optimum"])
                cert(f"ss/{r['file']}")
                row["certificate_class"] = "exact"
                row["method"] = "robust subset-sum search (bobi ss-solve)"
                notes.append(f"{int(r['nodes']):,} nodes, {r['cpu_seconds']} s CPU; lower bound re-proved by "
                             f"scripts/ss_check.py ({int(r['independent_check_nodes']):,} nodes)")
            elif fam == "clique":
                r = clq[n]
                val = num(r["upper_bound"])
                check(r["status"] == "optimal" and num(r["lower_bound"]) == val, f"clique {n}")
                cert(f"clique/{n}.res.json")
                row["certificate_class"] = "exact"
                if r["lb_argument"].startswith("Mantel"):
                    row["method"] = "Mantel's theorem + 3-partition"
                else:
                    check(n in clique_certs, f"case certificates of {n}")
                    row["method"] = "Brouwer 1981 Thm 2 + w3(G) > k (case certificates, integer checker) + 4-partition"
                notes.append(f"lower bound: {r['lb_argument']}; upper bound: {r['ub_certificate']}")
                if n in clique_certs:
                    notes.append(f"w3(G) > k: {clique_certs[n]} part-size cases certified by branch-and-bound trees, "
                                 f"checked by scripts/check_clique_certificates.py (clique/certificates/{n}.cert.jsonl.gz)")
            elif fam == "network":
                r = net[n]
                val = num(r["value"])
                check(r["certified"] == "true", f"network {n}")
                cert(f"network/{n}.res.json")
                row["certificate_class"] = "exact"
                row["method"] = "every attack enumerated, follower by exact tree DP (bobi fire)"
                notes.append(f"{int(r['attacks']):,} attacks, {r['optimal_attacks']} optimal; "
                             f"re-derived by the independent review (network/review.tsv)")
            elif fam == "multidim-knapsack":
                r = orc[n]
                val = num(r["our_point"])
                cert(f"or/{n}.res.json")
                row["certificate_class"] = "numerical"
                row["method"] = "follower LP at the bracket minimiser, follower solved exactly (bobi upper-multi)"
                notes.append(f"point meets the published branch-and-cut bound {fmt(val)}; the published lower bound "
                             f"{fmt(num(r['lit_lb']))} comes from runs its authors flag as numerically inconsistent")
            else:  # general: point meets the MIPLIB optimum of the HPR
                r = hpr[n]
                val = num(r["optimum"])
                row["certificate_class"] = "numerical"  # MIPLIB's published floating-point optimum
                row["method"] = "point meets the MIPLIB optimum of the HPR (HPR = MIPLIB model, scripts/compare_hpr.py)"
                m = re.search(r"\(results/(.*)\)", r["point_from"])
                if m:
                    cert(m.group(1))
                else:
                    cert("general/hpr_optimal.csv", True)
                    notes.append("point: BOBILib's own solution file (passes bobi verify --generic)")
                notes.append(f"lower bound: MIPLIB optimum of {r['miplib_instance']} ({r['miplib_optimum']})")
            row["primal_value"] = row["dual_bound"] = val
            if row["certificate"].endswith(".res.json"):
                fobj = objs[row["certificate"]][1]
                check(abs(fobj - val) <= 1e-9 * max(1, abs(val)), f"{n}: file objective {fobj} != {val}")
            if bval is None:
                notes.append("BOBILib lists no point")
            elif bval > val + 1e-6 * max(1, abs(val)):
                notes.append(f"BOBILib value {fmt(bval)} was not optimal")
            else:
                notes.append("BOBILib value already optimal; the proof is new")
        elif outcome == "no bilevel feasible point":
            d = rays[n]
            rel = f"rays/{n}.exact.ray.json" if os.path.exists(f"{R}/rays/{n}.exact.ray.json") else f"rays/{n}.ray.json"
            row.update(new_status="infeasible_bilevel", source="this work",
                       method="recession direction of the follower (Meyer 1974)")
            cert(rel, d["verified"])
            row["certificate_class"] = "exact" if d["verified"] else "numerical"
            if d["verified"]:
                notes.append(f"f^T d = {d['objective_along_d']:g} on {d['support']} "
                             f"variable{'s' if d['support'] != 1 else ''}, checked in exact "
                             f"rational arithmetic (scripts/verify_ray.py)")
            else:
                notes.append(f"f^T d = {d['objective_along_d']:g} on {d['support']} variables; floating point only: "
                             f"the exact check leaves residuals of about 1e-12 relative")
        elif outcome == "feasible":
            row.update(new_status="feasible", source="this work")
            if fam == "multidim-knapsack":
                r = orc[n]
                obj, rel = own_file("or/")
                check(num(r["our_point"]) == obj, f"OR point {n}")
                cert(rel)
                # one follower knapsack: integer data, exact knapsack oracle; several: HiGHS's dual bound
                row["certificate_class"] = "exact" if r["variant"] == "100" else "numerical"
                row["method"] = "follower LP at the bracket minimiser, follower solved exactly (bobi upper-multi)"
                lu = num(r["lit_ub"])
                if lu is None:
                    notes.append("first point anywhere")
                else:
                    src = or_pub_src.get(n)
                    if src is None:  # without --mkip-lit: keep the source recorded before
                        old = [x for x in prev.get(n, {}).get("notes", "").split("; ") if x.startswith("published point")]
                        m = re.search(r"\((.*)\)", old[0]) if old else None
                        src = m.group(1) if m else "published"
                    notes.append(f"published point {fmt(lu)} ({src}); BOBILib lists none")
            elif n in gpts and gpts[n]["status"] == "point":
                obj, rel = own_file("general/")
                cert(rel)
                row["certificate_class"] = tight_class(rel)
                tightened_note(gpts[n], "general")
                row["method"] = "HPR, follower, optimistic tie-break (bobi general)"
                if n in gen_pub:
                    pv, who = gen_pub[n]
                    cmp = "better than" if pv < obj else "equal to" if pv == obj else "worse than"
                    notes.append(f"verified point; BOBILib lists none; published point {fmt(pv)} ({who}), "
                                 f"{cmp} this one")
                else:
                    notes.append("first point anywhere; instance without coupling rows")
            else:
                r = cpl[n]
                obj, rel = own_file("general-coupled/")
                cert(rel)
                row["certificate_class"] = tight_class(rel)
                tightened_note(r, "general-coupled")
                row["method"] = f"bobi coupled, method {r['method']}"
                if n in gen_pub:
                    pv, who = gen_pub[n]
                    cmp = "better than" if pv < obj else "equal to" if pv == obj else "worse than"
                    notes.append(f"verified point; BOBILib lists none; published point {fmt(pv)} ({who}), "
                                 f"{cmp} this one; instance with coupling rows")
                else:
                    notes.append("first point anywhere; instance with coupling rows")
                if num(r["rel_error"]) and num(r["rel_error"]) > 1e-8:
                    notes.append(f"follower value {num(r['rel_error']):.1e} relative above its optimum "
                                 f"(within BOBILib's tolerance 1e-4)")
            row["primal_value"] = obj
            if fam == "multidim-knapsack" and num(orc[n]["best_lb"]) is not None and num(orc[n]["lit_lb"]):
                row["dual_bound"] = num(orc[n]["best_lb"])
                notes.append("dual bound published by Lozano, Bergman and Cire (2026)")
        else:
            row["new_status"] = "unchanged"
            row["primal_value"] = bval
            row["source"] = BOBILIB_KEY if bval is not None else ""
            obj, rel = own_file()
            if rel and (bval is None or obj < bval - 1e-6 * max(1, abs(bval))):
                row["primal_value"] = obj
                row["source"] = "this work"
                cert(rel)
                notes.append("verified point better than BOBILib's" if bval is not None else "verified point")
            if fam == "multidim-knapsack":
                r = orc[n]
                if rel:
                    row["method"] = ("dualised follower LP bracket, merged-row online bound, local search (bobi bounds)"
                                     if r["variant"] == "100" else
                                     "follower LP at the bracket minimiser, follower solved exactly (bobi upper-multi)")
                ll, lu, ol = num(r["lit_lb"]), num(r["lit_ub"]), num(r["our_lb"])
                lb = max([x for x in (ll, ol) if x is not None], default=None)
                row["dual_bound"] = lb
                if ol is not None and (ll is None or ol > ll + 1e-6):
                    notes.append(f"lower bound improved over the published {fmt(ll)}")
                elif ll is not None:
                    notes.append("dual bound published by Lozano, Bergman and Cire (2026)")
                if lu is not None and (row["primal_value"] is None or lu < row["primal_value"] - 1e-6):
                    notes.append(f"published point {fmt(lu)} (not verified here)")
                if ll is not None and lu is not None and ll > lu:
                    notes.append(f"published lower bound {fmt(ll)} exceeds published point {fmt(lu)}: inconsistent sources")
            elif fam == "clique":
                r = clq[n]
                row["dual_bound"] = num(r["lower_bound"])
                row["method"] = "Mantel's theorem (lower bound) + 4-partition (upper bound)"
                notes.append(f"open: {r['lower_bound']} <= value <= {r['upper_bound']}; {r['ub_certificate']}")
            elif n in gpts:
                g = gpts[n]
                why = g["why"] or f"point failed the final check (max violation {g['max_violation']})"
                notes.append(f"bobi general: {why}")
            elif n in cpl:
                c = cpl[n]
                notes.append("bobi coupled: " + ("no point within the time limit" if c["status"] == "none"
                                                 else "not attempted (batch budget spent)"))
        if n in EXTRA_NOTES:
            notes.append(EXTRA_NOTES[n])
        if n in or_ls_timelimit:
            notes.append(f"Lefebvre and Schmidt's (2025) run files also hold the value {fmt(or_ls_timelimit[n])} from a "
                         f"run stopped at the time limit; their own analysis does not count such rows, and it is not "
                         f"used as a published point here (literature/mkip_literature_status.csv, "
                         f"column lefebvre_schmidt_ub_timelimit)")
        if n in ls_notes and "Lefebvre and Schmidt" not in EXTRA_NOTES.get(n, ""):
            notes.append(f"Lefebvre and Schmidt's (2025) run files: {', '.join(ls_notes[n])} "
                         f"(notes/lefebvre_schmidt_discrepancies.tsv)")
        check((row["certificate_class"] == "na") == (row["new_status"] not in ("optimal", "infeasible_bilevel", "feasible")),
              f"certificate class of {n}")
        rows.append(row)

    fam_rank = {f: i for i, f in enumerate(FAMILY_ORDER)}
    rows.sort(key=lambda r: (fam_rank[r["family"]], r["collection"], natural(r["instance"])))

    # ---- assertions (paper spec) ----------------------------------------------------------
    C = collections.Counter
    st = C(r["bobilib_status"] for r in rows)
    ns = C(r["new_status"] for r in rows)
    unsolved = [r for r in rows if r["bobilib_status"] in ("feasible", "open")]
    lit_rows = [r for r in unsolved if r["new_status"] == "optimal" and r["source"] != "this work"]
    new_opt = [r for r in unsolved if r["new_status"] == "optimal" and r["source"] == "this work"]
    got = dict(instances=len(rows), bobilib_optimal=st["optimal"], bobilib_infeasible=st["infeasible"],
               bobilib_feasible=st["feasible"], bobilib_open=st["open"], unsolved=len(unsolved),
               literature=len(lit_rows), new_optimal=len(new_opt), infeasible_bilevel=ns["infeasible_bilevel"],
               first_point=ns["feasible"], unchanged=ns["unchanged"],
               settled=len(lit_rows) + len(new_opt) + ns["infeasible_bilevel"], optimal_now=ns["optimal"],
               packing=sum(1 for r in unsolved if r["family"] in ("knapsack", "multidim-knapsack")))
    for k, v in EXPECT.items():
        check(got[k] == v, f"{k}: {got[k]} != {v}")
    check(ns["bobilib_optimal"] == 1027 and ns["bobilib_infeasible"] == 33, "BOBILib statuses carried over")
    check(round(100 * got["settled"] / got["unsolved"]) == 41, "settled share 41%")
    by = C((r["family"], r["new_status"], r["source"]) for r in unsolved)
    want = {
        ("knapsack", "optimal", "WeningerFukasawa2025"): 376, ("knapsack", "optimal", "this work"): 16,
        ("multidim-knapsack", "optimal", "LozanoBergmanCire2026"): 88,
        ("multidim-knapsack", "optimal", "FischettiEtAl2019"): 52,
        ("multidim-knapsack", "optimal", "this work"): 0, ("multidim-knapsack", "feasible", "this work"): 140,
        ("clique", "optimal", "this work"): 57, ("network", "optimal", "this work"): 2,
        ("generalized", "optimal", "LozanoBergmanCire2026"): 4,
    }
    for k, v in want.items():
        check(by[k] == v, f"{k}: {by[k]} != {v}")
    fam_unch = C(r["family"] for r in unsolved if r["new_status"] == "unchanged")
    check(fam_unch["multidim-knapsack"] == 582 and fam_unch["clique"] == 3 and fam_unch["generalized"] == 8
          and fam_unch["general-pure"] + fam_unch["general-mixed"] == 123, f"unchanged per family {fam_unch}")
    gen = [r for r in unsolved if r["family"].startswith("general-") and r["bobilib_status"] != "infeasible"]
    gc = C((r["new_status"], r["source"] == "this work") for r in gen)
    check(len(gen) == 266 and gc[("optimal", False)] == 8 and gc[("optimal", True)] == len(hpr) - 1
          and gc[("infeasible_bilevel", True)] == 40 and gc[("feasible", True)] == 88, f"general outcomes {gc}")
    check(sum(1 for r in rows if r["family"] == "general-pure" and r["bobilib_status"] != "optimal") == 131
          and sum(1 for r in rows if r["family"] == "general-mixed" and r["bobilib_status"] != "optimal") == 168,
          "general pure 131 / mixed 168")
    ray_rows = [r for r in rows if r["new_status"] == "infeasible_bilevel"]
    check(all(r["verified"] == "yes" and r["certificate_class"] == "exact" for r in ray_rows),
          "every recession direction checked in exact arithmetic")
    check(all(r["certificate"].endswith(".exact.ray.json") for r in ray_rows if r["instance"].startswith("supportcase42")),
          "supportcase42: the exact directions")
    first = C(("OR" if r["family"] == "multidim-knapsack" else
               "general coupled" if r["certificate"].startswith("general-coupled/") else "general")
              + (" first anywhere" if "first point anywhere" in "; ".join(r["notes"]) else " published point")
              for r in rows if r["new_status"] == "feasible")
    check(first == {"OR first anywhere": 88, "OR published point": 52, "general first anywhere": 52,
                    "general published point": 8, "general coupled first anywhere": 27,
                    "general coupled published point": 1}, f"first points {first}")
    got["first_anywhere"] = sum(v for k, v in first.items() if k.endswith("first anywhere"))
    # Certificate classes (appendix, Definition of certificate classes): every row with a result has one.
    cls = C((r["new_status"], r["certificate_class"]) for r in rows)
    for c in CLASSES:
        got[f"class_{c}"] = sum(v for (_, k), v in cls.items() if k == c)
    check(sum(got[f"class_{c}"] for c in CLASSES) == got["settled"] + got["first_point"], f"classes {cls}")
    check(all(r["bobilib_status"] in ("feasible", "open") for r in rows if r["certificate_class"] != "na"),
          "classes only on rows that BOBILib does not list as optimal or infeasible")
    exact_parts = (sum(1 for r in new_opt if r["family"] in ("knapsack", "clique", "network"))
                   + ns["infeasible_bilevel"]
                   + sum(1 for r in rows if r["new_status"] == "feasible" and r["family"] == "multidim-knapsack"
                         and orc[r["instance"]]["variant"] == "100"))
    check(got["class_exact"] == exact_parts, f"exact class {got['class_exact']} != {exact_parts}")
    check(all(r["certificate_class"] == "numerical" for r in lit_rows), "published optima: numerical")
    got["class_by_status"] = {f"{st_} {c}": v for (st_, c), v in sorted(cls.items()) if c != "na"}

    def kind(r):
        if r["new_status"] == "infeasible_bilevel":
            return "no bilevel feasible point (recession direction)"
        if r["new_status"] == "optimal" and r["source"] != "this work":
            return "published optimum"
        if r["new_status"] == "optimal":
            return {"knapsack": "knapsack interdiction optimum (subset-sum search)",
                    "clique": "clique interdiction optimum", "network": "attack-protect optimum",
                    "multidim-knapsack": "OR optimum"}.get(r["family"], "optimum at the MIPLIB bound of the HPR")
        if r["family"] == "multidim-knapsack":
            return ("OR point, one follower knapsack" if orc[r["instance"]]["variant"] == "100"
                    else "OR point, several follower knapsacks")
        return ("point with coupling rows" if r["certificate"].startswith("general-coupled/")
                else "point without coupling rows")
    got["class_kinds"] = sorted(C((kind(r), r["certificate_class"]) for r in rows if r["certificate_class"] != "na").items(),
                                key=lambda kv: (CLASSES.index(kv[0][1]), -kv[1]))
    got["lit_no_file"] = sum(1 for r in lit_rows if r["verified"] == "na")
    got["or_inconsistent"] = sum(1 for r in orc.values() if num(r["lit_lb"]) is not None and num(r["lit_ub"]) is not None
                                 and num(r["lit_lb"]) > num(r["lit_ub"]))
    check(sum(1 for rel in objs if rel.startswith("or/")) == 690, "690 OR files")
    check(sum(1 for rel in objs if rel.startswith("general/")) == 64, "64 general files")
    check(sum(1 for rel in objs if rel.startswith("general-coupled/")) == 29, "29 coupled files")
    check(all(verified[rel] for rel in objs), "every solution file verified")
    for r in rows:
        if r["certificate"].endswith(".res.json") and r["new_status"] in ("optimal", "feasible", "bobilib_optimal"):
            obj = objs[r["certificate"]][1]
            check(abs(obj - r["primal_value"]) <= 1e-9 * max(1, abs(obj)), f"certificate value of {r['instance']}")
        if r["primal_value"] is not None and r["dual_bound"] is not None:
            check(r["dual_bound"] <= r["primal_value"] + 1e-6 * max(1, abs(r["primal_value"])),
                  f"dual bound above primal for {r['instance']}")

    # ---- instances.csv ---------------------------------------------------------------------
    cols = ["instance", "collection", "family", "bobilib_status", "bobilib_value", "new_status", "primal_value",
            "dual_bound", "source", "method", "certificate", "verified", "certificate_class", "notes"]
    buf = io.StringIO()
    w = csv.DictWriter(buf, fieldnames=cols, lineterminator="\n")
    w.writeheader()
    for r in rows:
        out = dict(r)
        for k in ("bobilib_value", "primal_value", "dual_bound"):
            out[k] = fmt(r[k])
        out["notes"] = "; ".join(r["notes"])
        w.writerow(out)
    write(f"{R}/instances.csv", buf.getvalue())

    # ---- bobilib_update.solu ---------------------------------------------------------------
    lines, tally = [], C()
    for r in sorted(rows, key=lambda r: natural(r["instance"])):
        n, bval, pv = r["instance"], r["bobilib_value"], r["primal_value"]
        if r["new_status"] == "optimal":
            lines.append(f"=opt=      {n}  {fmt(pv)}")
            tally["opt"] += 1
            tally["opt " + ("this work" if r["source"] == "this work" else "literature")] += 1
            continue
        if r["new_status"] == "infeasible_bilevel":
            lines.append(f"=inf=      {n}")
            tally["inf"] += 1
            continue
        if r["new_status"] in ("feasible", "unchanged") and r["source"] == "this work" and pv is not None:
            lines.append(f"=best=     {n}  {fmt(pv)}")
            tally["best"] += 1
        if r["new_status"] in ("feasible", "unchanged") and r["dual_bound"] is not None and (
                "lower bound improved over the published" in "; ".join(r["notes"]) or r["family"] == "clique"):
            lines.append(f"=bestdual= {n}  {fmt(r['dual_bound'])}")
            tally["bestdual"] += 1
            tally["bestdual " + r["family"]] += 1
    check(tally["opt"] == EXPECT["optimal_now"] and tally["inf"] == EXPECT["infeasible_bilevel"], f"solu tally {tally}")
    check(tally["opt literature"] == EXPECT["literature"] and tally["opt this work"] == EXPECT["new_optimal"],
          "solu optimal split")
    lit_counts = C(r["source"] for r in lit_rows)
    header = [
        "# bobilib_update.solu: status update for BOBILib (https://bobilib.org) in the MIPLIB/SCIP .solu format.",
        "# Generated by paper/scripts/build_data_package.py together with results/instances.csv; do not edit by hand.",
        f"# Reference: BOBILib metadata (collection.json) snapshot of {SNAPSHOT}.",
        "#",
        "# Sense: BOBILib minimises the upper-level (leader) objective c^T x + d^T y under the optimistic",
        "# convention; every VALUE is that objective. A dual bound is a lower bound on the bilevel optimum.",
        "#",
        "# One line per instance whose status differs from BOBILib's metadata, or whose best value improves:",
        "#   =opt=      NAME VALUE   proven optimal value (published, or proven in this work)",
        "#   =inf=      NAME         no bilevel feasible point exists (the follower's problem is infeasible or",
        "#                           unbounded for every leader choice; certificate: a recession direction)",
        "#   =best=     NAME VALUE   best verified bilevel feasible point (solution file in this package), not proven",
        "#   =bestdual= NAME VALUE   best proven lower bound, where this work proves a better one than any published",
        "# An instance can have both a =best= and a =bestdual= line. Values are printed with full precision",
        "# (integers without a decimal point, other values as the shortest decimal that round-trips a double).",
        "#",
        f"# Counts: =opt= {tally['opt']} ({tally['opt literature']} published, {tally['opt this work']} this work), "
        f"=inf= {tally['inf']}, =best= {tally['best']}, =bestdual= {tally['bestdual']}.",
        "# Sources of the published optima (instances.csv, column 'source'; full references in README.md):",
    ]
    for key, cnt in sorted(lit_counts.items(), key=lambda kv: -kv[1]):
        header.append(f"#   {key}: {cnt}")
    header += [
        "#   this work: every other line; the certificate of each is named in instances.csv (column 'certificate').",
        "#",
    ]
    float_rays = sorted(r["instance"] for r in ray_rows if r["verified"] != "yes")
    header += ([f"# Of the =inf= lines, {len(ray_rows) - len(float_rays)} rest on recession directions checked in exact "
                f"rational arithmetic; {', '.join(float_rays)}",
                "# rest on directions checked in floating point only."] if float_rays else
               [f"# Every =inf= line ({len(ray_rows)}) rests on a recession direction checked in exact rational arithmetic."])
    header += ["# instances.csv, column 'certificate_class', gives the certificate class of every =opt= and =inf= line.", "#"]
    write(f"{R}/bobilib_update.solu", "\n".join(header + lines) + "\n")

    # ---- README.md ---------------------------------------------------------------------------
    folder_counts = C(rel.split("/")[0] for rel in objs)
    nrays = len([f for f in os.listdir(f"{R}/rays") if f.endswith(".ray.json") and ".exact." not in f])
    cur_tight = [t for f, d in tight.items() if not f.endswith("/tolerance") for t in d.values()]
    check(len(cur_tight) == len(objs), f"tight verification covers {len(cur_tight)} of {len(objs)} files")
    got["tight_files"], got["tight_pass"] = len(cur_tight), sum(t["class"] == "pass" for t in cur_tight)
    got["tight_superseded"] = sum(len(d) for f, d in tight.items() if f.endswith("/tolerance"))
    got["tight_superseded_bench"] = sum(t["class"] == "benchmark-accepted" for f, d in tight.items()
                                        if f.endswith("/tolerance") for t in d.values())
    got["clique_cert_instances"], got["clique_cert_cases"] = len(clique_certs), sum(clique_certs.values())
    write(f"{R}/README.md", readme(got, tally, lit_counts, folder_counts, nrays, ns))

    # ---- SHA256SUMS --------------------------------------------------------------------------
    sums = []
    for root, dirs, names in os.walk(R):
        dirs[:] = [d for d in dirs if d != "__pycache__"]
        for nme in names:
            p = os.path.join(root, nme)
            rel = os.path.relpath(p, R)
            if rel == "SHA256SUMS":
                continue
            h = hashlib.sha256()
            with open(p, "rb") as f:
                for chunk in iter(lambda: f.read(1 << 20), b""):
                    h.update(chunk)
            sums.append((rel, h.hexdigest()))
    sums.sort()
    write(f"{R}/SHA256SUMS", "".join(f"{h}  {rel}\n" for rel, h in sums))

    print(f"build_data_package: {len(rows)} instances ({mode_meta}; literature values from {', '.join(lit_sources)})")
    print(f"  new status: " + ", ".join(f"{k} {v}" for k, v in sorted(ns.items())))
    print(f"  .solu: =opt= {tally['opt']}, =inf= {tally['inf']}, =best= {tally['best']}, =bestdual= {tally['bestdual']}")
    print(f"  SHA256SUMS: {len(sums)} files")


def write(path, text):
    with open(path, "w", newline="\n") as f:
        f.write(text)


def readme(got, tally, lit_counts, folder_counts, nrays, ns):
    refs = "\n".join(f"* `{k}`: {v}." for k, v in REFERENCES)
    return f"""# BOBILib status update: data package

Solution files, certificates and status tables for the 2,654 instances of the
Bilevel Optimization (Benchmark) Instance Library ([BOBILib](https://bobilib.org)),
accompanying the paper by Brian J. Connolly (Independent Researcher, EMAIL).
Everything here is regenerated by `make data` in `bobilib/paper/` (see Replicating);
this file, `instances.csv`, `bobilib_update.solu` and `SHA256SUMS` are written by
`paper/scripts/build_data_package.py` and should not be edited by hand.

## Cite

To cite this material, please cite both the paper and the data package:

* Paper: B. J. Connolly, TITLE OF THE PAPER, JOURNAL (YEAR), doi:PAPER-DOI (placeholder).
* Data: B. J. Connolly, BOBILib status update: data package (YEAR), doi:DATA-DOI (placeholder).

```bibtex
@misc{{ConnollyBOBILibData,
  author = {{Brian J. Connolly}},
  title  = {{{{BOBILib}} status update: data package}},
  year   = {{YEAR}},
  doi    = {{DATA-DOI}},
  note   = {{Placeholder: fill in when the data DOI is assigned}}
}}
```

## Description

BOBILib lists {got['bobilib_optimal']:,} instances as optimal, {got['bobilib_infeasible']} as infeasible,
{got['bobilib_feasible']:,} as feasible (a point is known, no proof) and {got['bobilib_open']} as open
(no point known), from two general-purpose bilevel solvers run for one hour each
(metadata snapshot of {SNAPSHOT}). Of the {got['unsolved']:,} feasible or open instances:

| Outcome | Instances |
| --- | ---: |
| already optimal in published work | {got['literature']} |
| proven optimal here | {got['new_optimal']} |
| no bilevel feasible point (proven here) | {got['infeasible_bilevel']} |
| verified point here, where BOBILib has none | {got['first_point']} |
| status unchanged | {got['unchanged']} |

So {got['settled']} ({round(100 * got['settled'] / got['unsolved'])}%) are settled, {got['optimal_now']} of them optimal.
Of the {got['first_point']} verified points where BOBILib has none, {got['first_anywhere']} are the first point
anywhere; for the other {got['first_point'] - got['first_anywhere']} a published point existed (named in `notes`).
Every claim is backed by a file in this package (a solution in BOBILib's
format that passes BOBILib-style verification, a recession-direction
certificate, a partition certificate) or by a published optimum (cited); for
{got['lit_no_file']} of the {got['literature']} published optima there is no solution file here
(`verified` = `na`), and only the published value is used.

Certificate classes (column `certificate_class`, defined below): of the
{got['settled'] + got['first_point']} rows with a result (optimal, no bilevel feasible point, or a verified point),
{got['class_exact']} are `exact`, {got['class_numerical']} `numerical` and {got['class_benchmark']} `benchmark`.
All {got['tight_files']:,} current solution files pass `bobi verify --tight`
(`notes/tight_verify.md`): the follower's value is within max(1e-6, 1e-9 |phi|) of
its proven optimum, not only within BOBILib's relative 1e-4. The
{got['tight_superseded']} files they replace ({got['tight_superseded_bench']} of them benchmark-accepted points) are kept in
`general/tolerance/` and `general-coupled/tolerance/` for the record.

## Data

```
results/
  README.md             this index
  instances.csv         one row per BOBILib instance ({got['instances']:,} rows), schema below
  bobilib_update.solu   status changes and improved values, MIPLIB/SCIP .solu format
  SHA256SUMS            sha256 of every other file here
  summary.json          per-instance outcome and per-family counts (scripts/summarize.py)
  kip/                  {folder_counts['kip']} knapsack interdiction solutions at published optima (Weninger-Fukasawa)
  ss/                   {folder_counts['ss']} subset-sum knapsack interdiction optima proven here; proofs.csv
  or/                   {folder_counts['or']} verified points for the 810 OR instances; bounds and compare.csv
  clique/               {folder_counts['clique']} clique-interdiction certificates, solver logs, separate review
  network/              {folder_counts['network']} attack-protect trees certified by enumeration; review
  rays/                 {nrays} recession directions: the follower is infeasible or unbounded for every leader choice
  general/              {folder_counts['general']} points for MIPLIB-derived instances without coupling rows; HPR-bound optima
  general-coupled/      {folder_counts['general-coupled']} points for MIPLIB-derived instances with coupling rows
  */tolerance/          superseded points (follower within BOBILib's tolerance only), replaced by tight ones
  literature/           published bounds and optima per instance (inputs of compare.csv and of this index)
  notes/                tight re-verification of every file, Lefebvre and Schmidt's run files checked against
                        this package, BOBILib's general20-30-20-20-20-2 file, instance structure of the general sets
```

Each folder has its own README with the method, the columns of its tables,
the commands that produced it and, where one was done, its separate review.

## File formats

**Solutions (`*.res.json`)** use BOBILib's solution format: a JSON object with
`name`, `bilevel_type` (`optimistic`), `status` (`optimal` or `feasible`),
`difficulty`, `objective_value` (the upper-level objective, which BOBILib
minimises), `upper_level_decisions` and `lower_level_decisions` (variable name
to value). `general/large_points.tar.gz` holds the nine files over 1 MB.

**Recession directions (`rays/*.ray.json`)** map follower variable names to
the entries of a direction d (its support only), as numbers or, in
`*.exact.ray.json`, exact rationals `"p/q"`. The certificate holds if d
satisfies every follower row with leader terms and right-hand sides dropped
(a.d <= 0 for a <= row, >= 0 for a >= row, = 0 for an equation), d_j >= 0 for
a finite lower bound and d_j <= 0 for a finite upper bound, and f^T d < 0
for the follower objective f. Leader variables only shift right-hand sides,
so d is a recession direction for every leader choice; with rational data the
follower's problem is then infeasible or unbounded for every leader choice
(Meyer 1974), so no bilevel feasible point exists.

**Clique partition certificates** are the solution files in `clique/`: the
leader deletes exactly the edges inside the parts of an r-partition of the
vertices (at most k of them), so the remainder is r-partite and its clique
number is at most r; the follower part is a maximum clique of the remainder.
`clique/summary.csv` gives, per instance, n, density, m, k, BOBILib's value,
the lower bound with its argument (Mantel, or Brouwer's Theorem 2 with the
proofs of w3(G) > k) and the upper bound with its partition.

**Clique case certificates (`clique/certificates/*.cert.jsonl.gz`)** prove
w3(G) > k for the {got['clique_cert_instances']} density-0.9 instances: one explicit branch-and-bound
tree per part-size case ({got['clique_cert_cases']} cases), checked from the instance files alone by
`scripts/check_clique_certificates.py` in integer arithmetic (`check.log`; format
and proof in `clique/certificates/README.md`).

**`instances.csv`** (UTF-8, comma separated, one header row):

| Column | Meaning |
| --- | --- |
| `instance` | BOBILib instance name |
| `collection` | BOBILib sub-collection (last directory of the instance path), e.g. `kp`, `or`, `miplib2017` |
| `family` | `knapsack`, `multidim-knapsack`, `clique`, `network`, `generalized`, `assignment` (interdiction), `general-pure`, `general-mixed` |
| `bobilib_status`, `bobilib_value` | BOBILib's status and best known objective (empty: none) |
| `new_status` | `optimal` (proven, published or here), `infeasible_bilevel` (no bilevel feasible point), `feasible` (verified point here; BOBILib had none; `notes` says whether it is the first point anywhere), `unchanged`, `bobilib_optimal`, `bobilib_infeasible` |
| `primal_value` | best upper-level value backed by a verified solution file (this package or BOBILib's) or a published optimum |
| `dual_bound` | best proven lower bound (the upper level is minimised), if any |
| `source` | citation key of the published result (see below), `this work`, or `{BOBILIB_KEY}` (BOBILib's own value) |
| `method` | how the status or value was obtained |
| `certificate` | path, relative to `results/`, of the solution or certificate file; `ARCHIVE.tar.gz:FILE` names an archive member |
| `verified` | `yes`: the certificate passed its check (`bobi verify`, or the exact ray check); `no`: it did not; `na`: no certificate. It says whether a check passed, not how rigorous it is (see `certificate_class`) |
| `certificate_class` | how rigorous the result of the row is (classes (i)-(iii) of the paper's appendix, "Certificate classes"): `exact`, `numerical`, `benchmark` for every row with `new_status` `optimal`, `infeasible_bilevel` or `feasible`; `na` otherwise |
| `notes` | free text, `;`-separated |

Rows per `new_status`: {', '.join(f'{k} {v:,}' for k, v in sorted(ns.items()))}.

**Certificate classes.** A result is in the weakest class of the steps it uses.

* `exact` (class (i)): checked in exact rational or integer arithmetic, or a
  mathematical proof together with an exhaustive integer computation; no
  tolerance enters. Rows: the subset-sum KIP optima (exhaustive integer search,
  re-proved by two integer-only provers), the clique optima (Mantel or
  Brouwer's theorem with the integer-checked case certificates, and the
  partition files), the network optima (every attack enumerated, exact tree
  DP), the recession directions (exact rational check), and OR points with one
  follower knapsack (integer data, exact knapsack oracle).
* `numerical` (class (ii)): rests on a floating-point LP, MILP or CP solver run
  with tolerances, or on a published optimum used as published. Rows: the
  {got['literature']} published optima (not re-certified here), the optima at the MIPLIB
  bound of the HPR (MIPLIB's published floating-point optima; optimal by
  accepted benchmark conventions), OR points with several follower knapsacks
  (follower optimum by HiGHS), and the points for the general instances (rows
  within 1e-6, follower optimum by HiGHS, and the follower's value within
  max(1e-6, 1e-9 |phi|) of the proven bound: they pass `bobi verify --tight`).
* `benchmark` (class (iii)): a point that passes BOBILib's tolerance-based
  checker only because of its tolerances: its follower value is above the
  follower's optimum by more than `bobi verify --tight` allows but within
  BOBILib's relative 1e-4, or its rows hold only within BOBILib's 1e-4. No
  current row is of this class; the superseded files in `*/tolerance/` and
  BOBILib's own file of general20-30-20-20-20-2 (`notes/`) are examples.

Rows per class and kind of result:

| Kind of result | Class | Rows |
| --- | --- | ---: |
{chr(10).join(f"| {k} | `{c}` | {v:,} |" for (k, c), v in got['class_kinds'])}

Other tables (columns in each folder's README): `kip/summary.csv` (published KIP
optima and the leader-vector source), `ss/proofs.csv` (optimum, nodes, CPU
seconds, independent-prover nodes), `or/compare.csv` (published lower and upper
bounds, BOBILib's value, ours, gaps), `clique/summary.csv`, `network/summary.csv`
and `network/highs_crosscheck.csv` (per-tree enumeration), `general/points.csv`
and `general-coupled/summary.csv` (per-instance outcome of `bobi general` and
`bobi coupled`), `general/hpr_optimal.csv` (points meeting the MIPLIB optimum of
the HPR), `rays/ray.tsv` and `rays/ray_verify.jsonl` (direction search and exact
checks), `*/verify_*.tsv` (`bobi verify` output).

**`bobilib_update.solu`** follows the MIPLIB/SCIP convention, one line per
instance whose status or best value changes: `=opt= NAME VALUE` (proven
optimal), `=inf= NAME` (no bilevel feasible point), `=best= NAME VALUE` (best
verified point, not proven) and `=bestdual= NAME VALUE` (best proven lower
bound, where this work proves a better one than any published: {tally['bestdual multidim-knapsack']} OR
instances with one follower knapsack and the {tally['bestdual clique']} open clique instances). It has
{tally['opt']} `=opt=`, {tally['inf']} `=inf=`, {tally['best']} `=best=` and {tally['bestdual']} `=bestdual=` lines.
Values are upper-level objectives of the minimising leader, at full precision.

Citation keys in `source`:

{refs}

## Verification

From `bobilib/`, with BOBILib's instance files in `data/collection`
(bobilib.org) and the crate built (`cargo build --release`):

```sh
(cd results && sha256sum -c SHA256SUMS)
D=data/collection
./target/release/bobi verify --instances $D results/kip results/ss results/network results/or
./target/release/bobi verify --instances $D --generic results/clique results/general results/general-coupled
mkdir -p /tmp/large && tar xzf results/general/large_points.tar.gz -C /tmp/large
./target/release/bobi verify --instances $D --generic /tmp/large
./target/release/bobi verify --instances $D --tight --time-limit 1200 -j 2 results/general /tmp/large   # any folder
python3 -I scripts/check_clique_certificates.py $D/interdiction/clique-interdiction/clique results/clique/certificates
python3 -I scripts/verify_ray.py INSTANCE.mps.gz INSTANCE.aux results/rays/INSTANCE.ray.json
python3 -I scripts/ss_check.py INSTANCE.mps.gz INSTANCE.aux - OPTIMUM   # second KIP lower-bound prover
python3 -I scripts/compare_hpr.py INSTANCE.mps.gz INSTANCE.aux MIPLIB.mps.gz   # HPR = MIPLIB model
```

`bobi verify` checks a file the way BOBILib's feasibility checker does: rows,
bounds and integrality within 1e-6, the stated objective, and the follower
re-solved with the leader fixed (exact knapsack oracle for one follower row,
a HiGHS MILP otherwise) and found optimal within BOBILib's relative 1e-4.
`bobi verify --tight` holds the follower to max(1e-6, 1e-9 |phi|) of its
proven optimum instead; all {got['tight_files']:,} current files pass it
(`notes/tight_verify.md`, `notes/tight_verify.tsv`). `bobi verify` is
separate from BOBILib's solvers but not from this work's code: it is part
of the same crate as the solvers and shares their instance reader and
knapsack oracle. Code that shares nothing with the solvers re-checks the
subset-sum lower bounds (`scripts/ss_check.py` and a second prover), the
recession directions (`scripts/verify_ray.py`), the HPR comparisons
(`scripts/compare_hpr.py`), the clique case certificates
(`scripts/check_clique_certificates.py`) and the re-derived tight general
and coupled points where its solves finished in time
(`scripts/coupled_tight_check.py`, HiGHS through highspy,
`*/tolerance/pycheck_tight.tsv`); the clique,
network and coupled results were also re-derived with separate code in
separate review sessions (see the folder READMEs). The software, the
experiments and these reviews were carried out with an AI coding agent
directed by the author; the reviews ran in separate sessions that wrote
their own code. Caveats: the optima at the MIPLIB bound rest on MIPLIB's
published floating-point optima; the OR lower bounds from the LP bracket rest
on HiGHS's dual bound (with a safety margin of 1e-6 relative); OR lower bounds
for several follower knapsacks are the literature's; the published OR points
include values of Lefebvre and Schmidt (2025) that are not verified, taken
only from runs that ended before the time limit, as in their own analysis
(their run files are checked against this package in
`notes/lefebvre_schmidt.md`); for {'one' if got['or_inconsistent'] == 1 else got['or_inconsistent']} OR
instance{'s' if got['or_inconsistent'] != 1 else ''} a published lower bound exceeds a published point
(`notes` of `instances.csv`{'; OR10x100-0.75_4.dat-0, closed by the published branch-and-cut bound' if got['or_inconsistent'] == 1 else ''}).

## Replicating

```sh
cd bobilib/paper
make data META=/path/to/collection.json   # this package, generated/facts.tex, appendix tables
make all                                  # also figures, paper and appendix
```

`make data` runs `scripts/build_data_package.py` (add `MKIP_LIT=`, `MIBLP_LIT=`
and `LBC_OPTIMAL=` to re-read the published optima from the literature tables;
without them, and without `META=`, they are read back from `instances.csv`),
then `scripts/gen_facts.py` and `scripts/gen_appendix_tables.py`.
`summary.json` is written by `python3 -I scripts/summarize.py --meta collection.json
--mkip-lit results/literature/mkip_literature_status.csv --miblp-lit
results/literature/miblp_literature_status.csv --lbc-optimal optimal.csv
--runs runs --results results` (from `bobilib/`; `optimal.csv` is
`solution_files/optimal.csv` of Lozano, Bergman and Cire's repository; it reads
the OR bounds and ray checks from this package and the `bobi general` runs from `runs/`).
`notes/instance_structure.csv` (the structure of the general and assignment
instances that the paper describes: variable and row split, coupling rows,
continuous linking variables, follower objective against the leader's) is
read from BOBILib's instance files by
`python3 -I scripts/instance_structure.py --meta collection.json --collection data/collection
--out results/notes/instance_structure.csv` (run from `bobilib/`; about 4 minutes on 2 threads). The
computations behind each folder are listed in its README and in
`bobilib/REPORT.md`; times were measured on a shared, often oversubscribed
4-core machine and are indicative only.

## License

LICENSE PLACEHOLDER (to be chosen by the author). BOBILib's instances and
the published results cited above remain under their own terms.

## Contact

Brian J. Connolly, EMAIL (placeholder).
"""


if __name__ == "__main__":
    main()
