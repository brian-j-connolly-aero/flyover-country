# Lefebvre and Schmidt's results checked against this work

H. Lefebvre and M. Schmidt, "A Dantzig-Wolfe single-level reformulation for
mixed-integer linear bilevel optimization: exact and heuristic approaches",
Optimization Online (posted 12 June 2025; the PDF there is dated 20 July 2026, 25 pages,
and its text is identical to the copy read here). Page numbers below are the
PDF's. The per-instance data are the run files on the first author's
open-science page for the paper: `results/results-{MILP,MILP-first,PADM,PADM-interdiction}.csv`,
`to_check.csv`, `bobilib.csv` (their BOBILib snapshot of 5 June 2025) and
`methodology.render.Rmd` (their analysis script). All of it was read as data.

Column meanings come from the authors' script:
MILP files `tag, instance, method, time, status, reason, best_obj, best_bound, lambda, pi_lb, pi_ub`;
PADM files `..., time, status, reason, best_obj, outer_loops, inner_loops, lambda, pi_lb, pi_ub`.
`lambda, pi_lb, pi_ub` are the dual-variable bounds of their Proposition 3 / Remark 2
(p. 9; for ex10: 8840 / -17680 / 8840, that is M = u - l, 2l - u, 2u - l with l = -8840, u = 0).
`status/reason` are idol's mapping of the solver status (Gurobi OPTIMAL becomes
`Optimal/Proved`). The script caps every time above 3600 s at 3600, sets the time of
every non-`Feasible` PADM run to 3600, and counts a run as solved or as having found a
point only if its time is below 3600; the status column of the MILP files is not used.

