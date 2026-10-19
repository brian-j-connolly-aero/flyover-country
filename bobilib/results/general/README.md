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

`pigeon-10_0_100`, `timtab1_0_100`, `ex10_0_100` and `neos-1445765_0_100`
were open (no point known); the points here come from `bobi general` (HPR,
then the follower, then an optimistic tie-break). For ex10 (MIPLIB optimum
100, all-ones objective on binaries) and neos-1445765 (MIPLIB optimum
-17783, integer coefficients on integer variables only) the objective takes
integer values only, so, as for noswot, the bound is safe by integrality
rounding if MIPLIB's published bound is valid; for timtab1 the agreement is
numerical. These optima are solver-certified numerical results (they rest
on MIPLIB's floating-point optima), not exact-arithmetic proofs; the column
`certificate_class` of `../instances.csv` says `numerical`. Lefebvre and
Schmidt's MILP run files also report ex10_0_100 optimal at 100; their paper
does not state it, so it is not counted as published
(`../notes/lefebvre_schmidt.md`). `pigeon-10_50_50` (open, with coupling rows) gets
its point at the bound from `bobi coupled`; the file is in
`results/general-coupled/`. For `noswot_*` and `neos-950242_0_100`, BOBILib's
own solution files already had the optimal value. For `noswot_*` the
proof is new; `neos-950242_0_100` was already listed as optimal in an
earlier BOBILib release, and the current one lists it as feasible.

## Feasible points without coupling rows (`points.csv`)

`bobi general` on the 98 open instances without coupling rows: 64 verified
bilevel feasible points (BOBILib lists no value for any of them); four of
them are optimal at the HPR bound (above). For 8 of the other 60 a point had
been published elsewhere (Fischetti et al. 2017; Vasquez, Lozano and van Hoeve
2025; Lozano, Bergman and Cire 2026; an earlier BOBILib release; Lefebvre and
Schmidt 2025); the notes of `../instances.csv` give each published value, and
for 6 of the 8 it is better than the point here. Of the four optimal ones,
ex10_0_100 also has a published point, at the same value, in Lefebvre and
Schmidt's run files (not counted as a published optimum). Of the other 34, 17 have no bilevel feasible point at all
(`../rays`); for 10 HiGHS found no HPR point in 300 s, for 6 the follower
was not solved to optimality in time, and 1 point failed the final check. Each
passes `bobi verify --generic` (rows of both levels within 1e-6, the stated
objective, the follower re-solved by HiGHS with the leader fixed and found
optimal within BOBILib's relative 1e-4). `points.csv` has one row per
instance, including the 34 without a point and why.

The nine solution files over 1 MB are in `large_points.tar.gz`
(`tar xzf large_points.tar.gz` before verifying).

## Strict tolerance

`bobi general`'s optimistic tie-break let the follower's value sit up to
1e-7 (1 + |phi|) above its optimum, well inside BOBILib's relative 1e-4,
and the leader's objective used that slack. Under `bobi verify --tight`
(follower within max(1e-6, 1e-9 |phi|) of its proven optimum,
`../notes/tight_verify.md`) 17 of the 64 original files failed: 15 loose
files and neos-873061_0_100 and netdiversion_0_100 in the archive, with the
follower 5e-8 to 1e-7 relative above its optimum. They passed only
BOBILib's tolerance-based check (benchmark-accepted points). None of them
is an HPR-bound optimum, and none has coupling rows.

The 17 files were replaced by points re-derived under the strict
tolerance, exactly as for the coupled folder (`../general-coupled/README.md`,
"Strict tolerance"): `bobi coupled-tighten` (`../../scripts/tighten_coupled.sh`,
`Tolerance::Strict` in `src/coupled.rs`) solves the follower at the old
file's leader values with zero relative gap (absolute gap and feasibility
tolerance 1e-9), giving phi*; then the optimistic tie-break (all rows, the
leader's objective minimised) with the follower's value capped at
phi* + 1e-9 (absolute; the retry with 1e-7 was never needed) and every
leader variable fixed. All 17 kept their leader values (`all-fixed`). The
new points are certified from scratch by `bobi certify --tight`: rows,
bounds and integrality within 1e-6, the follower's value at most 1e-6
(absolute) above the proven bound on its optimum, and the leader's
objective within 1e-6 + 1e-9 |F| of the proven tie-break bound.

| instance | old leader objective | new leader objective | worse by (relative) | old follower slack | new follower slack |
| --- | ---: | ---: | ---: | ---: | ---: |
| binkar10_1_0_100 | 5006810.579955 | 5006811.080028 | 0.500 (1.0e-7) | 0.500 | -4.9e-7 |
| cmflsp50-24-8-8_0_100 | 59480213.400598 | 59480214.654200 | 1.254 (2.1e-8) | 1.253 | -1.7e-7 |
| core2536-691_0_100 | 9999999000.000046 | 10000000000 | 1000 (1.0e-7) | 1000 | 0 |
| cost266-UUE_0_100 | 35801080.956898 | 35801082 | 1.043 (2.9e-8) | 1.043 | -1.1e-8 |
| exp-1-500-5-5_0_100 | 912510.912065 | 912510.999999999 | 0.0879 (9.6e-8) | 0.0879 | 1.3e-9 |
| ic97_potential_0_100 | 4002.9999646 | 4002.999999999 | 3.54e-5 (8.8e-9) | 3.54e-5 | 1.0e-9 |
| n4-3_0_100 | 34802892.519897 | 34802896 | 3.480 (1.0e-7) | 3.480 | 0 |
| n5-3_0_100 | 30001631.999933 | 30001635 | 3.000 (1.0e-7) | 3.000 | 0 |
| n9-3_0_100 | 50404627.959837 | 50404633 | 5.040 (1.0e-7) | 5.040 | 0 |
| neos-2978193-inde_0_100 | 61.4539947311 | 61.4540010633 | 6.33e-6 (1.0e-7) | 6.33e-6 | 6.8e-10 |
| pg5_34_0_100 | -11516.2870420 | -11516.2868595 | 1.83e-4 (1.6e-8) | 1.83e-4 | 1.0e-9 |
| pg_0_100 | -5364.2594068 | -5364.2592049 | 2.02e-4 (3.8e-8) | 2.02e-4 | 1.0e-9 |
| seymour1_0_100 | 959.5359765 | 959.5360452 | 6.87e-5 (7.2e-8) | 6.87e-5 | 1.0e-9 |
| tr12-30_0_100 | 201018.9907358 | 201018.999999999 | 9.26e-3 (4.6e-8) | 9.26e-3 | 1.0e-9 |
| uct-subprob_0_100 | 827.9999317 | 827.999999999 | 6.83e-5 (8.2e-8) | 6.83e-5 | 1.0e-9 |
| neos-873061_0_100 (archive) | 258644.394024 | 258644.419889 | 0.0259 (1.0e-7) | 0.0259 | 2.3e-10 |
| netdiversion_0_100 (archive) | 19850249 | 19850250 | 1 (5.0e-8) | 1 | 0 |

Follower slack: the file's follower value minus the proven bound on the
follower's optimum at its leader values (absolute; the follower minimises;
a small negative value is a follower value below the bound within the
feasibility tolerance). Relative: worse by / |old leader objective|. Every
new value is worse than the old one, as it must be: the old leader used the
follower's slack, and here the leader's objective moves with the
follower's, so the loss equals the old slack (to within 0.02%). All 17 are
still the first points known for their instances. No HPR-bound optimum
(`hpr_optimal.csv`: ex10_0_100, neos-1445765_0_100, pigeon-10_0_100,
timtab1_0_100 here) was among them, and none of their files changed.

An independent check (`scripts/coupled_tight_check.py`: HiGHS through
highspy, its own AUX parser, no `bobi` code, 120 s per solve; output
`tolerance/pycheck_tight.tsv`) finished on 13 of the 17 new files and
confirmed them: rows, bounds and integrality within 7.6e-10; the follower
re-solved with zero relative gap and feasibility tolerance 1e-9, slack at
most 1.3e-9; and, with the follower allowed the whole 1e-6 above its
optimum, the leader could gain at most 1.0e-6 (2.7e-6 for binkar10_1_0_100,
whose follower value sits 1.6e-6 below highspy's bound, inside the
feasibility tolerance). On cmflsp50-24-8-8, core2536-691, neos-873061 and
netdiversion the Python check did not finish within 600 s.

After the replacement all 64 files pass `bobi verify --tight`
(`../notes/tight_verify.md`). `bobi certify --tight` certifies 62 of the
64. The other two, neos-3627168-kasai_0_100 and neos-3754480-nidda_0_100
(not among the 17; follower slack 0 and 2.3e-10 under `verify --tight`),
have leader values at which the follower's problem is infeasible at
`certify --tight`'s feasibility tolerance of 1e-9 (the files meet their rows
within 3.4e-7 and 1.5e-10), so that check gets no follower bound; an
independent HiGHS check (`scripts/coupled_tight_check.py`) reports the
same. Their rows are within 1e-6 and their followers are optimal at HiGHS's
default tolerance 1e-7, so they count as passing the tight test.

