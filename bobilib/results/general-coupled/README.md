# General MIPLIB-derived instances with coupling rows

Verified bilevel feasible points for BOBILib's open general instances whose
leader rows contain follower variables (coupling rows). For these instances
the usual pipeline (solve the high point relaxation (HPR), fix the leader,
solve the follower, break ties optimistically) often fails: every follower
optimum at the HPR's leader values can violate a coupling row. `bobi
coupled` searches for leader values where some follower optimum meets
them (see `src/coupled.rs` and the main README).

## Candidates

BOBILib lists 87 instances with status `open` and at least one coupling
constraint (`collection.json`). Left out:

* 23 whose follower is unbounded for every leader choice (`unbounded` in
  `runs/ray.tsv` of the main checkout), so no bilevel feasible point exists;
* 10 with more than 100,000 variables.

That leaves 54, run smallest first (by number of variables). BOBILib lists
no objective value for any of them.

## Results

| | instances |
| --- | --- |
| candidates | 54 |
| attempted | 52 |
| verified feasible point (all of them the first known point) | **29** |
| no point within the time limit | 23 |
| not attempted (batch budget spent) | 2: neos-4722843-widden_50_50, square47_50_50 |

Which method found the point (the portfolio stops at the first one):

| method | points | instances it ran on |
| --- | --- | --- |
| `hpr`: the HPR's own leader values | 21 | 52 |
| `penalty`: HPR with F + rho f, rho from 0.1 to 1000 times a balancing scale, then f first | 5 | 28 |
| `follow`: best-response dynamics | 3 | 31 |
| `padm`: penalty alternating direction (Kleinert and Schmidt 2021) | 0 | 22 |
| `perturb`: continuous linking variables moved into the interior | 0 | 18 |
| `vcut`: value-function cuts | 0 | 20 |
| `nogood`: no-good cuts on integer linking variables | 0 | 13 |

So the HPR's leader values already work on 21 of 52 instances; on 8 of the
other 31 (where all follower optima at the HPR's leader values violate a
coupling row) the penalty HPR or best-response dynamics found a different,
unblocked leader choice. The cut-based methods (`vcut`, `nogood`) and PADM
found nothing within their share of the budget: on these instances each
master solve is a full MIPLIB-size MILP, so they get through only a few
iterations.

Why the 23 others have no point:

* 14: the HPR's leader values are blocked and no other method found
  unblocked ones (neos-3754480-nidda_50_50 also failed with 600 s);
* 6: HiGHS found no HPR point at all within the master cap (90 s; for
  momentum1, ex10, co-100, square41, satellites2-60-fs, neos-3216931-puriri
  even the single-level MIPLIB problem is hard);
* 3: the follower problem at the HPR's leader values was not solved in time
  (neos-4387871-tavua, neos-5052403-cygnet, sing44).

No instance was proven to have no bilevel feasible point (`vcut` and
`nogood` can prove that when their cuts are exact, but did not finish).

**pigeon-10_50_50 is optimal.** Its HPR is exactly MIPLIB's pigeon-10
(checked with `scripts/compare_hpr.py`), whose optimum is -9000, a lower
bound on the bilevel optimum; the point here has value -9000.
(timtab1_50_50's HPR is also exactly timtab1, optimum 764,772; the point
here has 764,774, so it is not proven optimal. HiGHS reported the HPR
"optimal" at 764,774, a numerical failure on this model.)

| instance | vars | coupling rows | linking vars (integer) | method | s to point | leader objective |
| --- | --- | --- | --- | --- | --- | --- |
| mik-250-1-100-1_50_50 | 251 | 76 | 50 (49) | hpr | 45 | 890671 |
| mik-250-20-75-4_50_50 | 270 | 75 | 37 (17) | hpr | 39 | 663799 |
| timtab1_50_50 | 397 | 86 | 105 (0) | hpr | 141 | 764773.9999998 |
| pigeon-10_50_50 | 490 | 277 | 179 (149) | hpr | 180 | -9000 (optimal) |
| ran16x16_50_50 | 512 | 112 | 144 (0) | hpr | 166 | 43000 |
| neos17_50_50 | 535 | 25 | 50 (0) | hpr | 19 | 0.687199999 (s) |
| sp150x300d_50_50 | 600 | 75 | 225 (0) | hpr | 0 | 300 |
| ic97_potential_50_50 | 728 | 455 | 330 (143) | penalty | 146 | 4197 (s) |
| rmine6_50_50 | 1096 | 1112 | 548 (548) | follow | 91 | -153.749802 |
| seymour1_50_50 | 1372 | 1668 | 256 (45) | hpr | 90 | 960.6331705 (s) |
| neos-4954672-berkel_50_50 | 1533 | 532 | 378 (0) | hpr | 90 | 8861094 |
| uct-subprob_50_50 | 2256 | 891 | 1128 (379) | hpr | 90 | 847.999999999 (s) |
| p200x1188c_50_50 | 2376 | 494 | 694 (0) | hpr | 3 | 3584430 |
| beasleyC3_50_50 | 2500 | 375 | 875 (0) | hpr | 90 | 6844 |
| n5-3_50_50 | 2550 | 484 | 1275 (0) | hpr | 90 | 30001775 (s) |
| pg5_34_50_50 | 2600 | 65 | 1300 (100) | penalty | 138 | -11591.16992 |
| pg_50_50 | 2700 | 63 | 362 (62) | follow | 49 | -6061.068071 (s) |
| neos-1337307_50_50 | 2840 | 2844 | 1420 (1420) | hpr | 94 | -201272 |
| mc11_50_50 | 3040 | 560 | 960 (0) | hpr | 90 | 128960 |
| n4-3_50_50 | 3596 | 618 | 1798 (0) | penalty | 150 | 50621410 (s) |
| neos-950242_50_50 | 5760 | 6724 | 1200 (1200) | follow | 92 | 24 |
| n9-3_50_50 | 7644 | 1182 | 3822 (0) | penalty | 154 | 74423855 (s) |
| mushroom-best_50_50 | 8468 | 396 | 4234 (4121) | hpr | 91 | 4230.055334 |
| neos-5188808-nattai_50_50 | 14544 | 8707 | 3818 (0) | hpr | 91 | 8.15560177 |
| neos-1445765_50_50 | 20617 | 1074 | 10309 (0) | penalty | 152 | 0 |
| neos-2978193-inde_50_50 | 20800 | 72 | 2336 (0) | hpr | 100 | 69.27508326 (s) |
| neos-934278_50_50 | 23123 | 3432 | 10638 (8394) | hpr | 151 | 32490 |
| neos-933966_50_50 | 31762 | 3125 | 15121 (12101) | hpr | 138 | 47506 |
| neos-827175_50_50 | 32504 | 6920 | 16078 (5098) | hpr | 22 | 122.00152 |

"90 s" points are HPR incumbents at the master cap (0.3 of the budget):
the HPR need not be solved to optimality, any HPR point whose leader values
are unblocked gives a bilevel feasible point.

(s): the point was re-derived under the strict follower tolerance after an
external review (see "Strict tolerance" below); method and time are those
of the search that found the original leader values.

## Verification

Every point is certified inside the search and again from the file:

* `bobi certify` (`scripts/certify_dir.sh`): rows, bounds and
  integrality within 1e-6; a fresh follower solve whose proven dual bound
  puts the point's follower value within a relative 1e-5 of the optimum
  (a 10x margin on BOBILib's 1e-4); and the optimistic tie-break (the
  leader's best objective over the follower's optima at these leader
  values, subject to all rows including the coupling rows) also within a
  relative 1e-5, by a proven bound. 29 of 29 pass.
* `bobi verify` (an independent check written like BOBILib's feasibility
  checker): 29 of 29 pass.

An independent review re-checked all 29 files twice more. The first check
was `bobi verify --generic --time-limit 600`. The second read the MPS files
with HiGHS 1.15 (highspy) instead of `mps.rs`, parsed the AUX files itself,
and solved each follower again. All 29 pass both checks. Every follower
problem solves to proven optimality within a second.

**Tolerance note (superseded).** The search's best response let the
follower's value sit up to 1e-7 (1 + |phi|) above its optimum, well inside
BOBILib's relative 1e-4, and the leader's objective used that slack. In 9 of
the 29 files the follower's value was about 1e-7 (relative) above the
optimum: ic97_potential, seymour1, uct-subprob, n4-3, n5-3, n9-3, neos17, pg
and neos-2978193-inde (all `_50_50`). Those files were feasible only within
the tolerance, and an external review of the paper asked for them to be fixed
or classified separately. They are kept, unchanged, as
`tolerance/<name>.res.json.tol`.

## Strict tolerance

The nine files were replaced by points whose follower response is optimal
to a tight tolerance: `bobi coupled-tighten` (`scripts/tighten_coupled.sh`,
`Tolerance::Strict` in `src/coupled.rs`) solves the follower at the old
file's leader values with zero relative gap (absolute gap and feasibility
tolerance 1e-9), giving phi* (its value, or its proven bound where
rounding the integers leaves the value below the bound); then the
optimistic tie-break (all rows, the leader's objective minimised) with the
follower's value capped at phi* + 1e-9 (absolute; the code retries with
1e-7 when the solver finds no point with 1e-9, which none of the nine
needed), every leader variable fixed. For pg and neos-2978193-inde
no such response meets the coupling rows at the files' own leader values
(the coupling rows force the follower 1.0e-7 relative above its optimum
there), so only the linking variables (the leader variables in follower
rows) stay fixed and the other leader variables are re-optimised in the same
tie-break; the follower's problem, and so phi*, is unchanged. The result is
certified from scratch by `bobi certify --tight`: rows, bounds and
integrality within 1e-6, the follower's value at most 1e-6 (absolute) above
the proven bound on its optimum, and the leader's objective within 1e-6 +
1e-9 |F| of the proven tie-break bound (tie-break over follower values at
most the point's own, solved to relative gap 1e-9).

| instance | old leader objective | new leader objective | worse by (relative) | old follower slack | new follower slack | leader values |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| ic97_potential_50_50 | 4196.9999407 | 4197 | 5.93e-5 (1.4e-8) | 5.93e-5 | 0 | all fixed |
| seymour1_50_50 | 960.6331017812 | 960.6331704802 | 6.87e-5 (7.2e-8) | 6.87e-5 | 1.0e-9 | all fixed |
| uct-subprob_50_50 | 847.9999317 | 847.999999999 | 6.83e-5 (8.1e-8) | 6.83e-5 | 1.0e-9 | all fixed |
| n4-3_50_50 | 50621405.7289484 | 50621410 | 4.27 (8.4e-8) | 4.27 | 0 | all fixed |
| n5-3_50_50 | 30001771.9999194 | 30001775 | 3.00 (1.0e-7) | 3.00 | 0 | all fixed |
| n9-3_50_50 | 74423848.7588194 | 74423855 | 6.24 (8.4e-8) | 6.24 | 0 | all fixed |
| neos17_50_50 | 0.6871998436 | 0.6871999990 | 1.55e-7 (2.3e-7) | 1.56e-7 | 1.0e-9 | all fixed |
| pg_50_50 | -6061.0720761766 | -6061.0680710009 | 4.01e-3 (6.6e-7) | 5.77e-5 | 1.0e-9 | linking fixed |
| neos-2978193-inde_50_50 | 69.2749846519 | 69.2750832560 | 9.86e-5 (1.4e-6) | 7.06e-6 | 1.0e-9 | linking fixed |

Follower slack: the file's follower value minus the proven bound on the
follower's optimum at its leader values (absolute; the follower minimises).
Every new value is worse than the old one, as it must be: the old leader
used the follower's slack. The loss is about 1e-7 relative for the seven
files with fixed leader values (the follower's slack times the leader's
price for it) and 6.6e-7 and 1.4e-6 relative for pg and neos-2978193-inde,
whose other leader variables had to move. The old neos17 file already met
the tight test (slack 1.6e-7 < 1e-6); it was re-derived anyway. All nine
are still the first known points for their instances.

An independent check (`scripts/coupled_tight_check.py`: HiGHS 1.15 through
highspy, its own AUX parser, no `bobi` code; output
`tolerance/pycheck_tight.tsv`) confirmed the nine new files: rows, bounds
and integrality within 2.3e-11; the follower re-solved with zero relative
gap and feasibility tolerance 1e-9, slack at most 1.0e-9; and, with the
follower allowed the whole 1e-6 above its optimum, the leader could gain at
most 1.0e-6.

After the replacement all 29 files pass `bobi certify --tight` and
`bobi verify --tight` (follower within max(1e-6, 1e-9 |phi|) of its
proven optimum; `../notes/tight_verify.md`).

## Files

* `<instance>.res.json`: the point in BOBILib's solution format (status
  `feasible`).
* `summary.csv`: one row per candidate: sizes, status (`point`, `none`,
  `not_attempted`), method, time to the point, leader and follower
  objectives, certification errors, and how each method ended; for the nine
  re-derived files the objectives, errors and `free_leader` are the new
  point's. Added columns: `tightened` (`all-fixed` / `linking-fixed` for the
  nine, empty otherwise), `follower_slack` (follower value minus the proven
  bound on its optimum, absolute, from `bobi certify --tight`) and
  `tight_certified` (`yes` / `no`).
* `tolerance/<instance>.res.json.tol`: the nine superseded files that used
  the follower tolerance; `tolerance/tighten.jsonl`: the `bobi
  coupled-tighten` run that replaced them (old and new objectives, follower
  slack, mode); `tolerance/certify_tight.tsv` and
  `tolerance/certify_tight_old.tsv`: `bobi certify --tight` on the 29
  current files (all VERIFIED) and on the nine old ones (8 REJECTED for
  the follower's slack; neos17's old file passes); `tolerance/pycheck_tight.tsv`:
  the independent check of the nine new files.

## Runs and reproduction

The batch ran single-threaded (HiGHS with 1 thread), one instance at a
time, `--follower-time 120`, on a machine shared with four other long jobs.
The first 8 candidates ran with 600 s per instance; after a container
restart the other 46 ran with 300 s per instance and a 2.5-hour batch
budget (`BUDGET=9000`), which stopped before the last 2. Raw per-instance
lines: `runs/coupled.jsonl`; method-by-method log: `runs/coupled.err`
(`runs/` is not committed).

```sh
cd bobilib
cargo build --release
# candidates: open, coupling rows, follower not unbounded everywhere, <= 100k variables
# (RAY_TSV is the output of `bobi ray` on the general instances)
python3 -I scripts/coupled_candidates.py path/to/collection.json RAY_TSV data > runs/coupled.txt
BUDGET=9000 sh scripts/coupled.sh runs/coupled.txt runs/coupled.jsonl runs/coupled.err 300 --follower-time 120
python3 -I scripts/coupled_summary.py path/to/collection.json results/general-coupled/summary.csv runs/coupled.jsonl
sh scripts/certify_dir.sh results/general-coupled 300
./target/release/bobi verify --instances data/collection results/general-coupled --time-limit 300
# the nine re-derived files: strict tie-break at the old files' leader values
sh scripts/tighten_coupled.sh results/general-coupled 600 results/general-coupled/tolerance/*.res.json.tol \
    > results/general-coupled/tolerance/tighten.jsonl
# tight certificate of every file, then the summary columns from it
for f in results/general-coupled/*.res.json; do n=$(basename $f .res.json)
  ./target/release/bobi certify --tight $(find data/collection -name $n.mps.gz) $f --follower-time 600
done > results/general-coupled/tolerance/certify_tight.tsv
python3 -I scripts/coupled_tighten_summary.py results/general-coupled/summary.csv \
    results/general-coupled/tolerance/tighten.jsonl results/general-coupled/tolerance/certify_tight.tsv
# tight verification, and the independent check of one file (HiGHS via highspy, no bobi code)
./target/release/bobi verify --instances data/collection --tight results/general-coupled --time-limit 1200
python3 -I scripts/coupled_tight_check.py 600 INSTANCE.mps.gz INSTANCE.aux results/general-coupled/pg_50_50.res.json
# a whole search under the strict tolerance
./target/release/bobi coupled FILE.mps.gz --strict --time 300 --follower-time 120 --out DIR
# one instance, all methods, with a log on stderr
./target/release/bobi coupled data/collection/general-bilevel/mixed-integer/miplib2017/ic97_potential_50_50.mps.gz \
    --time 300 --follower-time 120 --log --out results/general-coupled
```
