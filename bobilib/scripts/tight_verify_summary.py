"""Collect `bobi verify --tight` outputs (one TSV per results folder) into
results/notes/tight_verify.tsv and print per-folder counts.

usage: python3 -I scripts/tight_verify_summary.py OUT.tsv FOLDER=VERIFY.tsv ...

Each row gets a class:
  pass                 passes the tight test (rows, bounds and integrality
                       within 1e-6, the stated objective, the follower's
                       value within max(1e-6, 1e-9 |phi|) of its optimum)
  benchmark-accepted   fails only the tight follower test: BOBILib's
                       relative 1e-4 accepts the same follower value
  follower-not-optimal fails BOBILib's follower test too
  row-violation        a row, bound or integrality violated by more than 1e-6
  objective-mismatch   the stated objective is not the point's
  unproven             the follower's optimum was not proven in time
  error / no-solution  the file could not be checked
"""
import csv
import sys

COLS = ["folder", "name", "class", "reason", "result", "max_violation", "violation_at",
        "objective", "file_objective", "follower_optimal", "follower_bobilib", "phi_sol",
        "phi_check", "phi_bound", "follower_slack", "follower_rel_slack", "oracle",
        "seconds", "note"]


def num(s):
    try:
        return float(s)
    except (TypeError, ValueError):
        return None


def classify(r):
    res = r["result"]
    if res == "PASS":
        return "pass", ""
    if res == "ERROR":
        return "error", r["note"]
    if res == "NOSOL":
        return "no-solution", r["note"]
    reasons = []
    if r["feasible"] != "yes":
        reasons.append(f"row/bound/integrality violation {r['max_violation']} at {r['violation_at']}")
    if r["objective_match"] != "yes":
        reasons.append(f"stated objective {r['file_objective']} vs {r['objective']}")
    if reasons:
        cls = "row-violation" if r["feasible"] != "yes" else "objective-mismatch"
        return cls, "; ".join(reasons)
    slack = r["follower_slack"]
    if r["follower_optimal"] == "unproven":
        return "unproven", f"follower optimum not proven ({r['note']})"
    if r["follower_bobilib"] != "yes":
        return "follower-not-optimal", f"follower value {r['phi_sol']} vs optimum {r['phi_check']}"
    return "benchmark-accepted", f"follower slack {slack} above the proven optimum"


def main():
    out = sys.argv[1]
    rows = []
    for arg in sys.argv[2:]:
        folder, path = arg.split("=", 1)
        with open(path, newline="") as f:
            for r in csv.DictReader(f, delimiter="\t"):
                cls, why = classify(r)
                s, b = num(r.get("follower_slack")), num(r.get("phi_bound"))
                rel = f"{s / (1e-10 + abs(b)):.3e}" if s is not None and b is not None else "-"
                row = {k: r.get(k, "") for k in COLS}
                row.update(folder=folder, **{"class": cls, "reason": why or "-",
                                             "follower_rel_slack": rel})
                rows.append(row)
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, COLS, delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    classes = ["pass", "benchmark-accepted", "follower-not-optimal", "row-violation",
               "objective-mismatch", "unproven", "error", "no-solution"]
    folders = list(dict.fromkeys(r["folder"] for r in rows))
    print("folder\tfiles\t" + "\t".join(classes))
    for fo in folders + ["all"]:
        sel = [r for r in rows if fo in ("all", r["folder"])]
        print(f"{fo}\t{len(sel)}\t" + "\t".join(str(sum(r["class"] == c for r in sel)) for c in classes))
    for r in rows:
        if r["class"] != "pass":
            print(f"{r['folder']}\t{r['name']}\t{r['class']}\t{r['reason']}\trel={r['follower_rel_slack']}")


main()
