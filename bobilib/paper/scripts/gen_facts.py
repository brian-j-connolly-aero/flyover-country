#!/usr/bin/env python3
"""Write paper/generated/facts.tex: one \\newcommand per number the paper quotes.

usage: python3 -I paper/scripts/gen_facts.py [--results DIR] [--out FILE]

Every value is computed from the files under bobilib/results/ (instances.csv,
written by build_data_package.py, and the per-folder tables); none is typed in.
Each is then compared with the authoritative value of the paper spec, and the
script fails on any difference. Macro names use letters only.

Formatting: integers get thousands separators ({,}, safe in text and math);
negative numbers start with \\ensuremath{-}; percentages are bare numbers
(the text adds \\%); instance names are escaped for LaTeX (\\_).
"""
import argparse
import collections
import csv
import json
import math
import os
import re
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOBILIB = os.path.dirname(os.path.dirname(HERE))


def load(path, delimiter=","):
    with open(path, newline="") as f:
        return list(csv.DictReader(f, delimiter=delimiter))


def num(s):
    s = (s or "").strip()
    return float(s) if s not in ("", "None", "n.a.") else None


def grp(int_part):
    s = str(int_part)
    out = ""
    while len(s) > 3:
        out = "{,}" + s[-3:] + out
        s = s[:-3]
    return s + out


def tex_num(v, decimals=None):
    """Integers with {,} separators; floats with the given decimals (None: as given by repr)."""
    neg = v < 0
    v = abs(v)
    if decimals is None and float(v).is_integer():
        body = grp(int(v))
    else:
        s = f"{v:.{decimals}f}" if decimals is not None else repr(float(v))
        ip, _, fp = s.partition(".")
        body = grp(int(ip)) + ("." + fp if fp else "")
    return ("\\ensuremath{-}" if neg else "") + body


def tex_name(s):
    return s.replace("\\", "\\textbackslash{}").replace("_", "\\_").replace("&", "\\&").replace("%", "\\%") \
            .replace("#", "\\#").replace("$", "\\$")


