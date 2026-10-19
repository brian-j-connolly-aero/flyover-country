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
optimistic tie-break). `pigeon-10_50_50` (open, with coupling rows) gets
its point at the bound from `bobi coupled`; the file is in
`results/general-coupled/`. For `noswot_*` and `neos-950242_0_100`, BOBILib's
own solution files already had the optimal value. For `noswot_*` the
proof is new; `neos-950242_0_100` was already listed as optimal in an
earlier BOBILib release, and the current one lists it as feasible.

## Feasible points without coupling rows (`points.csv`)

`bobi general` on the 98 open instances without coupling rows: 64 verified
bilevel feasible points (BOBILib lists no value for any of them). For 9 of
them a point had been published elsewhere (Fischetti et al. 2017; Vasquez,
Lozano and van Hoeve 2025; Lozano, Bergman and Cire 2026; an earlier BOBILib
release; Lefebvre and Schmidt 2025); the notes of `../instances.csv` give
each published value, and for 6 of the 9 it is better than the point here. Of the other 34, 17 have no bilevel feasible point at all
(`../rays`); for 10 HiGHS found no HPR point in 300 s, for 6 the follower
was not solved to optimality in time, and 1 point failed the final check. Each
passes `bobi verify --generic` (rows of both levels within 1e-6, the stated
objective, the follower re-solved by HiGHS with the leader fixed and found
optimal within BOBILib's relative 1e-4). `points.csv` has one row per
instance, including the 34 without a point and why.

The nine solution files over 1 MB are in `large_points.tar.gz`
(`tar xzf large_points.tar.gz` before verifying).

**Tight tolerance.** `bobi general`'s optimistic tie-break lets the
follower's value sit up to 1e-7 (1 + |phi|) above its optimum. Under `bobi
verify --tight` (follower within max(1e-6, 1e-9 |phi|) of its proven
optimum) 47 of the 64 files pass; the other 17 (15 here, neos-873061_0_100
and netdiversion_0_100 in the archive) pass only BOBILib's relative 1e-4,
with the follower about 1e-7 relative above its optimum, so they are
benchmark-accepted points. None of them is an HPR-bound optimum. A dry run
of `bobi coupled-tighten` re-derives all 17 under the strict tolerance at
their own leader values, each worse by the follower's slack (6.3e-6 to
1,000; about 1e-7 relative). The files here were not replaced. Details:
`../notes/tight_verify.md`.
