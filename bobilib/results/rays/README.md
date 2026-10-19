# General instances with no bilevel feasible point

For each `<name>.ray.json` here, the follower's own rows admit a direction d
(follower variables only) with follower objective e.d < 0. Leader variables
enter follower rows only as constants, so d is a recession direction of the
follower's region for every leader choice. With rational data, a feasible
mixed-integer follower whose LP relaxation is unbounded along d is itself
unbounded (Meyer 1974), so no leader choice has an optimal follower
response: the instance has no bilevel feasible point.

`scripts/verify_ray.py` re-reads the instance independently of the Rust code
and checks each direction in exact rational arithmetic (`ray_verify.jsonl`,
one line per check; a later exact line supersedes an earlier failed one).
`scripts/repair_ray.py` projects a floating-point direction onto the exact
null space of its tight rows when rounding left tiny residuals.
`scripts/exact_ray.py` searches for a certificate in exact arithmetic
without the Rust code: first every unit direction, then (if none works) an
integer direction from a HiGHS MILP, re-checked exactly.

* 40 instances BOBILib lists as open: all 40 checked exactly, so every
  certificate is exact (class (i) in the paper's appendix; column
  `certificate_class` of `../instances.csv`). For supportcase42_0_100 and
  supportcase42_50_50 the floating-point directions (support 9,733) keep
  residuals of about 1e-12 relative, which repair did not remove;
  `exact_ray.py` found a certificate of support one instead:
  `supportcase42_*.exact.ray.json`, d = 1 on the continuous follower
  variable C9734 (bounds [0, inf), follower objective -1), which occurs
  only in one follower row, a <= row with coefficient -1. Increasing it
  keeps any follower point feasible and lowers the follower's objective
  without bound, so these two need not even Meyer's theorem. The exact
  check is the last line for each in `ray_verify.jsonl`.
* nu25-pr12_50_50 (BOBILib: infeasible) also has an exact direction, which
  agrees with BOBILib.
* `gen-ip054_*.exact.ray.json` are the repaired, exact directions for the
  two gen-ip054 instances whose first directions had rounding residuals.
* `../instances.csv` names, for each instance, the file that is its
  certificate (the `.exact.ray.json` file where one exists).

`ray.tsv` lists every instance `bobi ray` was run on, including those
without a follower ray.

```sh
./target/release/bobi ray data/.../gen-ip054_0_100.mps.gz --out results/rays
python3 -I scripts/verify_ray.py INSTANCE.mps.gz INSTANCE.aux results/rays/gen-ip054_0_100.ray.json
python3 -I scripts/exact_ray.py supportcase42_0_100.mps.gz supportcase42_0_100.aux results/rays/supportcase42_0_100.exact.ray.json
python3 -I scripts/verify_ray.py supportcase42_0_100.mps.gz supportcase42_0_100.aux results/rays/supportcase42_0_100.exact.ray.json
```