`points.csv` holds the new values for the 17 and three added columns:
`tightened` (`all-fixed` for the 17, empty otherwise), `follower_slack`
(follower value minus the proven bound on its optimum, absolute, from
`bobi certify --tight`) and `tight_certified` (`yes` / `no`, the verdict of
`bobi certify --tight`; empty for rows without a solution file).

Files in `tolerance/`: the 15 superseded loose files as
`<instance>.res.json.tol` and the two superseded archive files in
`large_points.tol.tar.gz`; `tighten.jsonl`, the `bobi coupled-tighten` run
(old and new objectives, follower slack, mode, seconds);
`certify_tight.tsv` and `certify_tight_old.tsv`, `bobi certify --tight` on
the 64 current files (62 VERIFIED) and on the 17 superseded ones (all
REJECTED for the follower's slack); `pycheck_tight.tsv`, the independent
check of the 17 new files.

```sh
cd bobilib
mkdir -p /tmp/large && tar xzf results/general/large_points.tar.gz -C /tmp/large
# the 17 re-derived files: strict tie-break at the old files' leader values
# (OLD: the 15 loose .res.json.tol files and the two from large_points.tol.tar.gz)
sh scripts/tighten_coupled.sh NEWDIR 600 OLD... > results/general/tolerance/tighten.jsonl
# tight certificate of every file, then the points.csv columns from it
for f in results/general/*.res.json /tmp/large/*.res.json; do n=$(basename $f .res.json)
  ./target/release/bobi certify --tight $(find data/collection -name $n.mps.gz) $f --follower-time 600
done | sort > results/general/tolerance/certify_tight.tsv
python3 -I scripts/coupled_tighten_summary.py results/general/points.csv \
    results/general/tolerance/tighten.jsonl results/general/tolerance/certify_tight.tsv
./target/release/bobi verify --instances data/collection --tight --time-limit 1200 results/general /tmp/large
# the independent check of one new file (HiGHS via highspy, no bobi code)
python3 -I scripts/coupled_tight_check.py 120 INSTANCE.mps.gz INSTANCE.aux results/general/pg_0_100.res.json
```
