"""Fold a re-run of the LP-follower bracket into bounds_one_knapsack.jsonl.

usage: python3 -I scripts/or_bracket_update.py OLD.jsonl BRACKET.jsonl OUT.jsonl

OLD.jsonl: `bobi bounds` lines (lb, lb_omega, lb_milp, ub, ...).
BRACKET.jsonl: `bobi bounds --milp-only` lines for the re-run instances
(lb_milp from HiGHS's dual bound with the safety margin, or null when HiGHS
reported no finite dual bound; lb_milp_proven; milp_optimal; min_lp_bound).
Re-run rows get the new lb_milp and its fields, `bracket_rerun` true,
lb = max(lb_omega, lb_milp) and the gap recomputed; ub (the value of the
solution file in results/or) is kept, and `bracket_ub_at_x` records the
follower's proven optimum at the bracket MILP's leader choice. Other rows keep their values, with `bracket_rerun` false and
`lb_milp_proven` null (not re-checked; their lb_milp does not exceed
lb_omega, so lb does not depend on it). Prints every lb that changed.
"""
import json
import sys


def main():
    old_path, new_path, out_path = sys.argv[1:4]
    new = {}
    for line in open(new_path):
        line = line.strip()
        if line.startswith("{"):
            d = json.loads(line)
            assert "name" in d and "error" not in d and "skip" not in d, line
            new[d["name"]] = d
    out, seen = [], set()
    for line in open(old_path):
        line = line.strip()
        if not line:
            continue
        d = json.loads(line)
        if "name" not in d:
            out.append(d)
            continue
        n = d["name"]
        if n in new:
            b = new[n]
            seen.add(n)
            lb_old = d["lb"]
            d["lb_milp"] = b["lb_milp"]
            d["lb_milp_proven"] = b["lb_milp_proven"]
            d["milp_optimal"] = b["milp_optimal"]
            d["min_lp_bound"] = b["min_lp_bound"]
            d["milp_time"] = b["milp_time"]
            d["bracket_secs"] = b["secs"]
            # the follower's proven optimum at the bracket MILP's leader choice
            # (a verified point value; ub stays the value of the solution file)
            d["bracket_ub_at_x"] = b["ub_at_x"]
            d["bracket_rerun"] = True
            d["lb"] = d["lb_omega"] if b["lb_milp"] is None else max(d["lb_omega"], b["lb_milp"])
            d["gap"] = round((d["ub"] - d["lb"]) / max(d["ub"], 1), 5)
            if d["lb"] != lb_old:
                print(f"{n}\tlb {lb_old} -> {d['lb']}\tlb_milp proven={b['lb_milp_proven']} "
                      f"optimal={b['milp_optimal']}\tmin_lp_bound={b['min_lp_bound']}")
        else:
            d["lb_milp_proven"] = None
            d["bracket_rerun"] = False
        out.append(d)
    missing = set(new) - seen
    assert not missing, missing
    with open(out_path, "w") as f:
        for d in out:
            f.write(json.dumps(d, separators=(",", ":")) + "\n")
    print(f"{len(seen)} re-run rows, {len(out)} lines written")


main()