Per-row evidence for everything below: `lefebvre_schmidt_discrepancies.tsv` (this
folder; one row per run-file row, with file and line number, the contradicting
result, our status, and where the authors' analysis uses the row).

## Summary

* **ex10_0_100**: their exact MILP reports `Optimal/Proved`, value 100, bound 100, at
  624 s. It is correct: the instance's HPR is exactly MIPLIB's ex10 (checked now with
  `scripts/compare_hpr.py`), whose published optimum is 100, and our verified point has
  value 100. The paper never names the instance and claims no optimum for any open
  instance; it counts it only as a feasible point found for an open instance.
* **Value 0 on OR instances**: 366 OR instances get an `Optimal/Proved` value of 0 (387
  get a 0 in some row). Every such row ends at the time limit (3600.0 to 3629.4 s) with
  bound 0. An elementary exact bound gives at least 568 on every one of them. The 0 is not
  a status code, a default value, a sign convention or an offset. It is the optimum of
  the high point relaxation (y = 0), which was accepted as the incumbent when the
  bilevel-feasibility separation did not finish before the time limit. The authors'
  analysis discards these rows (time 3600 means "not solved"), so Tables 2 and 3 and
  Figures 1 to 5 are not affected. Figure 6 takes the best value of any run without a
  time filter, and its 20 % "best" share for MILP-First on interdiction instances is
  very likely an artefact of these zeros.
* **highschool1-aigio_0_100**: the PADM point (value 0, `Feasible/Proved`, both penalty
  settings) cannot be bilevel feasible. Our exact recession direction (re-checked now)
  shows the follower has no optimal response for any leader choice. The "infinite
  multipliers" are the dual-variable bounds of Remark 2, not a diverging penalty. The
  paper does not name the instance, but it counts it among the 48 open instances where
  PADM "found feasible" (Table 2) and among the abstract's "66 instances that were still
  open".
* **Further problems in the run files that do reach the paper**: 91 instances with a
  known point are reported `Infeasible/Proved` (in 0.01 to 0.4 s for the 89 generalized
  ones, 23 to 300 s for mas74_0_100 and mas76_0_100), and 2 reported optima are worse
  than verified points. All 93 count as "solved" in the paper's "12 %" (p. 17). Table 3
  counts 102 MILP-First runs that ended with status `Infeasible` as "found feasible"
  (3 more enter only its total and Figure 3). 11 of them are among the abstract's 66.

## (1) ex10_0_100

Run files (results/):

```
results-MILP.csv:682        ex10_0_100,MILP,623.746,Optimal,Proved,100,100,8840,-17680,8840
results-MILP-first.csv:683  ex10_0_100,MILP-first,623.461,Optimal,Proved,100,100,8840,-17680,8840
results-PADM.csv:505,510    ex10_0_100,PADM,...,3600.2,Infeasible,TimeLimit,1e+20,...   (no point)
bobilib.csv:2372            ex10_0_100,...,None,open,hard
```

Both MILP runs end well before the limit, with finite dual-variable bounds and
bound = value. This is unlike the time-limit artefacts of section (2).

Independent evidence: `python3 -I scripts/compare_hpr.py ex10_0_100.mps.gz ex10_0_100.aux ex10.mps.gz`
(MIPLIB 2017 file, downloaded 2026-10-19) prints `HPR == MIP`. MIPLIB 2017 lists ex10 as
`=opt= 100` (miplib2017-v31.solu). The objective is the all-ones vector on 17,680 binaries,
so every value is an integer. 100 is therefore a lower bound under the same convention
as noswot (a published MIPLIB optimum, safe by integrality rounding if the published
bound is valid). Our verified point `general/ex10_0_100.res.json` has value 100
(`general/points.csv`: upper 100, lower -51). So 100 is optimal by the HPR argument, and
their run files agree.

The paper does not claim this optimum. ex10 is not named anywhere. Optimality claims
appear only as "found optimal (without proof)" for instances BOBILib lists as optimal
(Tables 2 and 3, p. 20). ex10_0_100 (and ex10_50_50) enter Table 3 only under "open:
found feasible 43", the abstract's 66, and the MILP "solved" share of Figure 1.

ex10_50_50 (coupling rows): their MILP and MILP-First runs also report `Optimal/Proved`
100 (677 s, 683 s; lines 689 and 691). `compare_hpr.py` again gives `HPR == MIP`, so
100 is a lower bound. We have no point there (`bobi coupled` found none), and their point
is unverified.

## (2) MILP runs reporting "optimal" value 0 on OR instances

The authors' test set has all 810 OR instances. Their MILP run never ends before the
limit on any of them: 441 end `Infeasible/TimeLimit` (no incumbent), 307 end
`Optimal/Proved` with value 0 and bound 0, and 62 end `Feasible/TimeLimit`. MILP-First
adds more zeros. In total 387 OR instances have a 0 in some row: 307 MILP `Optimal`,
305 MILP-First `Optimal`, 16 MILP and 7 MILP-First `Feasible/TimeLimit`, 635 rows in
all. 366 of these instances have an `Optimal` 0. The full list is in the TSV
(category `value 0 below proven lower bound`).

What contradicts the 0:

* An exact single-item bound computed here from the instance files. If the leader
  cannot interdict every follower item j that fits the follower's knapsacks alone and
  has p_j >= t (those items exceed a leader budget row), the follower can still take one
  of them, so the value is at least t. This gives at least 568 on all 387 instances.
* Lozano, Bergman and Cire's published lower bounds (`or/compare.csv` lit_lb), available
  for 224 of the 366, minimum 974. Our own lower bounds exist for 36 of them.
* Our verified points (`or/*.res.json`) for 268 of the 366, minimum 7,334. These are
  upper bounds and do not by themselves contradict a 0.
* The authors' own `to_check.csv` lists 227 of the 366 as `KO, not bilevel feasible`.
  Example: `results-MILP.csv:4 OR5x250-0.25_3.dat-0,MILP,3600.04,Optimal,Proved,0,0,64144,...`
  against the published lower bound 35,700 (and BOBILib's point 56,308).

What the 0 is. All 635 rows end at or just after the limit, and all have bound 0. The
0 is the optimum of the problem their branch and cut solves before any cut is added.
The node problem (11) of their Algorithm 1 (p. 11), with Z^0 empty, is the HPR plus the
McCormick system. For packing interdiction (leader objective p^T y with p >= 0), its
optimum is y = 0 with value 0. Lines 13-17 of Algorithm 1 accept an integer point as
the incumbent unless the separation problem yields a violated cut. The paper separates
"inside the Gurobi callback triggered by the MIPSOL event" (p. 15). In idol's generic
lazy-cut callback (`lib/src/mixed-integer/optimizers/callbacks/CutSeparation.cpp`,
commit 8f77367), the separation MILP receives the remaining time as its limit (line 43).
If it stops without a solution, the callback returns without a cut (line 48), so Gurobi
keeps the point. Its value 0 then equals the bound 0, Gurobi reports OPTIMAL, and idol
maps that to `Optimal/Proved` (`Optimizers_Gurobi.cpp` line 227). Whether the authors'
unpublished driver uses exactly this callback is not known. The rows match this
behaviour exactly. The nonzero time-limit values below proven bounds (section 4) are
presumably the same effect after some cuts. The value is therefore that of a relaxation,
not of a bilevel feasible point.

Does it reach the paper? Not the tables. The script turns these times into 3600 = "not
solved / no point", so Figures 1 to 5 and Tables 2 and 3 exclude them, and the paper's
text never mentions a 0. The issue is the CSV's status column read at face value, which
is what our literature compilation did for three nonzero values (section 4). The
exception is Figure 6 (p. 19), which uses "the best known feasible point computed by any
exact or heuristic method". The script computes it as `min(best_obj)` over all runs,
with no time or status filter. Rerunning that computation on the posted files over all
instances gives MILP-First "best" on 396 of 1,958 interdiction instances (20.2 %; the
paper says 20 %). 319 of those 396 are time-limit zeros. Without the time-limit MILP and
MILP-First values the share is 3.5 %, and MibS-First rises from 31.1 % to 38.0 % (the
paper says 35 %). The posted script itself restricts Figure 6 to instances where
MibS-First needs at least 120 s. There, MILP-First is "best" on 63.4 % of 426
interdiction instances, and on 0 % without the zeros. Either way most of the MILP-First
share in Figure 6a comes from the zeros.

## (3) highschool1-aigio_0_100

```
results-PADM.csv:789  highschool1-aigio_0_100,PADM,adapt,2,100,3298.97,Feasible,Proved,0,1,2,1e+20,-2e+20,1e+20
results-PADM.csv:794  highschool1-aigio_0_100,PADM,adapt,2,1000,3239.94,Feasible,Proved,0,1,2,1e+20,-2e+20,1e+20
results-MILP.csv:837  highschool1-aigio_0_100,MILP,3600.03,Infeasible,TimeLimit,1e+20,1e+20,1e+20,-2e+20,1e+20
```

Our certificate `rays/highschool1-aigio_0_100.ray.json` (d = 1 on `z` and on
`f(LimitBusyTimesConstraint_T0)`, d = 1/2 on `s16(LimitBusyTimesConstraint_T0,T0,Mo)`) was
re-checked today with `python3 -I scripts/verify_ray.py ...`:
`{"verified": true, "objective_along_d": -1.0, "support": 3, "problems": []}`, in exact
rational arithmetic. For every leader choice the follower is infeasible or unbounded
(Meyer 1974), so it has no optimal response and no bilevel feasible point exists.

What PADM's output means:

* `Feasible/Proved` in idol's PADM (`lib/src/mixed-integer/optimizers/padm/Optimizers_PADM.cpp`,
  `is_feasible` lines 308-317 and `check_feasibility` lines 511-518) means the penalised
  constraint violations are within tolerance. That makes the point feasible for the
  single-level model built so far, and the paper calls such points "stationary points of
  the single-level problem" (p. 5). That model equals the bilevel problem only under
  Assumption 1 (p. 6: "the follower's feasible region ... is bounded for all x"), which
  this instance violates.
* The "infinite multipliers" are not a diverging penalty. `outer_loops = 1` and
  `inner_loops = 2` show that PADM stopped after two iterations at its initial penalty.
  The triple `1e+20, -2e+20, 1e+20` is the Remark 2 bound triple (u - l, 2l - u, 2u - l)
  with l = -infinity. Their bound computation found the follower's objective unbounded
  below over the HPR, which is what our direction (f^T d = -1) implies.
* The value 0 equals MIPLIB's published optimum of highschool1-aigio, consistent with
  PADM returning an HPR solution whose follower part is not optimal.

Does the paper claim it? Not by name. It is counted in Table 2 (p. 20, "open 285 / found
feasible 48"), in the sentence "PADM computes a feasible point for 48 instances, which
were open in BOBILib" (p. 20), in the abstract's 66, and in Figures 3, 4b and 6. In their
own data the instance has infinite dual-variable bounds. The same holds for 102 other
test-set instances: all 90 generalized ones (follower variables with no upper bound, so
u = +infinity) and 12 MIPLIB-derived ones, for every one of which `rays/` holds a
recession direction. This conflicts with the test-set description "all problems in which
the linking variables are binary and the shared constraint set is bounded" (p. 15) and
with "have all variables explicitly bounded" (p. 3).

## (4) All run files against `instances.csv`

All rows of the four result files were scanned (MibS.csv and filemosi.csv are BOBILib's
own runs; their only differences are print rounding to 6 digits).

| Kind | Instances (rows) | Example (file:line) | Contradicted by | Artefact or paper |
| --- | ---: | --- | --- | --- |
| value 0 below a proven bound, at the time limit | 387 OR + 26 other (661) | MILP:4 OR5x250-0.25_3.dat-0, 0 | single-item bound >= 568; Lozano et al.'s bounds; published or BOBILib optima (KIP, clique, imkp, network) | run file; Fig. 6 only |
| nonzero value below a proven bound, at the time limit | 37 (38) | MILP:985 macrophage_0_100 "Optimal" 374 < 695; MILP:1705 OR10x500-0.25_5.dat-50 20,909 < 67,175 | published lower bounds and optima | run file; Fig. 6 only |
| `Infeasible/Proved` where a point is known | 91 (183) | MILP:597 general20-20-10-20-20-1, 0.04 s, bounds 1e+20 | BOBILib's solution files: 90 pass `bobi verify`, general20-30-20-20-20-2 only BOBILib's tolerance | paper: Figs 1-2 "solved", Table 3 "found feasible" (86 + 5) |
| `Optimal` worse than a verified point | 2 (2) | MILP:253 2AP05-2, -37 vs -46; MILP:609 general20-20-20-20-20-2, 299,882 vs 99,941 | BOBILib's solution files (pass `bobi verify`) | paper: Figs 1-2 "solved" |
| point where no bilevel feasible point exists | 1 (2) | PADM:789/794 highschool1-aigio_0_100, 0 | exact recession direction | paper: Table 2, abstract's 66 |
| `Infeasible` run counted as a found point | 14 (14) | MILP-first:52 trento1_50_50 | none (11 agree with our rays, 3 with BOBILib) | paper: Table 3 "open: found feasible" includes 11 |
| point rejected by their own `to_check.csv`, but counted | 3 (3) | MILP-first:1057 mine-166-5_50_50, "Optimal" -566,390,000 at 1,776 s | their checker (KO) | paper: Table 3, abstract's 66 |
| point on an instance BOBILib lists infeasible, at the limit | 8 (8) | MILP-first:876 air04_50_50 | BOBILib's status (solver runs) | run file only |

The values below proven bounds are confined to rows that hit the limit. No run that ended
before the limit gives a value below a proven bound, and no `Feasible` PADM or
PADM-Interdiction value lies below one.

Recounted with the authors' own time filter plus the status column:

* "MILP ... find a global solution to 12 % of the test set" (p. 17). 255 runs ended
  before the limit (11.5 %; their `ecdf_time_all.csv` gives 0.115072 = 255/2216). 105 of
  them are `Infeasible` results, and 91 of those are contradicted, as are 2 optima.
  At most 162 (7.3 %) are not contradicted.
* Table 3 (p. 20). "found feasible 43" for open instances consists of 29 `Feasible`
  and 3 `Optimal` runs plus 11 `Infeasible` runs. "Non-improving 360" includes 5
  `Infeasible` runs and "found feasible 652" includes 86. The rows sum to 1,082, but the
  printed total is 1,055 (47.6 %). Their exported data give 1,085/2,216 = 48.96 %. Runs
  that actually returned a point before the limit: 980 (44.2 %).
* Abstract, "feasible points for 66 instances that were still open". The 66 are the union
  of PADM-1000 (48) and MILP-First (43) on open instances. 11 of them are MILP-First
  `Infeasible` runs, 1 is highschool1-aigio_0_100 and 1 is mine-166-5_50_50 (KO in
  their own check). That leaves at most 53.
* Not reproducible from the posted files: "improve the best known solution for 427
  instances" (p. 3) against "12 instances" (abstract; Table 2 gives 12, reproduced
  exactly).
* Figure 5 and its text (p. 18): "60 % of the feasible points returned by PADM are below
  2 % of optimality gap and more than 80 % are below 10 % ... 55 % of the points found
  by MILP-First are below 2 %". The script's gap is a fraction,
  `(best_obj - best_known_obj)/(1e-10 + |best_known_obj|)`, plotted on an axis labelled
  "%". The quoted shares are those at fractions 2 and 10, that is 200 % and 1,000 %
  (exported `ecdf_gap_bobilib.csv`: PADM 0.609 at 2, 0.869 at 10; MILP-First 0.568 at 2).
  At a true 2 % they are 8.1 % (PADM) and 3.3 % (MILP-First). This looks like a unit
  slip in the paper, independent of the run-file artefacts.

## Recommendations

For this paper and data package:

1. ex10_0_100: classify as optimal at 100 by the HPR (MIPLIB) bound, as the sixth
   HPR-bound optimum, in the same certificate class as noswot. Record that Lefebvre and
   Schmidt's MILP run file reports the same optimum, although their paper does not state
   it. This changes headline counts by one each: proven optimal here +1 (HPR-bound
   optima 5 to 6), first verified points for general instances without coupling -1,
   optimal 608 to 609, settled 648 to 649. An alternative is to count it as "already
   optimal in published work" on the strength of the run file. I advise against that,
   because the same `Optimal/Proved` column is wrong on 366 OR instances. ex10_50_50:
   record the HPR lower bound 100 and their unverified run-file value 100. It stays
   unchanged without a verified point.
2. OR published points: use Lefebvre and Schmidt's run-file values only from rows that
   ended before the time limit, which are the rows their own analysis counts. The
   current `literature/mkip_literature_status.csv` takes 97 of its 629
   `lefebvre_schmidt_ub` values from time-limit rows (26 OR, 71 imkp). For 6 OR
   instances such a value is the best published point. Three of them are the
   "inconsistent" ones (20,909, 16,951, 10,859). The text should no longer say that "at
   least one of the two published values is wrong, and the points of this work do not
   decide which". These values come from rows of the same kind as the 366 impossible
   zeros, and the authors themselves do not count those rows as results. If only those
   three are dropped: points strictly better than every published value 469 to 471,
   worse 61 to 59, inconsistent 4 to 1 (the OR10x100-0.75_4.dat-0 closure). If all
   time-limit values are dropped: 474 better, 56 worse. The paired medians should be
   recomputed either way.
3. Small correction: neos-631710_0_100's published 556 is in the MILP-First file
   (242.9 s; MILP reaches 556 only at the limit). `build_data_package.py` says "MILP run
   files".
4. When citing their work, cite the method and avoid their headline counts (66, 12, 427,
   the 12 % solved share and the Figure 5 gap shares) unless qualified as above. Nothing
   in our results depends on them.

For how their results should be read: the paper's theory is not in question here, and
most of its runs are consistent with everything proven in this package. What needs
care is the run files: a time-limit row's `Optimal/Proved` or `Feasible` value (MILP,
MILP-First) is not a bilevel result, an `Infeasible/Proved` with dual bounds 1e+20 is not
an infeasibility proof, and a PADM `Feasible` point on an instance outside Assumption 1 is
not bilevel feasible. A corrected version of Tables 2 and 3, of the abstract's counts and
of Figure 5's axis would be a matter for the authors. Contacting them with this note and
the TSV seems the right way to raise it.

## Reproduction

* Scan and table: one-off Python scripts (run with `python3 -I`) that read the run files,
  `instances.csv`, `or/compare.csv`, `literature/mkip_literature_status.csv`, the OR
  instance files (single-item bound) and the `bobi verify` output for the 93 BOBILib
  solution files concerned (`./target/release/bobi verify --instances data/collection
  DIR`: 92 pass, general20-30-20-20-20-2 fails at 3.3e-4 as in
  `general20-30-20-20-20-2.verify.tsv`). The TSV's `used_in_paper` column applies the
  authors' filters from `methodology.render.Rmd`.
* Figures 5 and 6 and Tables 2 and 3 were recomputed from the posted files with the
  script's rules. Table 2, the end values of all six curves in their
  `ecdf_time_all.csv` (MibS 0.172834, MibS-First 0.800090, MILP 0.115072, MILP-First
  0.489621, PADM-100 0.701715, PADM-1000 0.701264), and the exported Figure 5 and
  Figure 6 data were reproduced exactly.
* The TSV's `instances_csv_primal_value` is `instances.csv`'s `primal_value` (ours,
  BOBILib's or a published optimum, see its `source`), not necessarily a point of this work.