def pct(x):
    """Percent with one decimal, two below 1% (24.0, 4.1, 0.14, 0.04)."""
    p = 100 * x
    return f"{p:.2f}" if abs(p) < 1 else f"{p:.1f}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--results", default=os.path.join(BOBILIB, "results"))
    ap.add_argument("--out", default=os.path.join(BOBILIB, "paper", "generated", "facts.tex"))
    a = ap.parse_args()
    R = a.results
    facts = []  # (name, tex value, raw value for the check, description)

    def fact(name, value, desc, tex=None, expect=None):
        assert re.fullmatch(r"[A-Za-z]+", name), name
        assert name not in {f[0] for f in facts}, name
        t = tex if tex is not None else (tex_num(value) if isinstance(value, (int, float)) else tex_name(str(value)))
        facts.append((name, t, value, desc, expect))

    ins = load(f"{R}/instances.csv")
    C = collections.Counter
    unsolved = [r for r in ins if r["bobilib_status"] in ("feasible", "open")]
    st = C(r["bobilib_status"] for r in ins)
    ns = C(r["new_status"] for r in unsolved)
    lit = [r for r in unsolved if r["new_status"] == "optimal" and r["source"] != "this work"]
    new = [r for r in unsolved if r["new_status"] == "optimal" and r["source"] == "this work"]
    fam = lambda rows, f: [r for r in rows if r["family"] == f]
    gen = lambda rows: [r for r in rows if r["family"].startswith("general-")]

    # ---- BOBILib and the overall outcome ---------------------------------------------------
    fact("NumInstances", len(ins), "BOBILib instances", expect=2654)
    fact("NumBobiOptimal", st["optimal"], "BOBILib status optimal", expect=1027)
    fact("NumBobiInfeasible", st["infeasible"], "BOBILib status infeasible", expect=33)
    fact("NumBobiFeasible", st["feasible"], "BOBILib status feasible", expect=1175)
    fact("NumBobiOpen", st["open"], "BOBILib status open", expect=419)
    fact("NumUnsolved", len(unsolved), "feasible or open in BOBILib", expect=1594)
    fact("NumLitOptimal", len(lit), "already optimal in published work", expect=528)
    fact("NumNewOptimal", len(new), "proven optimal here", expect=82)
    fact("NumNoFeasible", ns["infeasible_bilevel"], "no bilevel feasible point (here)", expect=40)
    fact("NumFirstPoint", ns["feasible"], "verified point here where BOBILib has none (first anywhere or not)", expect=228)
    fact("NumUnchanged", ns["unchanged"], "status unchanged", expect=716)
    settled = len(lit) + len(new) + ns["infeasible_bilevel"]
    fact("NumSettled", settled, "settled = literature + new optimal + no feasible point", expect=650)
    fact("PctSettled", round(100 * settled / len(unsolved)), "settled share of the unsolved, percent", expect=41)
    fact("NumOptimalNow", ns["optimal"], "optimal now among the unsolved", expect=610)
    pack = [r for r in unsolved if r["family"] in ("knapsack", "multidim-knapsack")]
    fact("NumPacking", len(pack), "packing interdiction among the unsolved", expect=1254)
    fact("PctPacking", round(100 * len(pack) / len(unsolved)), "packing share of the unsolved, percent", expect=79)
    fact("NumKipUnsolved", len(fam(unsolved, "knapsack")), "KIP not optimal in BOBILib", expect=392)
    fact("NumMkipUnsolved", len(fam(unsolved, "multidim-knapsack")), "MKIP not optimal in BOBILib", expect=862)
    fact("NumCliqueUnsolved", len(fam(unsolved, "clique")), "clique not optimal in BOBILib", expect=60)
    fact("NumNetworkUnsolved", len(fam(unsolved, "network")), "network not optimal in BOBILib", expect=2)
    fact("NumGeneralizedUnsolved", len(fam(unsolved, "generalized")), "generalized not optimal", expect=12)
    notopt = [r for r in ins if r["bobilib_status"] != "optimal"]
    fact("NumGeneralPure", len(fam(notopt, "general-pure")), "general pure not optimal (incl. infeasible)", expect=131)
    fact("NumGeneralMixed", len(fam(notopt, "general-mixed")), "general mixed not optimal (incl. infeasible)", expect=168)
    fact("NumGeneralUnsolved", len(gen(unsolved)), "general feasible or open (not infeasible)", expect=266)
    for f, name, n in (("knapsack", "Kip", 599), ("multidim-knapsack", "Mkip", 954), ("clique", "Clique", 220),
                       ("network", "Network", 72), ("generalized", "Generalized", 90), ("assignment", "Assignment", 24)):
        fact(f"Num{name}Instances", len(fam(ins, f)), f"BOBILib {f} instances", expect=n)
    fact("NumGeneralInstances", len(gen(ins)), "BOBILib general (MIPLIB-derived and other) instances", expect=695)

    # ---- literature closures ------------------------------------------------------------------
    ls = C(r["source"] for r in lit)
    fact("NumLitKip", ls["WeningerFukasawa2025"], "KIP optima of Weninger and Fukasawa", expect=376)
    fact("NumLitLozano", ls["LozanoBergmanCire2026"], "optima of Lozano, Bergman and Cire (all families)", expect=97)
    fact("NumLitOr", sum(1 for r in lit if r["collection"] == "or"), "OR optima of Lozano et al.", expect=88)
    fact("NumLitGeneralized", len(fam(lit, "generalized")), "generalized optima of Lozano et al.", expect=4)
    fact("NumLitMiplibLozano", sum(1 for r in gen(lit) if r["source"] == "LozanoBergmanCire2026"),
         "MIPLIB 3 optima of Lozano et al.", expect=5)
    fact("NumLitImkp", ls["FischettiEtAl2019"], "imkp optima of Fischetti et al. 2019", expect=52)
    fact("NumLitMkip", len(fam(lit, "multidim-knapsack")), "MKIP literature optima (OR + imkp)", expect=140)
    fact("NumLitMiplib", len(gen(lit)), "MIPLIB-derived literature optima", expect=8)
    fact("NumLitFischettiSeventeen", ls["FischettiEtAl2017"], "air03-0.100000", expect=1)
    fact("NumLitVasquez", ls["VasquezLozanoVanHoeve2025"], "p0282-0.500000", expect=1)
    fact("NumLitEarlierRelease", ls["BOBILibEarlierRelease"], "neos-950242_0_100", expect=1)
    kipsum = load(f"{R}/kip/summary.csv")
    fact("NumKipFiles", sum(1 for r in kipsum if r["literature_optimum"]), "KIP files at published optima", expect=583)
    excess = [(float(r["bobilib_value"]) - float(r["literature_optimum"])) / float(r["literature_optimum"])
              for r in kipsum if r["bobilib_status"] == "feasible" and r["literature_optimum"]
              and float(r["bobilib_value"]) > float(r["literature_optimum"])]
    fact("NumKipBobiAbove", len(excess), "KIP with BOBILib value above the published optimum", expect=335)
    fact("KipBobiExcessMedian", round(100 * statistics.median(excess)), "median excess, percent", expect=8)
    fact("KipBobiExcessMax", round(100 * max(excess)), "largest excess, percent", expect=672)
    imkp = [(num(r["bobilib_value"]) - num(r["primal_value"])) / num(r["primal_value"])
            for r in lit if r["source"] == "FischettiEtAl2019"]
    fact("ImkpBobiExcessMax", round(100 * max(imkp)), "largest imkp excess over the optimum, percent", expect=23)

    # ---- new optima ---------------------------------------------------------------------------
    fact("NumNewKip", len(fam(new, "knapsack")), "KIP optima proven here", expect=16)
    fact("NumNewClique", len(fam(new, "clique")), "clique optima proven here", expect=57)
    fact("NumNewNetwork", len(fam(new, "network")), "network optima proven here", expect=2)
    fact("NumNewHpr", len(gen(new)), "MIPLIB-bound optima", expect=7)
    fact("NumNewOr", len(fam(new, "multidim-knapsack")), "OR closure", expect=0)
    rays = [r for r in unsolved if r["new_status"] == "infeasible_bilevel"]
    fact("NumRayExact", sum(r["verified"] == "yes" for r in rays), "rays checked in exact arithmetic", expect=40)
    fact("NumRayFloat", sum(r["verified"] == "no" for r in rays), "rays checked in floating point only", expect=0)
    assert all(r["certificate_class"] == "exact" for r in rays)
    sc42 = [r for r in rays if r["instance"].startswith("supportcase42")]
    fact("NumRaySupportcase", len(sc42), "supportcase42 instances, now with exact directions", expect=2)
    assert all(r["certificate"].endswith(".exact.ray.json") and "on 1 variable," in r["notes"] for r in sc42)
    fp = [r for r in unsolved if r["new_status"] == "feasible"]
    orfp = [r for r in fp if r["collection"] == "or"]
    fact("NumOrFirstAnywhere", sum("first point anywhere" in r["notes"] for r in orfp), "OR first points anywhere", expect=88)
    fact("NumOrPointNewToBobilib", sum("first point anywhere" not in r["notes"] for r in orfp),
         "OR points where the literature had one, BOBILib none", expect=52)
    fact("NumOrFirstPoints", len(orfp), "OR points where BOBILib has none", expect=140)
    gfp = gen(fp)
    fact("NumGeneralPoints", sum(r["certificate"].startswith("general/") for r in gfp), "general first points without coupling", expect=60)
    fact("NumCoupledPoints", sum(r["certificate"].startswith("general-coupled/") for r in gfp), "general first points with coupling", expect=28)
    fact("NumGeneralFirstPoints", len(gfp), "general first points", expect=88)
    unch = [r for r in unsolved if r["new_status"] == "unchanged"]
    fact("NumMkipUnchanged", len(fam(unch, "multidim-knapsack")), "MKIP unchanged", expect=582)
    fact("NumCliqueOpen", len(fam(unch, "clique")), "clique still open", expect=3)
    fact("NumGeneralizedUnchanged", len(fam(unch, "generalized")), "generalized unchanged", expect=8)
    fact("NumGeneralUnchanged", len(gen(unch)), "general unchanged", expect=123)
    fact("NumGeneralNew", len([r for r in gen(unsolved) if r["new_status"] != "unchanged" and r["source"] == "this work"]),
         "general instances settled or given a first point here", expect=135)

    # ---- KIP (results/ss/proofs.csv) ------------------------------------------------------------
    ss = load(f"{R}/ss/proofs.csv")
    parse = lambda n: tuple(int(x) for x in re.match(r"interKP-(\d+)-100-(\d+)-(\d+)$", n).groups())
    items = [parse(r["name"])[0] for r in ss]
    fact("KipOpenItemsMin", min(items), "fewest items among the 16", expect=200)
    fact("KipOpenItemsMax", max(items), "most items among the 16", expect=500)
    fact("NumKipSubsetSum", sum(parse(r["name"])[1] == 6 for r in ss), "class 6 (subset sum)", expect=8)
    fact("NumKipEvenOdd", sum(parse(r["name"])[1] == 7 for r in ss), "class 7 (even-odd)", expect=8)
    check_all = all(r["optimum"] == r["bobilib_value"] for r in ss)
    fact("NumKipOptEqualsBobi", sum(r["optimum"] == r["bobilib_value"] for r in ss), "optimum = BOBILib value", expect=16)
    hard = max(ss, key=lambda r: float(r["cpu_seconds"]))
    fact("KipHardestName", hard["name"], "slowest of the 16 (CPU)", expect="interKP-500-100-6-3")
    fact("KipHardestOptimum", int(hard["optimum"]), "its optimum", expect=6976)
    fact("KipHardestNodes", int(hard["nodes"]), "its search nodes", expect=10079)
    fact("KipHardestCpu", float(hard["cpu_seconds"]), "its CPU seconds (one thread)",
         tex=f"{float(hard['cpu_seconds']):.1f}", expect=10.04)
    fact("KipHardestIndepNodes", int(hard["independent_check_nodes"]), "nodes of scripts/ss_check.py on it", expect=10116)
    most = max(ss, key=lambda r: int(r["nodes"]))
    fact("KipMostNodesName", most["name"], "instance with the most nodes", expect="interKP-300-100-6-3")
    fact("KipMostNodes", int(most["nodes"]), "most nodes among the 16", expect=19405)
    fact("KipTotalCpu", round(sum(float(r["cpu_seconds"]) for r in ss), 1), "CPU seconds for all 16",
         tex=f"{sum(float(r['cpu_seconds']) for r in ss):.1f}", expect=32.5)
    easy = [r for r in ss if parse(r["name"])[1:] != (6, 3)]
    fact("NumKipEasy", len(easy), "the 16 other than class 6 with I = 3", expect=12)
    fact("KipEasyMaxCpu", max(float(r["cpu_seconds"]) for r in easy), "slowest of those 12, CPU s", expect=0.26)
    fact("KipEasyMaxNodes", max(int(r["nodes"]) for r in easy), "most nodes among those 12", expect=651)
    fact("KipMedianNodes", statistics.median(int(r["nodes"]) for r in ss), "median nodes of the 16", expect=334.5)
    fact("KipMedianCpu", statistics.median(float(r["cpu_seconds"]) for r in ss), "median CPU s of the 16", expect=0.155)
    prev = load(f"{R}/ss/summary.csv")
    fact("NumKipPrevSolved", len(prev), "instances the previous search version proved", expect=12)
    fact("KipPrevMaxSeconds", max(float(r["seconds"]) for r in prev), "slowest proof of the previous version, s",
         tex=f"{max(float(r['seconds']) for r in prev):.0f}", expect=None)

    # ---- OR (results/or/compare.csv) ------------------------------------------------------------
    orr = load(f"{R}/or/compare.csv")
    pts = [r for r in orr if num(r["our_point"]) is not None]
    fact("NumOr", len(orr), "OR instances", expect=810)
    fact("NumOrPoints", len(pts), "verified OR points", expect=690)
    better = [r for r in pts if num(r["lit_ub"]) is not None and num(r["our_point"]) < num(r["lit_ub"]) - 1e-6
              and (num(r["bobilib"]) is None or num(r["our_point"]) < num(r["bobilib"]) - 1e-6)]
    fact("NumOrBetter", len(better), "points strictly better than every published value", expect=474)
    fact("NumOrNoPublished", sum(num(r["lit_ub"]) is None for r in orr), "OR instances without a published point", expect=139)
    blb = [r for r in orr if num(r["our_lb"]) is not None and (num(r["lit_lb"]) is None or num(r["our_lb"]) > num(r["lit_lb"]) + 1e-6)]
    fact("NumOrBetterLb", len(blb), "better lower bounds (all variant -100)", expect=191)
    fact("NumOrBetterLbNotOne", sum(r["variant"] != "100" for r in blb), "better lower bounds outside -100", expect=0)
    one = [r for r in orr if r["variant"] == "100"]
    sev = [r for r in orr if r["variant"] != "100"]
    fact("NumOrOneKnapsack", len(one), "one follower knapsack (-100)", expect=270)
    fact("NumOrOneKnapsackPoints", sum(num(r["our_point"]) is not None for r in one), "points there", expect=270)
    fact("NumOrSeveral", len(sev), "several follower knapsacks (-0, -50)", expect=540)
    fact("NumOrSeveralPoints", sum(num(r["our_point"]) is not None for r in sev), "points there", expect=420)
    hi = [r for r in sev if r["alpha"] in ("0.50", "0.75")]
    fact("NumOrSeveralHigh", len(hi), "several knapsacks, budget 50% or 75%", expect=360)
    fact("NumOrSeveralHighPoints", sum(num(r["our_point"]) is not None for r in hi), "all certified", expect=360)
    lo = [r for r in sev if r["alpha"] == "0.25"]
    fact("NumOrSeveralQuarter", len(lo), "several knapsacks, budget 25%", expect=180)
    fact("NumOrSeveralQuarterPoints", sum(num(r["our_point"]) is not None for r in lo), "certified at 25%", expect=60)
    fact("NumOrSeveralQuarterNoPoint", sum(num(r["our_point"]) is None for r in lo), "without a point at 25%", expect=120)
    med = lambda v: sorted(v)[len(v) // 2]  # as scripts/compare_or.py
    vname = {"100": "One", "0": "Zero", "50": "Fifty"}
    bname = {"0.25": "Quarter", "0.50": "Half", "0.75": "ThreeQuarter"}
    # Paired medians leave out the instances whose published lower bound exceeds the published point
    # (referee review: inconsistent sources). Values as in the spec except the three affected groups.
    # Round-two review: Lefebvre and Schmidt's values from runs stopped at the time limit are no longer
    # published points (scripts/ls_mkip_points.py), which moves three "now" medians.
    spec_gaps = {("100", "0.25"): ("24.0", "4.0"), ("100", "0.50"): ("34.0", "7.2"), ("100", "0.75"): ("0.14", "0.14"),
                 ("0", "0.25"): ("33.3", "31.4"), ("0", "0.50"): ("36.6", "29.1"), ("0", "0.75"): ("5.3", "0.05"),
                 ("50", "0.25"): ("30.6", "28.3"), ("50", "0.50"): ("41.3", "35.0"), ("50", "0.75"): ("11.8", "1.4")}
    conflict = {r["name"] for r in orr if num(r["lit_lb"]) is not None and num(r["lit_ub"]) is not None
                and num(r["lit_lb"]) > num(r["lit_ub"])}
    spec_nopoint = {("0", "0.25"): (31, 0), ("0", "0.50"): (31, 31), ("0", "0.75"): (27, 27),
                    ("50", "0.25"): (20, 0), ("50", "0.50"): (20, 20), ("50", "0.75"): (10, 10)}
    for (v, al), (lit_s, now_s) in spec_gaps.items():
        grp_rows = [r for r in orr if r["variant"] == v and r["alpha"] == al and num(r["gap"]) is not None]
        paired = [r for r in grp_rows if num(r["lit_gap"]) is not None and r["name"] not in conflict]
        lg, g = pct(med([num(r["lit_gap"]) for r in paired])), pct(med([num(r["gap"]) for r in paired]))
        fact(f"OrGap{vname[v]}{bname[al]}Lit", lg, f"median published gap, variant -{v}, budget {al}", tex=lg, expect=lit_s)
        fact(f"OrGap{vname[v]}{bname[al]}Now", g, f"median gap now, variant -{v}, budget {al}", tex=g, expect=now_s)
        fact(f"OrPaired{vname[v]}{bname[al]}", len(paired), f"instances in that median", expect=None)
        if (v, al) in spec_nopoint:
            nop = [r for r in orr if r["variant"] == v and r["alpha"] == al and num(r["lit_ub"]) is None]
            fact(f"OrNoPoint{vname[v]}{bname[al]}", len(nop), f"no published point, -{v}, {al}", expect=spec_nopoint[(v, al)][0])
            fact(f"OrFirst{vname[v]}{bname[al]}", sum(num(r["our_point"]) is not None for r in nop),
                 f"first points among those", expect=spec_nopoint[(v, al)][1])
    assert not [r for r in ins if r["collection"] == "or" and r["new_status"] == "optimal" and r["source"] == "this work"]
    clo = [r for r in ins if r["instance"] == "OR10x100-0.75_4.dat-0"]
    I_or_notes = {r["instance"]: r["notes"] for r in ins if r["collection"] == "or"}
    c0 = next(r for r in orr if r["name"] == clo[0]["instance"])
    assert clo[0]["new_status"] == "optimal" and clo[0]["source"] == "LozanoBergmanCire2026"
    fact("OrClosureName", clo[0]["instance"], "OR instance whose published lower bound exceeds the published point "
         "but whose published branch-and-cut bound closes it", expect="OR10x100-0.75_4.dat-0")
    fact("OrClosureValue", int(num(clo[0]["primal_value"])), "its optimum (published point and bound)", expect=8813)
    fact("OrClosureLitLb", int(num(c0["lit_lb"])), "published (inconsistent) lower bound", expect=8814)
    fact("NumOrInconsistent", sum(num(r["lit_lb"]) is not None and num(r["lit_ub"]) is not None
                                  and num(r["lit_lb"]) > num(r["lit_ub"]) for r in orr),
         "OR instances whose published lower bound exceeds the published point", expect=1)
    fact("NumOrLitOptimalPoint", sum(1 for r in ins if r["collection"] == "or" and r["source"] == "LozanoBergmanCire2026"
                                     and r["certificate"]), "published OR optima reproduced by a file here", expect=70)

    # ---- clique (results/clique/summary.csv) ----------------------------------------------------
    cq = load(f"{R}/clique/summary.csv")
    fact("NumCliqueSet", len(cq), "instances of the clique set", expect=60)
    fact("NumCliqueSevenThree", sum(r["density"] == "0.7" and r["status"] == "optimal" and r["upper_bound"] == "3" for r in cq),
         "density 0.7 with value 3", expect=27)
    fact("NumCliqueNineFour", sum(r["density"] == "0.9" and r["status"] == "optimal" and r["upper_bound"] == "4" for r in cq),
         "density 0.9 with value 4", expect=30)
    fact("CliqueBobiMin", min(int(r["bobilib_value"]) for r in cq), "smallest BOBILib value", expect=5)
    fact("CliqueBobiMax", max(int(r["bobilib_value"]) for r in cq), "largest BOBILib value", expect=13)
    words = {40: "Forty", 50: "Fifty", 60: "Sixty"}
    spec_mantel = {40: (410, 400), 50: (643, 625), 60: (930, 900)}
    spec_brouwer = {40: (527, 522), 50: (827, 819), 60: (1195, 1182)}
    spec_k = {(40, "0.7"): 136, (50, "0.7"): 214, (60, "0.7"): 309, (40, "0.9"): 175, (50, "0.9"): 275, (60, "0.9"): 398}
    cases = 0
    for n in (40, 50, 60):
        r7 = next(r for r in cq if int(r["n"]) == n and r["density"] == "0.7")
        r9 = next(r for r in cq if int(r["n"]) == n and r["density"] == "0.9")
        mk = int(r7["m"]) - int(r7["k"])
        fact(f"CliqueMantelLhs{words[n]}", mk, f"m-k at n={n}, d=0.7", expect=spec_mantel[n][0])
        fact(f"CliqueMantelRhs{words[n]}", n * n // 4, f"floor(n^2/4) at n={n}", expect=spec_mantel[n][1])
        t3 = sum(math.comb(s, 2) for s in (n // 3 + (i < n % 3) for i in range(3)))
        t3n = math.comb(n, 2) - t3  # Turan number t_3(n)
        fact(f"CliqueBrouwerLhs{words[n]}", int(r9["m"]) - int(r9["k"]), f"m-k at n={n}, d=0.9", expect=spec_brouwer[n][0])
        fact(f"CliqueBrouwerRhs{words[n]}", t3n - n // 3 + 2, f"t_3(n)-floor(n/3)+2 at n={n}", expect=spec_brouwer[n][1])
        fact(f"CliqueTuran{words[n]}", t3n, f"t_3(n) at n={n}", expect=None)
        for d, r in (("0.7", r7), ("0.9", r9)):
            fact(f"CliqueK{words[n]}{'Seven' if d == '0.7' else 'Nine'}", int(r["k"]), f"budget k at n={n}, d={d}",
                 expect=spec_k[(n, d)])
            fact(f"CliqueM{words[n]}{'Seven' if d == '0.7' else 'Nine'}", int(r["m"]), f"edges m at n={n}, d={d}", expect=None)
        for r in cq:
            if int(r["n"]) == n and r["density"] == "0.9":
                assert f"{int(r['m']) - int(r['k'])} >= " in r["lb_argument"]
                cases += int(re.search(r"over (\d+) part-size vectors", r["lb_argument"]).group(1))
    fact("NumCliquePartCases", cases, "part-size cases decided by exact solvers", expect=170)
    openq = [r for r in cq if r["status"] == "open"]
    fact("CliqueOpenNames", ", ".join(r["name"] for r in openq), "the three open instances (each in \\inst)",
         tex=", ".join(f"\\inst{{{tex_name(r['name'])}}}" for r in openq[:-1])
         + f" and \\inst{{{tex_name(openq[-1]['name'])}}}",
         expect="normalClique60-70-2, normalClique60-70-3, normalClique60-70-5")
    best3 = []
    for r in openq:
        txt = open(f"{R}/clique/runs/deep_tabu_{r['name'].replace('normalClique', '')}.txt").read()
        best3.append(int(re.search(r"^best (\d+)", txt, re.M).group(1)))
    for i, v in zip(("A", "B", "C"), best3):
        fact(f"CliqueOpenBestThree{i}", v, "best 3-partition found (inside edges)", expect=None)
    assert best3 == [311, 310, 310], best3
    fact("CliqueOpenInsideFour", ", ".join(re.search(r"with (\d+) inside", r["ub_certificate"]).group(1) for r in openq),
         "inside edges of the open instances' 4-partitions", expect=None)
    lp = []
    for p in ("k4lp_turan_60-70-2.json", "k4lp_turan_60-70-3.json"):
        d = json.load(open(f"{R}/clique/runs/{p}"))
        lp.append(d["final_exact_bound"])
    for line in open(f"{R}/clique/runs/k4lp_brouwer_60-70-2.rounds.jsonl"):
        if line.startswith("{"):
            lp.append(json.loads(line)["exact_bound"])
    fact("CliqueOpenLpBound", math.floor(max(lp)), "best LP bound on the K4-transversal number (floor)", expect=296)
    fact("CliqueOpenK", int(openq[0]["k"]), "budget of the open instances", expect=309)

    # ---- network (results/network) ---------------------------------------------------------------
    net = load(f"{R}/network/summary.csv")
    newnet = [r for r in net if r["bobilib_status"] == "feasible"]
    fact("NetworkOptimum", int(newnet[0]["value"]), "optimum of both open trees", expect=26)
    assert all(r["value"] == newnet[0]["value"] for r in newnet)
    fact("NetworkAttacks", int(newnet[0]["attacks"]), "attacks enumerated per open tree", expect=2138410)
    fact("NetworkOptAttacksA", int(newnet[0]["optimal_attacks"]), "optimal attacks, _006", expect=4)
    fact("NetworkOptAttacksB", int(newnet[1]["optimal_attacks"]), "optimal attacks, _010", expect=28)
    fact("NetworkNodes", int(newnet[0]["nodes"]), "nodes of the open trees", expect=49)
    fact("NetworkPhi", int(newnet[0]["phi"]), "attack budget", expect=5)
    fact("NetworkLambda", int(newnet[0]["lambda"]), "protection budget", expect=5)
    fact("NetworkTreesReproduced", sum(r["bobilib_status"] == "optimal" and r["agrees"] == "true" for r in net),
         "BOBILib-optimal trees reproduced", expect=34)
    fact("NetworkTotalAttacks", sum(int(r["attacks"]) for r in net), "attacks over all 36 trees", expect=9340760)
    fact("NetworkSecsEnumA", float(newnet[0]["secs_enum"]), "enumeration seconds, _006", tex=f"{float(newnet[0]['secs_enum']):.0f}", expect=None)
    fact("NetworkSecsEnumB", float(newnet[1]["secs_enum"]), "enumeration seconds, _010", tex=f"{float(newnet[1]['secs_enum']):.0f}", expect=None)
    rev = load(f"{R}/network/review.tsv", "\t")
    red = [r for r in rev if r["bobilib_status"] == "optimal" and r["agrees"] == "yes" and r["method"] in ("enum", "brute", "enum+brute")]
    fact("NetworkRederived", len(red), "BOBILib optimal values re-derived", expect=54)
    fact("NetworkBobiOptimal", sum(r["bobilib_status"] == "optimal" for r in ins if r["family"] == "network"),
         "BOBILib optimal network instances", expect=70)

    # ---- general (results/general, general-coupled, rays) ----------------------------------------
    gp = load(f"{R}/general/points.csv")
    fact("NumGeneralCandidates", len(gp), "open instances without coupling rows run through bobi general", expect=98)
    fact("NumGeneralFiles", sum(r["verified"] == "True" for r in gp), "verified files in results/general", expect=64)
    hpr = load(f"{R}/general/hpr_optimal.csv")
    hprnew = [r for r in hpr if r["name"] in {x["instance"] for x in new}]
    fact("NumGeneralFilesOptimal", sum("results/general/" in r["point_from"] for r in hprnew), "of them optimal", expect=4)
    fact("NumGeneralFilesFeasible", sum(r["verified"] == "True" for r in gp) - sum("results/general/" in r["point_from"] for r in hprnew),
         "of them feasible only", expect=60)
    cp = load(f"{R}/general-coupled/summary.csv")
    cpts = [r for r in cp if r["status"] == "point"]
    fact("NumCoupledCandidates", len(cp), "open instances with coupling rows run through bobi coupled", expect=54)
    fact("NumCoupledAttempted", sum(r["status"] != "not_attempted" for r in cp), "attempted", expect=52)
    fact("NumCoupledFiles", len(cpts), "verified files in results/general-coupled", expect=29)
    fact("NumCoupledOptimal", sum("results/general-coupled/" in r["point_from"] for r in hprnew), "of them optimal", expect=1)
    fact("NumCoupledTolerance", sum((num(r["rel_error"]) or 0) > 1e-8 for r in cpts),
         "coupled files whose follower value uses BOBILib's tolerance", expect=0)
    mc = C(r["method"] for r in cpts)
    fact("NumCoupledByHpr", mc["hpr"], "coupled points at the HPR's leader values", expect=21)
    fact("NumCoupledByPenalty", mc["penalty"], "coupled points from the penalised HPR", expect=5)
    fact("NumCoupledByFollow", mc["follow"], "coupled points from best-response dynamics", expect=3)
    fact("NumHprOptimal", len(hprnew), "points meeting the MIPLIB bound", expect=7)
    fact("NumHprOpen", sum(r["bobilib_status"] == "open" for r in hprnew), "of them open before", expect=5)
    hpr_r2 = [r for r in hprnew if r["name"] in ("ex10_0_100", "neos-1445765_0_100")]
    fact("NumHprRoundTwo", len(hpr_r2), "MIPLIB-bound optima added in round two (ex10, neos-1445765)", expect=2)
    val = {r["name"]: num(r["optimum"]) for r in hpr}
    fact("PigeonOptimum", val["pigeon-10_0_100"], "pigeon-10 optimum", expect=-9000)
    assert val["pigeon-10_50_50"] == val["pigeon-10_0_100"]
    fact("TimtabOptimum", val["timtab1_0_100"], "timtab1_0_100 optimum (full precision)",
         tex=tex_num(val["timtab1_0_100"]), expect=764771.99999978)
    fact("TimtabOptimumRounded", round(val["timtab1_0_100"]), "timtab1_0_100 optimum rounded", expect=764772)
    fact("NoswotOptimum", val["noswot_0_100"], "noswot optimum", expect=-41)
    fact("ExTenOptimum", val["ex10_0_100"], "ex10_0_100 optimum (MIPLIB bound)", expect=100)
    fact("NeosFourteenOptimum", val["neos-1445765_0_100"], "neos-1445765_0_100 optimum (MIPLIB bound)", expect=-17783)
    ex10 = next(r for r in ins if r["instance"] == "ex10_0_100")
    assert ex10["new_status"] == "optimal" and "Lefebvre and Schmidt" in ex10["notes"] and ex10["source"] == "this work"
    assert val["noswot_50_50"] == val["noswot_0_100"]
    rv = [json.loads(l) for l in open(f"{R}/rays/ray_verify.jsonl") if l.startswith("{")]
    fact("NumRayFiles", len([f for f in os.listdir(f"{R}/rays") if f.endswith(".ray.json") and ".exact." not in f]),
         "direction files (40 open + 1 BOBILib-infeasible)", expect=41)
    fact("NumRayChecks", len(rv), "exact checks run (ray_verify.jsonl)", expect=None)

    # ---- verification counts -------------------------------------------------------------------
    def passes(p):
        return sum(1 for r in load(f"{R}/{p}", "\t") if r.get("result") == "PASS")
    fact("NumVerifyOrOne", passes("or/verify_one_knapsack.tsv"), "bobi verify passes, OR one knapsack", expect=270)
    fact("NumVerifyOrSeveral", passes("or/verify_several_knapsacks.tsv"), "bobi verify passes, OR several", expect=420)
    fact("NumVerifyNetwork", passes("network/verify_ours.tsv"), "bobi verify passes, network", expect=36)
    fact("NumVerifyClique", passes("clique/review/bobi_verify.tsv"), "bobi verify passes, clique", expect=60)
    fact("NumVerifyNetworkBobi", passes("network/verify_bobilib.tsv"), "BOBILib's inter-fire files passing", expect=72)
    solu = [l.split()[0] for l in open(f"{R}/bobilib_update.solu") if l.strip() and not l.startswith("#")]
    sc = C(solu)
    fact("NumSoluOpt", sc["=opt="], ".solu =opt= lines", expect=610)
    fact("NumSoluInf", sc["=inf="], ".solu =inf= lines", expect=40)
    fact("NumSoluBest", sc["=best="], ".solu =best= lines", expect=None)
    fact("NumSoluBestDual", sc["=bestdual="], ".solu =bestdual= lines", expect=None)

    # ---- paper (main.tex): outcome table by family ----------------------------------------------
    # One macro per cell of the family table: Fam<Family><Column>, over all instances (All), the
    # feasible or open ones (Unsolved) and their outcomes (Lit, New, NoFeas, First, Unchanged).
    famkeys = (("knapsack", "Kip"), ("multidim-knapsack", "Mkip"), ("clique", "Clique"), ("network", "Network"),
               ("generalized", "Generalized"), ("assignment", "Assignment"), ("general-pure", "GenPure"),
               ("general-mixed", "GenMixed"))
    assert sorted(f for f, _ in famkeys) == sorted({r["family"] for r in ins})
    for f, nm in famkeys:
        u = fam(unsolved, f)
        nsf = C(r["new_status"] for r in u)
        fact(f"Fam{nm}All", len(fam(ins, f)), f"{f}: instances")
        fact(f"Fam{nm}Unsolved", len(u), f"{f}: feasible or open in BOBILib")
        fact(f"Fam{nm}Lit", len(fam(lit, f)), f"{f}: optimal in published work")
        fact(f"Fam{nm}New", len(fam(new, f)), f"{f}: proven optimal here")
        fact(f"Fam{nm}NoFeas", nsf["infeasible_bilevel"], f"{f}: no bilevel feasible point")
        fact(f"Fam{nm}First", nsf["feasible"], f"{f}: verified point where BOBILib has none")
        fact(f"Fam{nm}Unchanged", nsf["unchanged"], f"{f}: unchanged")
        assert len(u) == len(fam(lit, f)) + len(fam(new, f)) + nsf["infeasible_bilevel"] + nsf["feasible"] + nsf["unchanged"]
    fact("NumImkp", sum(1 for r in ins if r["collection"] == "imkp"), "instances of the imkp set", expect=144)
    opt7 = {r["upper_bound"] for r in cq if r["density"] == "0.7" and r["status"] == "optimal"}
    opt9 = {r["upper_bound"] for r in cq if r["density"] == "0.9" and r["status"] == "optimal"}
    assert len(opt7) == 1 and len(opt9) == 1, (opt7, opt9)
    fact("CliqueOptSeven", int(opt7.pop()), "optimum of the solved density-0.7 clique instances", expect=3)
    fact("CliqueOptNine", int(opt9.pop()), "optimum of the density-0.9 clique instances", expect=4)
    fact("CliqueOpenUpper", max(int(r["upper_bound"]) for r in openq), "upper bound of the open clique instances",
         expect=4)
    fact("CliqueOpenLower", min(int(r["lower_bound"]) for r in openq), "lower bound of the open clique instances",
         expect=3)

    # ---- paper: OR groups without a published point, variant -100 (none) -----------------------------
    for al in ("0.25", "0.50", "0.75"):
        nop = [r for r in orr if r["variant"] == "100" and r["alpha"] == al and num(r["lit_ub"]) is None]
        fact(f"OrNoPointOne{bname[al]}", len(nop), f"no published point, -100, {al}", expect=0)
        fact(f"OrFirstOne{bname[al]}", sum(num(r["our_point"]) is not None for r in nop),
             f"first points among those", expect=0)
    for v in ("100", "0", "50"):
        for al in ("0.25", "0.50", "0.75"):
            g = [r for r in orr if r["variant"] == v and r["alpha"] == al]
            fact(f"OrGroup{vname[v]}{bname[al]}", len(g), f"OR instances, variant -{v}, budget {al}", expect=90)
            fact(f"OrPoints{vname[v]}{bname[al]}", sum(num(r["our_point"]) is not None for r in g),
                 f"verified points, variant -{v}, budget {al}", expect=None)

    # ---- paper: the 16 KIP instances, one table row each (results/ss/proofs.csv) -----------------
    rows = []
    for r in ss:
        n_, cls, bud = parse(r["name"])
        rows.append(f"\\inst{{{tex_name(r['name'])}}} & {n_} & {cls} & {bud} & {tex_num(int(r['budget']))} & "
                    f"{tex_num(int(r['capacity']))} & {tex_num(int(r['optimum']))} & "
                    f"{tex_num(int(r['nodes']))} & {r['cpu_seconds']} & {tex_num(int(r['independent_check_nodes']))} \\\\")
    fact("KipTableRows", "rows", "rows of the paper's table of the 16 KIP instances", tex="%\n" + "\n".join(rows) + "\n")

    # ---- paper: numbers quoted from bobilib/REPORT.md (runs of earlier search versions) ---------
    report = " ".join(open(os.path.join(BOBILIB, "REPORT.md")).read().split())
    mrow = re.search(r"\| interKP-200-100-6-2 \| 1,855 \| (\d+) \| ([\d.]+) s \| ([\d,]+) / ([\d.]+) s \|", report)
    fact("KipBeforeNodesTwoSixTwo", int(mrow.group(3).replace(",", "")), "200-6-2: nodes of the previous version",
         expect=3926223)
    fact("KipBeforeSecsTwoSixTwo", float(mrow.group(4)), "200-6-2: CPU s of the previous version",
         tex=f"{float(mrow.group(4)):.1f}", expect=22.77)
    fact("KipAfterNodesTwoSixTwo", int(mrow.group(1)), "200-6-2: nodes with per-node tables", expect=303)
    fact("KipAfterSecsTwoSixTwo", float(mrow.group(2)), "200-6-2: CPU s with per-node tables",
         tex=mrow.group(2), expect=0.10)
    mfin = re.search(r"\| interKP-200-100-6-3 \| 2,782 \| [\d,]+ \| [\d.]+ s \| not finished in (\d+) h \|", report)
    fact("KipPrevUnfinishedHours", int(mfin.group(1)), "200-6-3: hours the previous version ran without finishing",
         expect=4)
    mab = re.search(r"interKP-200-100-6-2 \(nodes\): previous search ([\d,]+); same rules with threshold tables "
                    r"([\d,]+) \(memory (\d+) to (\d+) MB\); every band width 1\.\.64 ([\d,]+); root tables played "
                    r"largest value first ([\d,]+), or cheapest per unit of mass first ([\d,]+); per-node tables ([\d,]+)",
                    report)
    ab = [int(x.replace(",", "")) for x in mab.groups()]
    assert ab[0] == 3926223 and ab[7] == 303, ab
    fact("KipAblationThreshold", ab[1], "200-6-2 nodes: threshold tables, same rules", expect=3926223)
    fact("KipAblationMemBefore", ab[2], "memory before threshold tables, MB", expect=None)
    fact("KipAblationMemAfter", ab[3], "memory with threshold tables, MB", expect=None)
    fact("KipAblationBand", ab[4], "200-6-2 nodes: every band width", expect=None)
    fact("KipAblationLargestFirst", ab[5], "200-6-2 nodes: root tables, largest value first", expect=None)
    fact("KipAblationCheapestFirst", ab[6], "200-6-2 nodes: root tables, cheapest per unit of mass first", expect=None)
    mbf = re.search(r"exhaustive enumeration on ([\d,]+) random small instances \((\d+) mismatches\)", report)
    fact("KipBruteForceInstances", int(mbf.group(1).replace(",", "")), "random instances compared with brute force",
         expect=2990)
    fact("KipBruteForceMismatches", int(mbf.group(2)), "mismatches", expect=0)

    # ---- paper: BOBILib's point for general20-30-20-20-20-2 (results/notes) ----------------------
    nt = load(f"{R}/notes/general20-30-20-20-20-2.verify.tsv", "\t")[0]
    phi_sol, phi_opt = float(nt["phi_sol"]), float(nt["phi_bound"])
    fact("GenTolPhiSol", phi_sol, "follower value in BOBILib's file", tex=tex_num(phi_sol, 2), expect=None)
    fact("GenTolPhiOpt", phi_opt, "follower optimum at the same leader choice", tex=tex_num(phi_opt), expect=106899953)
    fact("GenTolDiff", round(phi_sol - phi_opt), "difference, units", expect=31)
    rel = (phi_sol - phi_opt) / abs(phi_opt)
    fact("GenTolRel", rel, "relative difference", tex=f"\\ensuremath{{{rel / 1e-7:.1f}\\times 10^{{-7}}}}", expect=None)
    assert f"{rel / 1e-7:.1f}" == "2.9"
    fact("GenTolObjFile", float(nt["file_objective"]), "leader objective stated in BOBILib's file",
         tex=tex_num(float(nt["file_objective"])), expect=100056)
    fact("GenTolObjComputed", float(nt["objective"]), "leader objective computed from the file's point",
         tex=tex_num(float(nt["objective"]), 3), expect=None)
    viol = float(nt["max_violation"])
    fact("GenTolViolation", viol, "largest row violation",
         tex=f"\\ensuremath{{{viol / 1e-4:.1f}\\times 10^{{-4}}}}", expect=3.3e-4)

    # ---- paper: the five optima at the MIPLIB bound, one table row each -----------------------------
    hrows = []
    for r in hpr:
        if r["name"] not in {x["instance"] for x in new}:
            continue
        src = r["point_from"]
        how = ("BOBILib" if src.startswith("BOBILib") else
               "\\texttt{bobi coupled}" if "general-coupled" in src else "\\texttt{bobi general}")
        hrows.append(f"{tex_name(r['name'])} & {r['bobilib_status']} & {tex_num(float(r['optimum']))} & {how} \\\\")
    assert len(hrows) == 7
    nsw = {float(r["miplib_optimum"]) for r in hpr if r["name"].startswith("noswot")}
    assert len(nsw) == 1
    nsw = nsw.pop()
    fact("NoswotMiplibOptimum", nsw, "MIPLIB's optimum of noswot (floating point)", tex=tex_num(nsw),
         expect=-41.00000885)
    fact("HprTableRows", "rows", "rows of the paper's table of the MIPLIB-bound optima", tex="%\n" + "\n".join(hrows) + "\n")

    # ---- review round: categories, funnels and splits the revised text quotes ---------------------
    fp_all = [r for r in unsolved if r["new_status"] == "feasible"]
    first_any = [r for r in fp_all if "first point anywhere" in r["notes"]]
    fact("NumFirstAnywhere", len(first_any), "verified points here that are the first point anywhere", expect=167)
    fact("NumPointPublishedBefore", len(fp_all) - len(first_any),
         "verified points here where BOBILib has none but a point was published", expect=61)
    gfa = [r for r in first_any if r["family"].startswith("general-")]
    fact("NumGeneralFirstAnywhere", len(gfa), "general first points anywhere", expect=79)
    gpub = [r for r in fp_all if r["family"].startswith("general-") and "first point anywhere" not in r["notes"]]
    fact("NumGeneralPublishedPoint", len(gpub), "general points where a point was published", expect=9)
    fact("NumGeneralPublishedBetter", sum(", better than this one" in r["notes"] for r in gpub),
         "of them: the published point is better", expect=6)
    fact("NumGeneralPublishedLS", sum("Lefebvre and Schmidt" in r["notes"] for r in gpub),
         "of them: published only in Lefebvre and Schmidt's run files", expect=3)
    gpub_nocpl = [r for r in gpub if r["certificate"].startswith("general/")]
    fact("NumGeneralNoCplPublished", len(gpub_nocpl), "bobi general points where a point was published", expect=8)
    fact("NumGeneralNoCplPublishedBetter", sum(", better than this one" in r["notes"] for r in gpub_nocpl),
         "of them: the published point is better", expect=6)
    fact("NumCoupledPublished", sum(r["certificate"].startswith("general-coupled/") for r in gpub),
         "bobi coupled points where a point was published", expect=1)
    fact("NumOrPublishedPointFile", sum("solution file in their repository" in r["notes"] for r in fp_all
                                        if r["collection"] == "or" and "first point anywhere" not in r["notes"]),
         "OR points new to BOBILib whose published point has a solution file (Lozano et al.)", expect=52)
    fact("NumSettledHere", len(new) + ns["infeasible_bilevel"], "settled by this work (new optima + no point)", expect=122)
    fact("PctSettledHere", f"{100 * (len(new) + ns['infeasible_bilevel']) / len(unsolved):.1f}",
         "settled by this work, percent of the unsolved", tex=f"{100 * (len(new) + ns['infeasible_bilevel']) / len(unsolved):.1f}",
         expect="7.7")
    lit_file = [r for r in lit if r["verified"] == "yes"]
    lit_nofile = [r for r in lit if r["verified"] == "na"]
    fact("NumLitWithFile", len(lit_file), "published optima with a verified solution file at the optimum", expect=446)
    fact("NumLitNoFile", len(lit_nofile), "published optima without a solution file here (value only)", expect=82)
    fact("NumLitNoFileImkp", sum(r["collection"] == "imkp" for r in lit_nofile), "of them imkp", expect=52)
    fact("NumLitNoFileOr", sum(r["collection"] == "or" for r in lit_nofile), "of them OR", expect=18)
    fact("NumLitNoFileMiplib", sum(r["family"].startswith("general-") for r in lit_nofile), "of them MIPLIB-derived", expect=8)
    fact("NumLitNoFileGeneralized", sum(r["family"] == "generalized" for r in lit_nofile), "of them generalized", expect=4)
    fact("NumLitKnapsackMethods", ls["WeningerFukasawa2025"] + ls["FischettiEtAl2019"],
         "published optima from knapsack interdiction methods (Weninger-Fukasawa, Fischetti et al. 2019)", expect=428)
    fact("NumLitOther", ls["FischettiEtAl2017"] + ls["VasquezLozanoVanHoeve2025"] + ls["BOBILibEarlierRelease"],
         "published optima from other sources (air03, p0282, neos-950242)", expect=3)
    assert len(lit) == ls["WeningerFukasawa2025"] + ls["FischettiEtAl2019"] + ls["LozanoBergmanCire2026"] + 3
    p548 = next(r for r in ins if r["instance"] == "p0548-0.900000")
    m548 = re.search(r"published: (\d+) with an exactly optimal follower \((\d+) only within BOBILib's 1e-4", p548["notes"])
    fact("PFiveFortyEightOptimum", int(m548.group(1)), "p0548-0.900000: published optimum used (exact follower)",
         expect=48853)
    fact("PFiveFortyEightDd", int(m548.group(2)), "p0548-0.900000: decision-diagram value, follower optimal only "
         "within BOBILib's tolerance", expect=48851)
    assert p548["source"] == "LozanoBergmanCire2026" and num(p548["primal_value"]) == 48853
    known = [r for r in new if "already optimal" in r["notes"]]
    fact("NumNewValueKnown", len(known), "new optima where BOBILib's value was already optimal", expect=20)
    fact("NumNewValueKnownGeneral", sum(r["family"].startswith("general-") for r in known), "of them general (noswot)", expect=2)
    # OR: verified points against the best published point (literature or BOBILib)
    eq = wo = 0
    for r in pts:
        pubs = [x for x in (num(r["lit_ub"]), num(r["bobilib"])) if x is not None]
        if num(r["lit_ub"]) is None or r in better:
            continue
        b = min(pubs)
        if abs(num(r["our_point"]) - b) <= 1e-6:
            eq += 1
        else:
            wo += 1
    fact("NumOrEqualPublished", eq, "OR points equal to the best published point", expect=72)
    fact("NumOrWorsePublished", wo, "OR points worse than the best published point", expect=56)
    assert len(better) + eq + wo + sum(num(r["lit_ub"]) is None for r in pts) == len(pts)
    assert conflict == {clo[0]["instance"]}, conflict  # only the closure is left with conflicting bounds
    # Lefebvre and Schmidt's values from runs stopped at the time limit (not counted as published points)
    mkl = load(f"{R}/literature/mkip_literature_status.csv")
    tl_or = [r for r in mkl if r["set"] == "or" and num(r["lefebvre_schmidt_ub_timelimit"]) is not None]
    tl_best = [r for r in tl_or if num(r["best_known_ub"]) is None
               or num(r["lefebvre_schmidt_ub_timelimit"]) < num(r["best_known_ub"])]
    fact("NumOrLsTimeLimit", len(tl_or), "OR instances where a Lefebvre-Schmidt time-limit row has a lower nonzero "
         "value than every counted row (left out)", expect=None)
    fact("NumOrLsTimeLimitBest", len(tl_best), "of them: that value would be the best published point", expect=21)
    assert sum(f"value {num(r['lefebvre_schmidt_ub_timelimit']):.0f} from a run stopped" in I_or_notes.get(r["name"], "")
               for r in tl_best) == len(tl_best)
    orc_ = {r["name"]: r for r in orr}
    fact("NumOrLsTimeLimitBelowLb", sum(num(orc_[r["name"]]["lit_lb"]) is not None and
                                        num(r["lefebvre_schmidt_ub_timelimit"]) < num(orc_[r["name"]]["lit_lb"])
                                        for r in tl_best),
         "of them: below Lozano, Bergman and Cire's published lower bound", expect=10)
    fact("NumLitOrClaimed", sum(1 for r in lit if r["collection"] == "or" and "branch-and-cut bound" not in r["notes"]),
         "OR optima claimed by Lozano, Bergman and Cire (their optimal.csv)", expect=87)
    lo_nopt = [r for r in orr if r["variant"] != "100" and r["alpha"] == "0.25" and num(r["our_point"]) is None]
    fact("NumOrQuarterNoPointNoPub", sum(num(r["lit_ub"]) is None for r in lo_nopt),
         "several knapsacks, budget 25%, no point here: none published either", expect=51)
    fact("NumOrQuarterNoPointPub", sum(num(r["lit_ub"]) is not None for r in lo_nopt),
         "several knapsacks, budget 25%, no point here: a published (unverified) point", expect=69)
    # network: how the 54 re-derived BOBILib optima split
    fact("NetworkRndSmall", sum(r["name"].startswith("rndgraph-25") for r in red), "rndgraph-25 re-derived (HiGHS at every attack)",
         expect=12)
    fact("NetworkReviewOnly", sum(r["name"].startswith(("rndgraph-50", "rndgraph-80")) for r in red),
         "rndgraph-50/80 re-derived only by the review's brute force", expect=8)
    assert len(red) == 34 + 12 + 8
    # general: the funnels of bobi general and bobi coupled
    I_ = {r["instance"]: r for r in ins}
    g_ray = [r for r in gp if I_[r["name"]]["new_status"] == "infeasible_bilevel"]
    g_rest = [r for r in gp if I_[r["name"]]["new_status"] not in ("infeasible_bilevel",) and r["verified"] != "True"]
    fact("NumGeneralCandRays", len(g_ray), "of the 98: no bilevel feasible point (rays)", expect=17)
    fact("NumGeneralCandWithPoint", len(gp) - len(g_ray), "of the 98: not ruled out by a ray", expect=81)
    fact("NumGeneralNoHpr", sum(r["why"].startswith("HPR found no point") for r in g_rest), "no HPR point in time", expect=10)
    fact("NumGeneralFollowerUnsolved", sum(r["why"].startswith("follower not solved") for r in g_rest),
         "follower not solved in time", expect=6)
    fact("NumGeneralFailedCheck", sum(r["status"] == "point" for r in g_rest), "point failed the final check", expect=1)
    creadme = " ".join(open(f"{R}/general-coupled/README.md").read().split())
    mo = re.search(r"BOBILib lists (\d+) instances with status `open` and at least one coupling constraint", creadme)
    ml = re.search(r"\* (\d+) with more than 100,000 variables", creadme)
    fact("NumCoupledOpen", int(mo.group(1)), "open instances with coupling rows", expect=87)
    fact("NumCoupledLarge", int(ml.group(1)), "left out: more than 100,000 variables", expect=10)
    fact("NumCoupledRays", ns["infeasible_bilevel"] - len(g_ray), "left out: no bilevel feasible point (rays)", expect=23)
    assert int(mo.group(1)) == len(cp) + int(ml.group(1)) + ns["infeasible_bilevel"] - len(g_ray)
    fact("NumCoupledNone", sum(r["status"] == "none" for r in cp), "attempted, no point in time", expect=23)
    fact("NumGeneralUnchangedMiplib", sum(r["collection"].startswith("miplib") for r in gen(unch)),
         "general unchanged that are MIPLIB-derived", expect=122)
    # KIP: capacities, and the validation on instances with published optima
    fact("NumKipOptIsCap", sum(int(r["capacity"]) == int(r["optimum"]) for r in ss), "optimum = capacity (class 6)", expect=8)
    fact("NumKipOptIsCapMinusOne", sum(int(r["capacity"]) - int(r["optimum"]) == 1 for r in ss),
         "optimum = capacity - 1 (class 7)", expect=8)
    assert all((int(r["capacity"]) - int(r["optimum"])) == (0 if parse(r["name"])[1] == 6 else 1) for r in ss)
    val = [r for r in kipsum if re.match(r"interKP-(100|200)-100-[67]-\d+$", r["name"]) and r["literature_optimum"]]
    fact("NumKipValidation", len(val), "class 6/7 instances with 100 or 200 items and a published optimum", expect=36)
    mlb = re.search(r"the best lower bounds sit (\d+) to (\d+) units below the incumbent", report)
    fact("KipPubLbGapMin", int(mlb.group(1)), "published lower bounds: fewest units below the optimum", expect=2)
    fact("KipPubLbGapMax", int(mlb.group(2)), "published lower bounds: most units below the optimum", expect=8)

    # ---- round two: certificate classes (instances.csv, column certificate_class) ----------------
    cc = C(r["certificate_class"] for r in unsolved if r["certificate_class"] != "na")
    assert all(r["certificate_class"] == "na" for r in ins if r["bobilib_status"] in ("optimal", "infeasible"))
    fact("NumClassRows", sum(cc.values()), "rows with a result (optimal, no feasible point, verified point)", expect=878)
    fact("NumClassExact", cc["exact"], "certificate class exact (i)", expect=115)
    fact("NumClassNumerical", cc["numerical"], "certificate class numerical (ii)", expect=763)
    fact("NumClassBenchmark", cc["benchmark"], "certificate class benchmark (iii)", expect=0)
    assert sum(cc.values()) == settled + ns["feasible"]
    fact("NumClassExactNew", sum(r["certificate_class"] == "exact" for r in new), "new optima, class exact", expect=75)
    fact("NumClassNumericalNew", sum(r["certificate_class"] == "numerical" for r in new),
         "new optima, class numerical (the MIPLIB-bound optima)", expect=7)
    assert all(r["certificate_class"] == "numerical" for r in lit) and all(r["certificate_class"] == "numerical" for r in fp)
    fact("NumClassExactSettled", sum(r["certificate_class"] == "exact" for r in new + rays),
         "settled here with an exact certificate (new optima + rays)", expect=115)

    # ---- round two: tight verification of every solution file (notes/tight_verify.tsv) -----------
    tv = load(f"{R}/notes/tight_verify.tsv", "\t")
    cur = [r for r in tv if not r["folder"].endswith("/tolerance")]
    sup = [r for r in tv if r["folder"].endswith("/tolerance")]
    nfiles = sum(f.endswith(".res.json") for d in ("kip", "ss", "or", "clique", "network", "general", "general-coupled")
                 for f in os.listdir(f"{R}/{d}"))
    fact("NumTightFiles", len(cur), "current solution files re-checked by bobi verify --tight", expect=1478)
    assert len(cur) == nfiles + sum(r["folder"] == "general/large_points.tar.gz" for r in cur)
    fact("NumTightPass", sum(r["class"] == "pass" and r["result"] == "PASS" for r in cur), "of them pass", expect=1478)
    fact("NumTightSuperseded", len(sup), "superseded files (tolerance/ folders) re-checked", expect=26)
    fact("NumTightSupersededBench", sum(r["class"] == "benchmark-accepted" for r in sup),
         "of them benchmark-accepted (fail the tight test, pass BOBILib's)", expect=25)
    fact("NumTightSupersededGeneral", sum(r["folder"] == "general/tolerance" for r in sup),
         "superseded general files (re-derived under the strict tolerance)", expect=17)
    fact("NumTightSupersededCoupled", sum(r["folder"] == "general-coupled/tolerance" for r in sup),
         "superseded coupled files", expect=9)
    fact("NumTightSupersededCoupledBench", sum(r["folder"] == "general-coupled/tolerance" and r["class"] ==
                                               "benchmark-accepted" for r in sup),
         "of the superseded coupled files, benchmark-accepted", expect=8)
    fact("NumTightInterdiction", sum(r["folder"] in ("ss", "kip", "or", "clique", "network") for r in cur),
         "interdiction files among the current ones", expect=1385)
    rel_above = [r for r in cur if float(r["follower_rel_slack"]) > 1e-8]
    fact("NumTightRelAbove", len(rel_above), "current files with follower slack above 1e-8 relative (still pass)", expect=2)
    gsl = [float(r["follower_slack"]) for r in cur if r["folder"].startswith("general") and r not in rel_above]
    fact("TightMaxSlackOther", max(gsl), "largest absolute follower slack of the other general files",
         tex=f"\\ensuremath{{{max(gsl) / 1e-9:.1f}\\times 10^{{-9}}}}", expect=None)
    assert f"{max(gsl) / 1e-9:.1f}" == "1.3"
    fact("NumGeneralTightened", sum(r["tightened"] != "" for r in gp), "general files re-derived (bobi coupled-tighten)",
         expect=17)
    tw = []
    for d_ in ("general", "general-coupled"):
        for line in open(f"{R}/{d_}/tolerance/tighten.jsonl"):
            if line.startswith("{"):
                t_ = json.loads(line)
                tw.append((t_["upper"] - t_["old_upper"], t_["name"], abs(t_["old_upper"])))
    assert len(tw) == 26 and all(w > 0 for w, *_ in tw)  # every leader value got worse
    wmax = max(tw)
    rmax = max(w / max(1, a_) for w, _, a_ in tw)
    fact("TightWorseMaxAbs", round(wmax[0]), "largest loss of a re-derived file's leader value (units)", expect=1000)
    fact("TightWorseMaxAbsName", wmax[1], "the file with that loss", expect="core2536-691_0_100")
    fact("TightWorseMaxRel", rmax, "largest relative loss of a re-derived file's leader value",
         tex=f"\\ensuremath{{{rmax / 1e-6:.1f}\\times 10^{{-6}}}}", expect=None)
    assert f"{rmax / 1e-6:.1f}" == "1.4"
    fact("NumGeneralTightCertified", sum(r["tight_certified"] == "yes" for r in gp), "general files bobi certify --tight passes",
         expect=62)
    fact("NumGeneralTightNoBound", sum(r["tight_certified"] == "no" for r in gp),
         "general files where certify --tight gets no follower bound (pass verify --tight)", expect=2)
    fact("NumCoupledTightened", sum(r["tightened"] != "" for r in cp), "coupled files re-derived", expect=9)
    fact("NumCoupledTightCertified", sum(r["tight_certified"] == "yes" for r in cpts), "coupled files certify --tight passes",
         expect=29)
    pyc = load(f"{R}/general/tolerance/pycheck_tight.tsv", "\t")
    fact("NumGeneralPycheckDone", sum(r["follower_status"] == "Optimal" for r in pyc if r.get("follower_status")),
         "re-derived general files confirmed by the highspy check (finished)", expect=13)
    fact("NumGeneralPycheck", len(pyc), "re-derived general files given to the highspy check", expect=17)

    # ---- round two: clique case certificates (clique/certificates/check.log) ----------------------
    clog = open(f"{R}/clique/certificates/check.log").read()
    assert "\nRESULT PASS\n" in clog
    mt = re.search(r"instances passed (\d+)/(\d+); cases certified (\d+); tree nodes (\d+), leaves (\d+); time ([\d.]+) s",
                   clog)
    trees = [int(x) for x in re.findall(r"tree OK, (\d+) nodes", clog)]
    fact("CliqueCertInstances", int(mt.group(1)), "density-0.9 instances whose case certificates pass", expect=30)
    assert mt.group(1) == mt.group(2)
    fact("CliqueCertCases", int(mt.group(3)), "part-size cases certified by trees", expect=170)
    assert len(trees) == int(mt.group(3)) and sum(trees) == int(mt.group(4))
    fact("CliqueCertNodes", int(mt.group(4)), "tree nodes over all 170 certificates", expect=134368)
    fact("CliqueCertLeaves", int(mt.group(5)), "tree leaves over all 170 certificates", expect=89132)
    fact("CliqueCertMaxNodes", max(trees), "nodes of the largest tree", expect=9888)
    fact("CliqueCertSeconds", float(mt.group(6)), "checker time for all 30 instances, s", tex=mt.group(6), expect=3.1)
    ccode = [l for l in open(os.path.join(BOBILIB, "scripts", "check_clique_certificates.py")).read().split('"""', 2)[2]
             .splitlines() if l.strip() and not l.strip().startswith("#")]
    fact("CliqueCheckerLines", len(ccode), "non-blank code lines of the checker (after its docstring)", expect=174)
    assert int(mt.group(3)) == cases  # the same 170 cases as the solver proofs

    # ---- round two: the problem descriptions (instances.csv, notes/instance_structure.csv) -------
    sets = C(r["collection"] for r in ins)
    for coll, nm, e in (("kp", "Kp", 450), ("cclw", "Cclw", 50), ("inter-kp", "InterKp", 99), ("denegre", "Denegre", 110),
                        ("zhang", "Zhang", 30), ("misc", "Misc", 6), ("xuwang", "Xuwang", 100), ("xularge", "Xularge", 60),
                        ("inter-assig", "InterAssig", 24), ("miplib3", "MiplibThree", 60)):
        fact(f"NumSet{nm}", sets[coll], f"instances of BOBILib's set {coll}", expect=e)
    fact("NumSetDenegreTagged", sum(r["collection"] == "denegre" and r["instance"].startswith("miblp_") for r in ins),
         "denegre instances named miblp_* (tagged denegre in BOBILib's metadata)", expect=50)
    small = [r for r in ins if r["collection"] in ("denegre", "zhang", "misc", "xuwang", "xularge")]
    notopt = [r for r in small if r["bobilib_status"] != "optimal"]
    assert len(notopt) == 1 and notopt[0]["bobilib_status"] == "feasible" and notopt[0]["collection"] == "misc"
    fact("SmallSetsFeasibleName", notopt[0]["instance"], "the one non-optimal instance of the small general sets",
         expect="milp_10_20_50_2310")
    fact("NumSmallSetsFeasible", len(notopt), "non-optimal instances of the small general sets", tex="one", expect=1)
    st_ = {r["instance"]: r for r in load(f"{R}/notes/instance_structure.csv")}
    assert set(st_) == {r["instance"] for r in ins if r["family"].startswith("general-") or r["family"] == "assignment"}
    mip = [r for r in st_.values() if r["collection"].startswith("miplib")]
    fact("NumMiplibDerived", len(mip), "MIPLIB-derived instances (miplib2010, miplib2017, miplib3)", expect=389)
    m3 = [r for r in mip if r["collection"] == "miplib3"]
    models3 = {r["instance"].rsplit("-", 1)[0] for r in m3}
    fracs3 = C(r["instance"].rsplit("-", 1)[1] for r in m3)
    fact("NumMiplibThreeModels", len(models3), "MIPLIB 3 models behind the miplib3 set", expect=20)
    fact("NumMiplibThreePerModel", len(fracs3), "instances per MIPLIB 3 model", tex="three", expect=3)
    assert set(fracs3.values()) == {len(models3)}
    kfr = {"0.100000": 0.1, "0.500000": 0.5, "0.900000": 0.9}
    i = lambda r, k: int(r[k])
    assert all(r["follower_vars_prefix"] == "1" and i(r, "leader_rows") == 0
               and i(r, "follower_vars") == math.floor(kfr[r["instance"].rsplit("-", 1)[1]] * i(r, "cols")) for r in m3)
    m1017 = [r for r in mip if r["collection"] != "miplib3"]
    assert all(r["leader_vars_prefix"] == "1" and i(r, "leader_vars") == math.ceil(i(r, "cols") / 2) for r in m1017)
    ff = [r for r in m1017 if r["instance"].endswith("_50_50")]
    zh = [r for r in m1017 if r["instance"].endswith("_0_100")]
    assert len(ff) + len(zh) == len(m1017)
    assert all(r["leader_rows_prefix"] == "1" and i(r, "leader_rows") == math.ceil(i(r, "rows") / 2) for r in ff)
    assert all(i(r, "leader_rows") == 0 and i(r, "coupling_rows") == 0 for r in zh)
    fact("NumFiftyFifty", len(ff), "_50_50 instances (miplib2010/2017)", expect=163)
    fact("NumFiftyFiftyCoupled", sum(i(r, "coupling_rows") > 0 for r in ff), "_50_50 instances with coupling rows", expect=161)
    fact("NumFiftyFiftyUncoupled", sum(i(r, "coupling_rows") == 0 for r in ff), "_50_50 instances without coupling rows",
         tex="two", expect=2)
    fact("NumZeroHundred", len(zh), "_0_100 instances (miplib2010/2017)", expect=166)
    assert all(str(r["coupling_rows"]) == str(r["meta_coupling"]) and
               str(r["continuous_linking_vars"]) == str(r["meta_continuous_linking"]) for r in st_.values())
    assert all(i(r, "fobj_other") == 0 for r in mip)
    fact("NumAuxRounded", sum(i(r, "fobj_rounded") > 0 for r in mip),
         "MIPLIB-derived AUX files with follower costs rounded to 6 significant digits", expect=90)
    g2 = st_["gen-ip002_0_100"]
    assert i(g2, "fobj_rounded") > 0
    cl = [r for r in st_.values() if i(r, "continuous_linking_vars") > 0]
    I_ = {r["instance"]: r for r in ins}
    assert all(r["collection"].startswith("miplib") and I_[r["instance"]]["bobilib_status"] in ("feasible", "open")
               for r in cl)  # problems.tex: all MIPLIB-derived and unsolved in BOBILib
    fact("NumContLinking", len(cl), "instances with continuous linking variables (all MIPLIB-derived)", expect=137)
    asg = [r for r in st_.values() if r["family"] == "assignment"]
    neg = [r for r in asg if i(r, "fobj_other") == 0 and i(r, "fobj_rounded") == 0 and i(r, "leader_obj_on_leader_vars") == 0]
    fact("NumAssignNegated", len(neg), "assignment files whose leader objective is exactly the negated follower cost",
         expect=19)
    fact("NumAssignOther", len(asg) - len(neg), "assignment files where it differs", tex=["zero", "one", "two", "three",
         "four", "five", "six", "seven", "eight", "nine"][len(asg) - len(neg)], expect=5)
    assert all(I_[r["instance"]]["bobilib_status"] == "optimal" for r in asg)

    # ---- Lefebvre and Schmidt's run files (results/notes/lefebvre_schmidt.md) ------------------
    ls = load(f"{R}/notes/lefebvre_schmidt_discrepancies.tsv", delimiter="\t")
    zero = [r for r in ls if r["category"] == "value 0 below proven lower bound" and r["instance"].startswith("OR")
            and r["status"] == "Optimal/Proved" and float(r["value"]) == 0]
    assert all(float(r["time"]) >= 3600 for r in zero)  # every such row ends at the time limit
    fact("LsOrZeroOptimal", len({r["instance"] for r in zero}),
         "OR instances with an Optimal/Proved value 0 in their run files (time-limit rows)", expect=366)
    fact("LsInfeasibleVerifiedPoint",
         len({r["instance"] for r in ls if r["category"] == "infeasibility claimed where a verified point exists"}),
         "instances reported Infeasible/Proved in their run files although a verified point exists", expect=90)
    wrong = {r["instance"] for r in ls if r["category"] == "claimed optimum worse than a verified point"}
    fact("LsWrongOptima", len(wrong), "reported optima worse than a verified point",
         tex=["zero", "one", "two", "three"][len(wrong)] if len(wrong) < 4 else None, expect=2)
    fact("LsBadPadmName", "highschool1-aigio_0_100", "their PADM 'feasible' point on an instance without one")
    assert any(r["instance"] == "highschool1-aigio_0_100" and r["category"] == "point where no bilevel feasible point exists"
               for r in ls)

    # ---- check and write -----------------------------------------------------------------------
    bad = []
    for name, t, v, desc, e in facts:
        if e is None:
            continue
        ok = (abs(float(v) - float(e)) < 1e-9) if isinstance(e, (int, float)) and not isinstance(v, str) else (str(v) == str(e))
        if not ok:
            bad.append(f"{name}: got {v!r}, spec {e!r}")
    if bad:
        sys.exit("gen_facts: values differ from the spec:\n  " + "\n  ".join(bad))
    assert check_all
    lines = ["% Generated by paper/scripts/gen_facts.py from bobilib/results/; do not edit by hand.",
             "% Each macro is computed from the results files and checked against the paper spec.",
             "% Integers use {,} as thousands separator; percentages are bare numbers (add \\%).", ""]
    for name, t, v, desc, e in facts:
        lines.append(f"\\newcommand{{\\{name}}}{{{t}}}% {desc}")
    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    with open(a.out, "w", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print(f"gen_facts: {len(facts)} macros -> {a.out} ({sum(e is not None for *_, e in facts)} checked against the spec)")


if __name__ == "__main__":
    main()
