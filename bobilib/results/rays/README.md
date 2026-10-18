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

* 40 instances BOBILib lists as open: 38 checked exactly; for
  supportcase42_0_100 and supportcase42_50_50 the residuals are about
  1e-12 relative and repair did not remove them, so those two rest on
  floating point.
* nu25-pr12_50_50 (BOBILib: infeasible) also has an exact direction, which
  agrees with BOBILib.
* `gen-ip054_*.exact.ray.json` are the repaired, exact directions for the
  two gen-ip054 instances whose first directions had rounding residuals.

`ray.tsv` lists every instance `bobi ray` was run on, including those
without a follower ray.

```sh
./target/release/bobi ray data/.../gen-ip054_0_100.mps.gz --out results/rays
python3 -I scripts/verify_ray.py INSTANCE.mps.gz INSTANCE.aux results/rays/gen-ip054_0_100.ray.json
```
