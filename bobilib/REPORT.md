# BOBILib: what can be solved, and approaches that resolve many instances at once

*Status as of 2026-10-18. `scripts/summarize.py` regenerates every count from
the files in `results/` and `runs/`; `results/summary.json` is its output.*

## The question

[BOBILib](https://bobilib.org) (the Bilevel Optimization Benchmark Instance
Library; Thürauf, Kleinert, Ljubić, Ralphs and Schmidt, *Mathematical
Programming Computation* 2026) holds 2,654 mixed-integer bilevel linear
problems (MIBLPs): a leader decides first, a follower then solves its own
optimisation problem given the leader's decision, and the leader's
objective depends on the follower's reply. BOBILib lists 1,027 instances as
optimal, 33 as infeasible, 1,175 as feasible (a point is known, no proof)
and 419 as open (no point known). We asked which of the 1,594 feasible or
open instances can be solved or improved, and whether new approaches could
resolve many of them at once.

## Short answer

Yes. Of the 1,594:

| Outcome | Instances |
| --- | ---: |
| already proven optimal in published work (BOBILib's labels are stale) | 527 |
| proven optimal here for the first time | 81 |
| proven here to have no bilevel feasible point at all | 40 |
| first verified point here, where BOBILib knows none | 230 |
| status unchanged (many with better points, see below) | 716 |

So 648 (41%) are now settled, 608 of them optimal. On top of the status
changes, 469 instances of the OR set get a verified point strictly better
than every published value.

Two observations carry most of the weight.

1. **BOBILib's statuses come from two general-purpose bilevel solvers**
   (MibS and Fischetti et al.'s MIX++), one hour on one thread each. The
   specialised literature is far ahead: 376 knapsack interdiction, 139
   multidimensional knapsack interdiction, 4 generalized interdiction and 8
   MIPLIB-derived instances are solved in published work.
2. **Most unsolved instances share a structure that general solvers ignore.**
   1,254 of the 1,594 are *packing interdiction* games: the leader removes
   items within one or more budgets, the follower packs what is left into
   one or more knapsacks, and the game is zero-sum. Another 60 are clique
   interdiction on graphs whose density puts them in reach of extremal
   graph theory, and almost all of the 266 general instances are built
   from MIPLIB (the Mixed Integer Programming Library) problems whose
   published optima are lower bounds.

The approaches that resolve whole families rather than single instances:

| Approach | Family | Effect |
| --- | --- | --- |
| Robust subset-sum search (count formulation, online tables in threshold form, rebuilt at every node from its decided sums) | knapsack interdiction, subset-sum and even-odd classes | all 16 instances unsolved anywhere proven optimal (8 even-odd, 8 subset-sum), each in about 10 s of CPU time or less on one thread, and checked by an independent prover |
| Dualised follower linear program (LP) bracket and a merged-row online bound | multidimensional knapsack interdiction (OR set), one follower knapsack | paired median gap 24% → 4.1% (budget 25%) and 34% → 7.1% (budget 50%); 191 better lower bounds |
| Certified points from the follower LP | OR set, several follower knapsacks | all 360 instances with budget 50% or 75% certified; 88 first points anywhere |
| Extremal graph theory (Mantel 1907, Brouwer 1981) plus partition certificates | edge clique interdiction | 57 of 60 proven optimal at 3 or 4, where BOBILib lists 5 to 13 |
| Exhaustive attacks with an exact tree dynamic program (DP) | attack-protect network interdiction | both non-optimal instances proven optimal |
| Recession-ray certificates | MIPLIB-derived | 40 open instances have no bilevel feasible point (38 checked in exact arithmetic) |
| High point relaxation (HPR), then the follower, then an optimistic tie-break; a portfolio for coupling rows | MIPLIB-derived | 93 first verified points (64 without coupling rows, 29 with), 3 of them optimal |
| HPR bound = published MIPLIB optimum | MIPLIB-derived | 5 proven optimal (3 of them open, with no point known before) |

What stays hard: 3 clique instances (value 3 or 4), OR instances with
several follower knapsacks at budget 25% (120 still without a certified
point, and the best lower bounds 28-31% away), 8 generalized interdiction
instances with 20 or more linking bits, and 123 MIPLIB-derived instances.

## Family by family

### Knapsack interdiction (599 instances; 392 not optimal in BOBILib)

* **Literature.** Weninger and Fukasawa's combinatorial branch and bound
  (IPCO 2023; *Math. Programming* 210, 2025) solves 583 of the 599. BOBILib
  lists 376 of those as not optimal; at 335 of them its value is above the
  optimum (median 8%, up to 672%). `results/kip/` holds a verified solution
  file for each of the 583 (`bobi kip-import`, checked with `bobi verify`).
* **The 16 left.** All have budget index 2 or 3 and 200-500 items; 8 are in
  the subset-sum class (profit equals weight) and 8 in the even-odd class.
  Weninger and Fukasawa (16 threads, 1 h), their reimplementation of Della
  Croce and Scatamacchia, and Lozano, Bergman and Cire's decision diagrams
  (2 h) all leave them open; the best lower bounds sit 2 to 8 units below
  the incumbent, which equals the follower's capacity (or capacity minus
  one for even-odd).
* **New.** `robust_ss` decides "can the leader keep the follower below the
  capacity?" by search over *how many copies of each weight survive*
  (identical items are interchangeable for the follower, and the leader
  should remove the cheapest copies first). Its strongest rule is an online
  relaxation: the leader deletes one weight class at a time and may react
  to what the follower took from earlier classes, which only helps the
  leader, so whatever the follower can still guarantee holds in the real
  game. The guarantee is antitone in the budget, so it is stored as one
  budget threshold per (class, residual sum), and a single backward pass
  computes it for the undecided classes with the node's own decided sums as
  the set the game must end in. That is cheap enough to do at every node
  that would branch. The order of play matters a great deal: classes that
  are cheapest to delete per unit of mass go first, so the follower sees
  the leader's likely deletions before it commits, and the expensive
  classes are kept for fine adjustment. Contiguous windows of reachable
  sums and the stall analysis remain as cheaper rules. All 16 are now
  proven optimal (one thread, CPU seconds on a shared machine, where wall
  times were 2-3 times longer; "before" is the previous version of the same
  search on the same machine):

  | Instance | Optimum | Nodes | Time | Before (nodes / time) |
  | --- | ---: | ---: | ---: | ---: |
  | interKP-200-100-6-2 | 1,855 | 303 | 0.10 s | 3,926,223 / 22.77 s |
  | interKP-300-100-6-2 | 2,868 | 366 | 0.16 s | 12,302,768 / 102.59 s |
  | interKP-400-100-6-2 | 3,832 | 257 | 0.15 s | 7,217,774 / 75.29 s |
  | interKP-500-100-6-2 | 4,651 | 190 | 0.26 s | 14,290,940 / 174.28 s |
  | interKP-200-100-6-3 | 2,782 | 13,498 | 3.93 s | not finished in 2 h |
  | interKP-300-100-6-3 | 4,302 | 19,405 | 8.06 s | not finished in 3 h |
  | interKP-400-100-6-3 | 5,748 | 13,430 | 9.01 s | not run to the end |
  | interKP-500-100-6-3 | 6,976 | 10,079 | 10.04 s | not run |
  | interKP-200-100-7-2 | 1,872 | 22 | 0.06 s | 48,598 / 0.49 s |
  | interKP-300-100-7-2 | 2,894 | 27 | 0.04 s | 115,415 / 1.78 s |
  | interKP-400-100-7-2 | 3,866 | 18 | 0.05 s | 269,550 / 5.62 s |
  | interKP-500-100-7-2 | 4,694 | 1 | 0.05 s | 357,279 / 10.26 s |
  | interKP-200-100-7-3 | 2,808 | 651 | 0.10 s | 2,341,175 / 9.61 s |
  | interKP-300-100-7-3 | 4,340 | 470 | 0.20 s | 3,256,897 / 20.35 s |
  | interKP-400-100-7-3 | 5,800 | 378 | 0.17 s | 10,710,774 / 86.43 s |
  | interKP-500-100-7-3 | 7,042 | 256 | 0.15 s | 13,346,235 / 137.19 s |

  In every case BOBILib's value was already optimal; the proof is new.
  The evidence for the four index-3 proofs is independent several times
  over: `scripts/ss_check.py` (the designer's prover: own parser, plain
  search, the per-node rule written from its definition, no code shared
  with the Rust search) and `scripts/ss_judge_check.py` (a reviewer's
  prover written without reading either) confirm all 16 lower bounds; two
  other redesigns of the search, built separately (witness reuse, and
  block tables from a hitting-set view), prove 200-6-3 at 2,782 as well,
  and the witness version all four; the reviewer compared every binary
  with exhaustive enumeration on 2,990 random small instances (0
  mismatches); and every solution file passes `bobi verify`. See
  `results/ss/`. The threshold form and the per-node end set are proved
  in Lean (`lean/Bobi/Threshold.lean`, `lean/Bobi/EndTable.lean`); the range
  shortcuts and early exit inside the node-table pass are argued in the doc
  comments of `src/robust_ss.rs`. One change at a
  time on interKP-200-100-6-2 (nodes): previous search 3,926,223; same
  rules with threshold tables 3,926,223 (memory 478 to 229 MB); every band
  width 1..64 3,278,153; root tables played largest value first 1,128,566,
  or cheapest per unit of mass first 513,834; per-node tables 303. With
  100 items, the class 6 and 7 instances with budget index 4-10, which the
  previous version did not finish in two minutes, now close at the root,
  and all 20 match Weninger and Fukasawa's optima.

### Multidimensional knapsack interdiction (954; 862 not optimal)

* **imkp (144).** Fischetti, Ljubić, Monaci and Sinnl (*INFORMS Journal on
  Computing* 2019) solved all 144; BOBILib lists 52 as only feasible, with
  values up to 23% above the optimum.
* **OR (810).** Built from Chu and Beasley's multidimensional knapsacks in
  the OR-Library (Operations Research Library): 100-500 items, 5-30 rows,
  budgets 25/50/75%, and three variants (-100: one follower knapsack; -0
  and -50: several). BOBILib has no optimum. Lozano, Bergman and Cire
  (*Operations Research* 2026) proved 87 (budget 75%); 139 had no published
  point.
* **New, one follower knapsack (270).** Dualising the follower's LP gives
  U(x) = min over l >= 0 of l C + sum_j max(0, p_j - l w_j)(1 - x_j); with
  binary x the product linearises exactly, so min over x of U(x) is one
  mixed-integer linear program (MILP). A knapsack's optimum is within one
  item's profit of its LP value, which brackets the bilevel optimum.
  Weninger and Fukasawa's online relaxation, generalised to several leader
  rows by merging them with tuned multipliers and to large integer data by
  rounding in the safe direction, gives a second, usually stronger lower
  bound. With local search on the leader, all 270 get a verified point and
  191 a better lower bound than published. Median gaps over the same
  instances:

  | Budget | Literature | Now |
  | --- | ---: | ---: |
  | 25% | 24.0% | 4.1% |
  | 50% | 34.0% | 7.1% |
  | 75% | 0.14% | 0.14% |

* **New, several follower knapsacks (540).** The follower's LP at the
  minimiser of U is a valid upper bound on the optimum for every instance,
  and whenever HiGHS solves the follower exactly at that leader choice the
  point is certified. All 360 instances with budget 50% or 75% are
  certified, including the 500-item, 30-row ones with no point anywhere; at
  budget 25%, 60 of 180. Lower bounds here are the literature's, so the
  median gaps move only through better points:

  | Variant, budget | Median gap, literature | Median gap, now | No published point: first point here |
  | --- | ---: | ---: | ---: |
  | -0, 25% | 33.3% | 31.4% | 0 of 31 |
  | -0, 50% | 36.6% | 29.1% | 31 of 31 |
  | -0, 75% | 5.1% | 0.04% | 27 of 27 |
  | -50, 25% | 29.6% | 28.2% | 0 of 20 |
  | -50, 50% | 41.3% | 34.8% | 20 of 20 |
  | -50, 75% | 11.8% | 1.4% | 10 of 10 |

  (Medians are over the instances with both a published lower and upper
  bound.)

* Over all 810: 690 verified points, 469 strictly better than every
  published value (BOBILib's included), 88 the first anywhere
  (`results/or/compare.csv` per instance, next to the solution files).
* One closure: for OR10x100-0.75_4.dat-0 the published lower bound 8,814
  comes from runs its authors marked numerically inconsistent; our point
  8,813 (verified with SCIP) meets their branch-and-cut bound of 8,813.
* Tried and dropped: an interdiction-cut loop (the leader's MILP with one
  cut per follower packing) stalls far below the published bounds (4,515
  against 8,747 after 600 s on OR10x100-0.75_3.dat-0).

### Edge clique interdiction (220; 60 not optimal)

The leader deletes at most k edges (a quarter of them) and the follower
takes a maximum clique of what remains. The 60 `clique` instances (40-60
vertices, density 0.7 or 0.9) are unsolved in the literature, and BOBILib's
values are 5 to 13. With m edges, m - k remain after any deletion:

* **Density 0.7, value 3 (27 of 30).** m - k > floor(n^2/4), so by Mantel's
  theorem a triangle always survives (value >= 3). Deleting the edges inside
  the parts of a 3-way vertex partition with at most k inside edges leaves a
  3-partite graph (value <= 3).
* **Density 0.9, value 4 (all 30).** m - k >= t_3(n) - floor(n/3) + 2, so by
  Brouwer's theorem (A. E. Brouwer, *Some lotto numbers from an extension
  of Turán's theorem*, Mathematisch Centrum ZW 152/81, 1981, Theorem 2; n >=
  2r + 1 holds) a K4-free remainder would be 3-partite, and the deletion
  would contain every inside edge of some 3-partition. Exact solver runs
  show every 3-partition has more than k inside edges, so value >= 4; a
  4-partition gives <= 4.
* **Open:** normalClique60-70-2, -3 and -5, between 3 and 4. Value 3 needs a
  set of at most 309 edges meeting every K4; the best found has 310-311,
  and LP bounds on that set stop near 296.
* The same arguments reproduce all 160 optimal values of the `bcpins` and
  `plusbcpins` sets.

`results/clique/` holds the 60 solution files, the partitions, the solver
logs and an independent review.

### Network interdiction (72; 2 not optimal)

In the attack-protect game (`inter-fire`, Baggio et al. 2021) the leader
attacks at most Phi nodes, the follower then protects at most Lambda, and
the attack spreads from attacked nodes through unprotected neighbours; the
follower maximises the nodes it saves and the leader minimises them. For
tree-50_1-5-5_006 and _010, `bobi fire` enumerates all 2,138,410 attacks of
up to 5 of the 49 nodes and solves the follower on each forest exactly by a
DP over (protected, saved) per node with the budget as a knapsack
dimension, so the minimum over attacks is a proven bound. Both are optimal
at 26, BOBILib's value. The same code reproduces all 34 tree values BOBILib
lists as optimal; with brute force on the small random graphs, 54 of
BOBILib's 70 optimal values are re-derived (`results/network/`).

### Generalized interdiction (90; 12 not optimal)

Lozano et al. prove 4. BOBILib's value for general20-30-20-20-20-2 rests on
a follower response that is 19 units suboptimal, accepted only because its
relative tolerance of 1e-4 is large next to follower objectives near 1e8.
Enumerating the linking bits reproduces the published optimum 299,577 for
general30-30-10-20-20-8 (10 bits); with 20 bits the same takes about 58 CPU
hours per instance, so the other 8 need a pruned search.

### MIPLIB-derived instances (266 not optimal, not counting 33 infeasible)

* **Literature (8).** 5 MIPLIB 3 instances by Lozano et al. (decision
  diagrams), air03-0.100000 by Fischetti et al. (2017), p0282-0.500000 by
  Vasquez, Lozano and van Hoeve (2025), and neos-950242_0_100 in an earlier
  BOBILib release.
* **No bilevel feasible point at all (40 open instances).** The follower's
  own rows admit a direction d with follower objective e.d < 0. Leader
  variables only shift right-hand sides, so d is a recession direction for
  every leader choice, and with rational data a feasible mixed-integer
  follower whose LP relaxation is unbounded is itself unbounded (Meyer
  1974): no leader choice has an optimal response. 38 directions are checked
  in exact rational arithmetic (`scripts/verify_ray.py`), 2 (supportcase42)
  only in floating point (`results/rays/`).
* **First feasible points without coupling rows (64).** Solve the HPR (the
  rows of both levels with the leader's objective, without the follower's
  optimality), fix the leader, solve the follower to optimality, and break
  ties optimistically. This handles continuous linking variables, which
  neither BOBILib solver accepts (`results/general/`).
* **First feasible points with coupling rows (29).** When leader rows
  contain follower variables, the follower's optima at the HPR's leader
  values can violate them. `bobi coupled` certifies or rules out a choice
  of the linking variables with one MILP and searches for a usable one: 21
  points at the HPR's own values, 5 from a penalised HPR, 3 from
  best-response dynamics, out of 52 candidates run
  (`results/general-coupled/`). In 9 of these files the follower's value
  uses BOBILib's tolerance (up to 1e-7 relative above its optimum); all
  pass BOBILib's 1e-4 check.
* **Optimal at the MIPLIB bound (5).** The HPR of these instances is the
  MIPLIB problem they were built from (checked in exact arithmetic by
  `scripts/compare_hpr.py`), so its published optimum bounds the bilevel
  optimum from below. pigeon-10_0_100, pigeon-10_50_50 (-9,000) and
  timtab1_0_100 (764,772), all open, get points at that bound;
  BOBILib's own points for noswot_0_100 and noswot_50_50 (-41) already met
  it.

## How the results are checked

* **Solution files.** Every file in `results/` passes `bobi verify`, which
  checks the way BOBILib's own checker does: every row and bound within
  1e-6, integrality, the stated objective, and the follower re-solved with
  the leader fixed (exact knapsack oracle for one follower row, a HiGHS
  MILP otherwise) and found optimal within BOBILib's relative 1e-4. The
  batch outputs behind this report: 270 of 270 (OR, one follower
  knapsack), 420 of 420 (OR, several), 64 of 64 (MIPLIB-derived), and all
  of the 12 subset-sum, 60 clique, 36 network and 29 coupled files.
* **Independent reviews.** The clique, network and coupled results were
  each re-derived by a separate reviewer, with its own instance reader and
  different solvers or code (its own maximum-clique searches, HiGHS and a
  hand-written branch and bound for the 3-partition bounds, a separate C
  implementation of the tree DP compared at all 9.3 M attacks). The
  reviewers found only wording errors, now fixed; their notes are in each
  `results/*/README.md`.
* **Optimality proofs** are published (cited), ours by an exact search
  whose pruning rules are proved in Lean (subset sum), arguments with
  checkable certificates (clique, rays, MIPLIB bounds), or exhaustive
  enumeration (network).

## Proofs in Lean

`lean/` (Lean 4 with Mathlib) proves the lemmas the new methods rest on:

* monotonicity of interdiction and the cheapest-copies count formulation;
* window extension and the stall test;
* Weninger and Fukasawa's online lower bound, for any item order;
* soundness of the robust online table, of the band tables and of both
  pruning rules; the tables with an arbitrary end set and the per-node rule
  of the subset-sum search, for any play order; antitonicity in the budget
  and the threshold (min-max) form the code stores (the range shortcuts
  inside the node-table pass and the match between the Rust loops and the
  recursion are argued in the code and tested against brute force, not
  formalised);
* for clique interdiction, Mantel's theorem in deletion form (more than
  n^2/4 edges left after deleting k means a triangle survives, checked for
  the three density-0.7 sizes) and the partition upper bound (deleting the
  edges inside an r-colouring leaves no clique on r + 1 vertices).

