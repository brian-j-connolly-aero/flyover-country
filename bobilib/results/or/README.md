# Multidimensional knapsack interdiction, OR set

The 810 OR instances (`OR{rows}x{items}-{budget}_{i}.dat-{variant}`) are
built from Chu and Beasley's multidimensional knapsacks: variant -100 has
one follower knapsack, variants -0 and -50 several. BOBILib lists none as
optimal; Lozano, Bergman and Cire (*Operations Research* 2026) proved 87
(their `optimal.csv`), and one more, OR10x100-0.75_4.dat-0, is closed by
their published point and the branch-and-cut bound in their repository's
run data (8,813).

* `*.res.json`: 690 verified points (all 270 one-knapsack instances; all
  360 several-knapsack instances with budget 50% or 75%; 60 of 180 at budget
  25%). Every file passes `bobi verify` (`verify_*.tsv`; follower re-solved
  exactly by the knapsack oracle or HiGHS).
* `compare.csv`: per instance, the literature's bounds (Lozano et al.'s
  lower bound and the best published point), ours, and the combined gap.
  The best published point counts Lefebvre and Schmidt's values only from
  runs that ended before the time limit, as their own analysis does
  (`../literature/`, `scripts/ls_mkip_points.py`). 474 points are strictly
  better than every published value, BOBILib's included, and 88 are the
  first point for their instance. For one
  follower knapsack, 191 lower bounds beat the published ones.
* `bounds_one_knapsack.jsonl` (`bobi bounds`): lower bounds from the
  dualised follower LP bracket (`lb_milp`) and the merged-row online bound
  (`lb_omega`), and the local-search point (`ub`, the value of the
  solution file here). `lb` = max(`lb_omega`, `lb_milp`). The bracket was
  re-run with a corrected rounding for the 104 instances where `lb_milp`
  exceeds `lb_omega` (see "LP bracket" below); the first version is kept
  as `bounds_one_knapsack.v1.jsonl`.
* `points_several_knapsacks.jsonl` (`bobi upper-multi`): the follower LP
  bound at the bracket's minimiser and whether HiGHS solved the follower
  there exactly (`certified`).

```sh
./target/release/bobi bounds INSTANCE.mps.gz --milp-time 60 --ls-time 30 --out results/or
./target/release/bobi upper-multi INSTANCE.mps.gz --milp-time 30 --follower-time 60 --out results/or
./target/release/bobi verify --instances data/collection results/or
python3 -I scripts/compare_or.py mkip_literature_status.csv collection.json bounds_one_knapsack.jsonl points_several_knapsacks.jsonl --csv compare.csv
```

## LP bracket

The bracket solves min over leader choices x of U(x), the follower's LP
value, as one MILP (the follower's LP dualised, `PackingInterdiction::
lp_follower_bound` in `src/mkip.rs`). With one follower knapsack the
follower's integer optimum is at least U(x) - max_j p_j, so the bilevel
optimum is at least min_x U(x) - max_j p_j. The first version took
min_x U(x) from HiGHS's dual bound but fell back to the incumbent's value
when the dual bound was not finite (the incumbent's value is an upper bound
on min_x U(x), not a lower one), and rounded ceil(min_lp - pmax) with no
allowance for the floating-point error of the dual bound. The corrected
code uses only a finite dual bound (otherwise there is no bracket bound:
`lb_milp` null, `lb_milp_proven` false) and rounds
ceil(min_lp - pmax - 1e-6 max(1, |min_lp|)). The bound remains a
solver-certified numerical result (class (ii)): it rests on HiGHS's dual
bound within that margin.

All 104 instances with `lb_milp` > `lb_omega` were re-run with `bobi
bounds --milp-only --milp-time 120` (two instances at a time; the first
version ran with `--milp-time 60` on a busier machine). Every re-run has a
finite dual bound; 34 bracket MILPs were solved to optimality, 70 stopped at
the time limit. Re-run rows carry `lb_milp_proven`, `milp_optimal`,
`min_lp_bound` (HiGHS's dual bound), `milp_time`, `bracket_secs`,
`bracket_ub_at_x` (the follower's proven optimum at the bracket MILP's
leader choice) and `bracket_rerun` true; the other 166 rows have
`bracket_rerun` false and `lb_milp_proven` null (not re-run: their `lb` is
`lb_omega`, the exact online bound).

Of the 104 lower bounds, 79 rose (the longer time limit; by 1 to 1,425),
21 are unchanged and 4 fell by 1 (OR5x100-0.75_5, _8 and _10 and
OR10x100-0.75_5, variant -100: their dual bound is an integer plus a
rounding error, 13322.000000000024 for example, and the margin now keeps
ceil from rounding up past the integer; all four are optimal in the
literature, whose bound is higher). No first-version bound is more than 1
above its corrected value. 17 of the rises raise the best known lower
bound (ours was already above the published one in all 17); the count of
lower bounds better than the published ones stays 191. For 50
of the 104 instances the bracket's own leader choice has a better follower
optimum than the solution file here (`bracket_ub_at_x` < `ub`); those points
were not written.

```sh
./target/release/bobi bounds INSTANCE.mps.gz --milp-only --milp-time 120 >> bracket.jsonl
python3 -I scripts/or_bracket_update.py bounds_one_knapsack.v1.jsonl bracket.jsonl bounds_one_knapsack.jsonl
python3 -I scripts/compare_or.py mkip_literature_status.csv collection.json bounds_one_knapsack.jsonl points_several_knapsacks.jsonl --csv compare.csv
```
