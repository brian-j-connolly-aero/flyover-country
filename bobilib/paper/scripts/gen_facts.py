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
    fact("NumNewOptimal", len(new), "proven optimal here", expect=80)
    fact("NumNoFeasible", ns["infeasible_bilevel"], "no bilevel feasible point (here)", expect=40)
    fact("NumFirstPoint", ns["feasible"], "verified point here where BOBILib has none (first anywhere or not)", expect=230)
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
    fact("NumNewHpr", len(gen(new)), "MIPLIB-bound optima", expect=5)
    fact("NumNewOr", len(fam(new, "multidim-knapsack")), "OR closure", expect=0)
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
    # Paired medians leave out the instances whose published lower bound exceeds the published point
    # (referee review: inconsistent sources). Values as in the spec except the three affected groups.
    spec_gaps = {("100", "0.25"): ("24.0", "4.1"), ("100", "0.50"): ("34.0", "7.1"), ("100", "0.75"): ("0.14", "0.14"),
                 ("0", "0.25"): ("33.3", "31.8"), ("0", "0.50"): ("36.6", "29.1"), ("0", "0.75"): ("5.3", "0.05"),
                 ("50", "0.25"): ("30.6", "28.3"), ("50", "0.50"): ("41.3", "34.8"), ("50", "0.75"): ("11.8", "1.4")}
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
    c0 = next(r for r in orr if r["name"] == clo[0]["instance"])
    assert clo[0]["new_status"] == "optimal" and clo[0]["source"] == "LozanoBergmanCire2026"
    fact("OrClosureName", clo[0]["instance"], "OR instance whose published lower bound exceeds the published point "
         "but whose published branch-and-cut bound closes it", expect="OR10x100-0.75_4.dat-0")
    fact("OrClosureValue", int(num(clo[0]["primal_value"])), "its optimum (published point and bound)", expect=8813)
    fact("OrClosureLitLb", int(num(c0["lit_lb"])), "published (inconsistent) lower bound", expect=8814)
    fact("NumOrInconsistent", sum(num(r["lit_lb"]) is not None and num(r["lit_ub"]) is not None
                                  and num(r["lit_lb"]) > num(r["lit_ub"]) for r in orr),
         "OR instances whose published lower bound exceeds the published point", expect=4)
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
         tex=", ".join(f"\\inst{{{tex_name(r['name'])}}}" for r in openq),
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
    assert len(hrows) == 5
    nsw = {float(r["miplib_optimum"]) for r in hpr if r["name"].startswith("noswot")}
    assert len(nsw) == 1
    nsw = nsw.pop()
    fact("NoswotMiplibOptimum", nsw, "MIPLIB's optimum of noswot (floating point)", tex=tex_num(nsw),
         expect=-41.00000885)
    fact("HprTableRows", "rows", "rows of the paper's table of the MIPLIB-bound optima", tex="%\n" + "\n".join(hrows) + "\n")

    # ---- review round: categories, funnels and splits the revised text quotes ---------------------
    fp_all = [r for r in unsolved if r["new_status"] == "feasible"]
    first_any = [r for r in fp_all if "first point anywhere" in r["notes"]]
    fact("NumFirstAnywhere", len(first_any), "verified points here that are the first point anywhere", expect=168)
    fact("NumPointPublishedBefore", len(fp_all) - len(first_any),
         "verified points here where BOBILib has none but a point was published", expect=62)
    gfa = [r for r in first_any if r["family"].startswith("general-")]
    fact("NumGeneralFirstAnywhere", len(gfa), "general first points anywhere", expect=80)
    gpub = [r for r in fp_all if r["family"].startswith("general-") and "first point anywhere" not in r["notes"]]
    fact("NumGeneralPublishedPoint", len(gpub), "general points where a point was published", expect=10)
    fact("NumGeneralPublishedBetter", sum(", better than this one" in r["notes"] for r in gpub),
         "of them: the published point is better", expect=6)
    fact("NumGeneralPublishedLS", sum("Lefebvre and Schmidt" in r["notes"] for r in gpub),
         "of them: published only in Lefebvre and Schmidt's run files", expect=4)
    gpub_nocpl = [r for r in gpub if r["certificate"].startswith("general/")]
    fact("NumGeneralNoCplPublished", len(gpub_nocpl), "bobi general points where a point was published", expect=9)
    fact("NumGeneralNoCplPublishedBetter", sum(", better than this one" in r["notes"] for r in gpub_nocpl),
         "of them: the published point is better", expect=6)
    fact("NumCoupledPublished", sum(r["certificate"].startswith("general-coupled/") for r in gpub),
         "bobi coupled points where a point was published", expect=1)
    fact("NumOrPublishedPointFile", sum("solution file in their repository" in r["notes"] for r in fp_all
                                        if r["collection"] == "or" and "first point anywhere" not in r["notes"]),
         "OR points new to BOBILib whose published point has a solution file (Lozano et al.)", expect=52)
    fact("NumSettledHere", len(new) + ns["infeasible_bilevel"], "settled by this work (new optima + no point)", expect=120)
    fact("PctSettledHere", f"{100 * (len(new) + ns['infeasible_bilevel']) / len(unsolved):.1f}",
         "settled by this work, percent of the unsolved", tex=f"{100 * (len(new) + ns['infeasible_bilevel']) / len(unsolved):.1f}",
         expect="7.5")
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
    fact("NumOrWorsePublished", wo, "OR points worse than the best published point", expect=61)
    assert len(better) + eq + wo + sum(num(r["lit_ub"]) is None for r in pts) == len(pts)
    conf = sorted((r for r in orr if r["name"] in conflict and r["name"] != clo[0]["instance"]),
                  key=lambda r: (int(r["m"]), int(r["n"]), r["name"]))
    fact("OrConflictNames", ", ".join(r["name"] for r in conf), "OR instances whose published point lies below the "
         "published lower bound (besides the closure; each in \\inst)",
         tex=", ".join(f"\\inst{{{tex_name(r['name'])}}}" for r in conf),
         expect="OR5x100-0.25_7.dat-50, OR5x100-0.25_8.dat-0, OR10x500-0.25_5.dat-50")
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