There is no `sorry` and no axiom beyond Lean's standard three.
`lean/README.md` maps each theorem to the code and lists what is not
formalised: scaling, the LP bracket, the search code itself, Brouwer's
stability theorem and the solver runs behind the density-0.9 bounds.

## Reproduce

```sh
cd bobilib
cargo build --release
D=data/collection
# subset-sum knapsack interdiction
./target/release/bobi ss-solve $D/interdiction/knapsack-interdiction/kp/interKP-200-100-6-3.mps.gz --out runs/ss_sol
python3 -I scripts/ss_check.py $D/interdiction/knapsack-interdiction/kp/interKP-200-100-6-3.{mps.gz,aux}
# OR set: bounds and certified points
./target/release/bobi bounds $D/interdiction/multidimensional-knapsack-interdiction/or/OR10x100-0.25_1.dat-100.mps.gz --out runs/or100
./target/release/bobi upper-multi $D/interdiction/multidimensional-knapsack-interdiction/or/OR10x250-0.75_1.dat-0.mps.gz --out runs/ormulti
# network interdiction
./target/release/bobi fire $D/interdiction/network-interdiction/inter-fire --only tree-50_1-5-5 --out results/network
# MIPLIB-derived: unbounded followers, feasible points, coupling rows
./target/release/bobi ray $D/general-bilevel/.../gen-ip054_0_100.mps.gz --out results/rays
python3 -I scripts/verify_ray.py INSTANCE.mps.gz INSTANCE.aux results/rays/gen-ip054_0_100.exact.ray.json
./target/release/bobi general $D/general-bilevel/.../mik-250-1-100-1_0_100.mps.gz --out runs/general
./target/release/bobi coupled $D/general-bilevel/.../pigeon-10_50_50.mps.gz --out runs/coupled
python3 -I scripts/compare_hpr.py INSTANCE.mps.gz INSTANCE.aux pigeon-10.mps.gz
# check any solution file the way BOBILib does
./target/release/bobi verify --instances $D results/kip results/ss results/network results/or
./target/release/bobi verify --instances $D --generic results/general results/general-coupled results/clique
```

Clique scripts and commands are in `results/clique/README.md`.

## Caveats

* "Literature" counts use the per-instance data the authors publish
  (Weninger and Fukasawa's answer files, Fischetti et al.'s tables, Lozano
  et al.'s solution files); where only a value is published, the instance
  is counted as solved but its file here is ours.
* Two of the 40 no-feasible-point certificates (supportcase42) are checked
  only in floating point.
* The OR medians compare instances that have both a published lower and
  upper bound; instances without a published point are counted separately.
* Machine: 4 shared cores, often oversubscribed, so times are indicative
  only.

## Next steps

* Send the new optima, certificates and solution files to the BOBILib
  maintainers, who update statuses from submitted solutions.
* The 4 remaining subset-sum instances: a stronger search (tables rebuilt
  deeper in the tree, witness reuse, or a hitting-set formulation over copy
  counts) is the most promising route.
* OR with several follower knapsacks: a lower bound that exploits the
  structure (the single-knapsack bracket does not carry over), and points
  for the 120 budget-25% instances without one.
* Generalized interdiction with 20 linking bits: a pruned enumeration.
