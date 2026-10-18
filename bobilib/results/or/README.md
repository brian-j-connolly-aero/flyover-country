# Multidimensional knapsack interdiction, OR set

The 810 OR instances (`OR{rows}x{items}-{budget}_{i}.dat-{variant}`) are
built from Chu and Beasley's multidimensional knapsacks: variant -100 has
one follower knapsack, variants -0 and -50 several. BOBILib lists none as
optimal; Lozano, Bergman and Cire (*Operations Research* 2026) proved 87.

* `*.res.json`: 690 verified points (all 270 one-knapsack instances; all
  360 several-knapsack instances with budget 50% or 75%; 60 of 180 at budget
  25%). Every file passes `bobi verify` (`verify_*.tsv`; follower re-solved
  exactly by the knapsack oracle or HiGHS).
* `compare.csv`: per instance, the literature's bounds (Lozano et al.'s
  lower bound and the best published point), ours, and the combined gap.
  469 points are strictly better than every published value, BOBILib's
  included, and 88 are the first point for their instance. For one
  follower knapsack, 191 lower bounds beat the published ones.
* `bounds_one_knapsack.jsonl` (`bobi bounds`): lower bounds from the
  dualised follower LP bracket (`lb_milp`) and the merged-row online bound
  (`lb_omega`), and the local-search point.
* `points_several_knapsacks.jsonl` (`bobi upper-multi`): the follower LP
  bound at the bracket's minimiser and whether HiGHS solved the follower
  there exactly (`certified`).

```sh
./target/release/bobi bounds INSTANCE.mps.gz --milp-time 60 --ls-time 30 --out results/or
./target/release/bobi upper-multi INSTANCE.mps.gz --milp-time 30 --follower-time 60 --out results/or
./target/release/bobi verify --instances data/collection results/or
python3 -I scripts/compare_or.py mkip_literature_status.csv collection.json bounds_one_knapsack.jsonl points_several_knapsacks.jsonl --csv compare.csv
```
