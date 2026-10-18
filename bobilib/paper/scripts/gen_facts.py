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
    fact("NumLitOptimal", len(lit), "already optimal in published work", expect=527)
    fact("NumNewOptimal", len(new), "proven optimal here", expect=81)
    fact("NumNoFeasible", ns["infeasible_bilevel"], "no bilevel feasible point (here)", expect=40)
    fact("NumFirstPoint", ns["feasible"], "first verified point here, BOBILib has none", expect=230)
    fact("NumUnchanged", ns["unchanged"], "status unchanged", expect=716)
    settled = len(lit) + len(new) + ns["infeasible_bilevel"]
    fact("NumSettled", settled, "settled = literature + new optimal + no feasible point", expect=648)
    fact("PctSettled", round(100 * settled / len(unsolved)), "settled share of the unsolved, percent", expect=41)
    fact("NumOptimalNow", ns["optimal"], "optimal now among the unsolved", expect=608)
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
    fact("NumLitLozano", ls["LozanoBergmanCire2026"], "optima of Lozano, Bergman and Cire (all families)", expect=96)
    fact("NumLitOr", sum(1 for r in lit if r["collection"] == "or"), "OR optima of Lozano et al.", expect=87)
    fact("NumLitGeneralized", len(fam(lit, "generalized")), "generalized optima of Lozano et al.", expect=4)
    fact("NumLitMiplibLozano", sum(1 for r in gen(lit) if r["source"] == "LozanoBergmanCire2026"),
         "MIPLIB 3 optima of Lozano et al.", expect=5)
    fact("NumLitImkp", ls["FischettiEtAl2019"], "imkp optima of Fischetti et al. 2019", expect=52)
    fact("NumLitMkip", len(fam(lit, "multidim-knapsack")), "MKIP literature optima (OR + imkp)", expect=139)
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
    fact("NumNewHpr", len(gen(new)), "MIPLIB-bound optima", expect=5)
    fact("NumNewOr", len(fam(new, "multidim-knapsack")), "OR closure", expect=1)
    rays = [r for r in unsolved if r["new_status"] == "infeasible_bilevel"]
    fact("NumRayExact", sum(r["verified"] == "yes" for r in rays), "rays checked in exact arithmetic", expect=38)
    fact("NumRayFloat", sum(r["verified"] == "no" for r in rays), "rays checked in floating point only", expect=2)
    fp = [r for r in unsolved if r["new_status"] == "feasible"]
    orfp = [r for r in fp if r["collection"] == "or"]
    fact("NumOrFirstAnywhere", sum("first point anywhere" in r["notes"] for r in orfp), "OR first points anywhere", expect=88)
    fact("NumOrPointNewToBobilib", sum("first point anywhere" not in r["notes"] for r in orfp),
         "OR points where the literature had one, BOBILib none", expect=52)
    fact("NumOrFirstPoints", len(orfp), "OR points where BOBILib has none", expect=140)
    gfp = gen(fp)
    fact("NumGeneralPoints", sum(r["certificate"].startswith("general/") for r in gfp), "general first points without coupling", expect=62)
    fact("NumCoupledPoints", sum(r["certificate"].startswith("general-coupled/") for r in gfp), "general first points with coupling", expect=28)
    fact("NumGeneralFirstPoints", len(gfp), "general first points", expect=90)
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
    fact("NumKipEasy", len(easy), "the 16 other than class 6, budget index 3", expect=12)
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
    fact("NumOrBetter", len(better), "points strictly better than every published value", expect=469)
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
    spec_gaps = {("100", "0.25"): ("24.0", "4.1"), ("100", "0.50"): ("34.0", "7.1"), ("100", "0.75"): ("0.14", "0.14"),
                 ("0", "0.25"): ("33.3", "31.4"), ("0", "0.50"): ("36.6", "29.1"), ("0", "0.75"): ("5.1", "0.04"),
                 ("50", "0.25"): ("29.6", "28.2"), ("50", "0.50"): ("41.3", "34.8"), ("50", "0.75"): ("11.8", "1.4")}
    spec_nopoint = {("0", "0.25"): (31, 0), ("0", "0.50"): (31, 31), ("0", "0.75"): (27, 27),
                    ("50", "0.25"): (20, 0), ("50", "0.50"): (20, 20), ("50", "0.75"): (10, 10)}
    for (v, al), (lit_s, now_s) in spec_gaps.items():
        grp_rows = [r for r in orr if r["variant"] == v and r["alpha"] == al and num(r["gap"]) is not None]
        paired = [r for r in grp_rows if num(r["lit_gap"]) is not None]
        lg, g = pct(med([num(r["lit_gap"]) for r in paired])), pct(med([num(r["gap"]) for r in paired]))
        fact(f"OrGap{vname[v]}{bname[al]}Lit", lg, f"median published gap, variant -{v}, budget {al}", tex=lg, expect=lit_s)
        fact(f"OrGap{vname[v]}{bname[al]}Now", g, f"median gap now, variant -{v}, budget {al}", tex=g, expect=now_s)
        fact(f"OrPaired{vname[v]}{bname[al]}", len(paired), f"instances in that median", expect=None)
        if (v, al) in spec_nopoint:
            nop = [r for r in orr if r["variant"] == v and r["alpha"] == al and num(r["lit_ub"]) is None]
            fact(f"OrNoPoint{vname[v]}{bname[al]}", len(nop), f"no published point, -{v}, {al}", expect=spec_nopoint[(v, al)][0])
            fact(f"OrFirst{vname[v]}{bname[al]}", sum(num(r["our_point"]) is not None for r in nop),
                 f"first points among those", expect=spec_nopoint[(v, al)][1])
    clo = [r for r in ins if r["collection"] == "or" and r["new_status"] == "optimal" and r["source"] == "this work"]
    assert len(clo) == 1
    c0 = next(r for r in orr if r["name"] == clo[0]["instance"])
    fact("OrClosureName", clo[0]["instance"], "the OR instance closed here", expect="OR10x100-0.75_4.dat-0")
    fact("OrClosureValue", int(num(clo[0]["primal_value"])), "its optimum", expect=8813)
    fact("OrClosureLitLb", int(num(c0["lit_lb"])), "published (inconsistent) lower bound", expect=8814)
    fact("NumOrInconsistent", sum(num(r["lit_lb"]) is not None and num(r["lit_ub"]) is not None
                                  and num(r["lit_lb"]) > num(r["lit_ub"]) for r in orr),
         "OR instances whose published lower bound exceeds the published point", expect=4)
    fact("NumOrLitOptimalPoint", sum(1 for r in ins if r["collection"] == "or" and r["source"] == "LozanoBergmanCire2026"
                                     and r["certificate"]), "published OR optima reproduced by a file here", expect=69)

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
    fact("CliqueOpenNames", ", ".join(r["name"] for r in openq), "the three open instances",
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
    fact("NumGeneralFilesOptimal", sum("results/general/" in r["point_from"] for r in hprnew), "of them optimal", expect=2)
    fact("NumGeneralFilesFeasible", sum(r["verified"] == "True" for r in gp) - sum("results/general/" in r["point_from"] for r in hprnew),
         "of them feasible only", expect=62)
    cp = load(f"{R}/general-coupled/summary.csv")
    cpts = [r for r in cp if r["status"] == "point"]
    fact("NumCoupledCandidates", len(cp), "open instances with coupling rows run through bobi coupled", expect=54)
    fact("NumCoupledAttempted", sum(r["status"] != "not_attempted" for r in cp), "attempted", expect=52)
    fact("NumCoupledFiles", len(cpts), "verified files in results/general-coupled", expect=29)
    fact("NumCoupledOptimal", sum("results/general-coupled/" in r["point_from"] for r in hprnew), "of them optimal", expect=1)
    fact("NumCoupledTolerance", sum((num(r["rel_error"]) or 0) > 1e-8 for r in cpts),
         "coupled files whose follower value uses BOBILib's tolerance", expect=9)
    mc = C(r["method"] for r in cpts)
    fact("NumCoupledByHpr", mc["hpr"], "coupled points at the HPR's leader values", expect=21)
    fact("NumCoupledByPenalty", mc["penalty"], "coupled points from the penalised HPR", expect=5)
    fact("NumCoupledByFollow", mc["follow"], "coupled points from best-response dynamics", expect=3)
    fact("NumHprOptimal", len(hprnew), "points meeting the MIPLIB bound", expect=5)
    fact("NumHprOpen", sum(r["bobilib_status"] == "open" for r in hprnew), "of them open before", expect=3)
    val = {r["name"]: num(r["optimum"]) for r in hpr}
    fact("PigeonOptimum", val["pigeon-10_0_100"], "pigeon-10 optimum", expect=-9000)
    assert val["pigeon-10_50_50"] == val["pigeon-10_0_100"]
    fact("TimtabOptimum", val["timtab1_0_100"], "timtab1_0_100 optimum (full precision)",
         tex=tex_num(val["timtab1_0_100"]), expect=764771.99999978)
    fact("TimtabOptimumRounded", round(val["timtab1_0_100"]), "timtab1_0_100 optimum rounded", expect=764772)
    fact("NoswotOptimum", val["noswot_0_100"], "noswot optimum", expect=-41)
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
    fact("NumSoluOpt", sc["=opt="], ".solu =opt= lines", expect=608)
    fact("NumSoluInf", sc["=inf="], ".solu =inf= lines", expect=40)
    fact("NumSoluBest", sc["=best="], ".solu =best= lines", expect=None)
    fact("NumSoluBestDual", sc["=bestdual="], ".solu =bestdual= lines", expect=None)

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
