# General MIPLIB-derived instances

## Optimal because the point meets the HPR bound (`hpr_optimal.csv`)

BOBILib builds these instances from a MIPLIB problem by assigning variables
and rows to the two levels; the leader minimises the original objective. Its
high point relaxation (HPR: every row of both levels, the leader's objective,
no optimality condition on the follower) is therefore the MIPLIB problem
itself, whose optimum is a lower bound on the bilevel optimum. A bilevel
feasible point with that value is optimal.

For each row, `scripts/compare_hpr.py` checked in exact arithmetic that the
instance's HPR is the MIPLIB 2017 model (same variables, rows,
coefficients, sides, bounds, integrality and objective), and `bobi verify
--generic` checked the point (rows, objective, follower optimality by an
exact re-solve). MIPLIB optima are from `miplib2017-v31.solu`.

```sh
python3 -I scripts/compare_hpr.py INSTANCE.mps.gz INSTANCE.aux noswot.mps.gz
./target/release/bobi verify --instances data/collection --generic results/general
```

`pigeon-10_0_100` and `timtab1_0_100` were open (no point known); the
points here come from `bobi general` (HPR, then the follower, then an
optimistic tie-break). For `noswot_*` and `neos-950242_0_100`, BOBILib's
own solution files already had the optimal value. For `noswot_*` the
proof is new; `neos-950242_0_100` was already listed as optimal in an
earlier BOBILib release, and the current one lists it as feasible.
