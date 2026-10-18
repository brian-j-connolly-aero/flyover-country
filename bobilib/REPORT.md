# BOBILib: what can be solved, and approaches that resolve many instances at once

*Status as of 2026-10-18. Counts marked (running) are from batches still in
progress when this was written; `scripts/summarize.py` regenerates them.*

## The question

[BOBILib](https://bobilib.org) (Thürauf, Kleinert, Ljubić, Ralphs, Schmidt,
*Mathematical Programming Computation* 2026) is a library of 2,654
mixed-integer bilevel linear problems (MIBLPs). It lists 1,027 as optimal,
33 as infeasible, 1,175 as feasible (a point is known, no proof) and 419 as
open (no point known). We asked which of the 1,594 feasible or open
instances can be solved or improved, and whether new approaches could
resolve many of them at once.

## Short answer

Yes. Two observations carry most of the weight.

1. **BOBILib's statuses come from two general-purpose bilevel solvers**
   (MibS and Fischetti et al.'s MIX++), one hour on one thread each. They
   lag the specialised literature: 527 of the 1,594 instances are already
   solved in published work (376 knapsack interdiction, 139
   multidimensional knapsack interdiction, 4 generalized interdiction, 8
   MIPLIB-derived).
2. **79% of the unsolved instances are one structure.** 1,254 of the 1,594
   are *packing interdiction* games: the leader removes items within one or
   more budgets, then the follower packs what is left into one or more
   knapsacks, and the game is zero-sum. General solvers do not exploit
   this; specialised combinatorial algorithms do.

On top of the literature, this work adds these approaches, each resolving a
whole family rather than single instances:

| Approach | Family | Effect |
| --- | --- | --- |
| Robust subset-sum search (count formulation, online tables in threshold form, rebuilt at every node from its decided sums) | knapsack interdiction, subset-sum classes | all 16 instances unsolved anywhere proven optimal (8 even-odd, 8 subset-sum), each in about 10 s of CPU time or less on one thread, and checked by an independent prover |
| LP-follower bracket and merged-row online bound | multidimensional knapsack interdiction (OR set), one follower knapsack | median gap 17% → ~1.6% (budget 25%), 31% → ~3.5% (budget 50%); better certified points on most instances |
| Certified points via the follower LP | OR set, several follower knapsacks | every budget-75% instance certified (180/180); first certified points for 72 instances with none anywhere (running) |
| Extremal graph theory (Mantel/Turán, Brouwer's stability theorem) | edge clique interdiction | 57 of 60 proven optimal (all 60 unsolved in the literature); optimum 3 or 4 where BOBILib lists 5 to 13 |
| Recession-ray certificates | general MIPLIB-derived instances | 40 "open" instances have no bilevel feasible point at all (follower unbounded for every leader choice; 38 checked in exact arithmetic) |
| HPR, follower, optimistic tie-break | general instances without coupling rows, incl. continuous linking variables | feasible points where neither BOBILib solver applies (running) |
| HPR bound = MIPLIB optimum | general MIPLIB-derived instances | 4 proven optimal (2 of them open, with no point known before) |
| Exhaustive enumeration with exact follower solves | network interdiction trees, generalized interdiction with few linking bits | the 2 non-optimal tree instances certified optimal; a published optimum reproduced |

What stays hard: most OR
instances with several follower knapsacks and budget 25-50% (certified
points, but the best lower bounds are still 30-40% away), 3 clique
instances, generalized interdiction with 20 or more linking bits, and
general instances with coupling rows.

## Family by family

### Knapsack interdiction (599 instances; 392 not optimal in BOBILib)

* **Literature.** Weninger and Fukasawa's combinatorial branch and bound
  (IPCO 2023; *Math. Programming* 210, 2025) solves 583 of the 599. BOBILib
  lists 376 of those as not optimal; at 335 of them its value is above the
  optimum (median 8%, up to 672%). `results/kip/` holds a verified solution
  file for each of the 583 (`bobi kip-import`, checked with `bobi verify`).
* **The 16 left.** All have a follower whose profit equals its weight
  (subset-sum), budget index 2 or 3 and 200-500 items. Weninger and
  Fukasawa (16 threads, 1 h), their reimplementation of Della Croce and
  Scatamacchia, and Lozano, Bergman and Cire's decision diagrams (2 h) all
  leave them open; the best lower bounds sit 2 to 8 units below the
  incumbent, which equals the follower's capacity (or capacity minus one
  for the even-odd class).
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
  | interKP-300-100-6-3 | 4,302 | 19,405 | 8.06 s | open, not run |
  | interKP-400-100-6-3 | 5,748 | 13,430 | 9.01 s | open, not run |
  | interKP-500-100-6-3 | 6,976 | 10,079 | 10.04 s | open, not run |
  | interKP-200-100-7-2 | 1,872 | 22 | 0.06 s | 48,598 / 0.49 s |
  | interKP-300-100-7-2 | 2,894 | 27 | 0.04 s | 115,415 / 1.78 s |
  | interKP-400-100-7-2 | 3,866 | 18 | 0.05 s | 269,550 / 5.62 s |
  | interKP-500-100-7-2 | 4,694 | 1 | 0.05 s | 357,279 / 10.26 s |
  | interKP-200-100-7-3 | 2,808 | 651 | 0.10 s | 2,341,175 / 9.61 s |
  | interKP-300-100-7-3 | 4,340 | 470 | 0.20 s | 3,256,897 / 20.35 s |
  | interKP-400-100-7-3 | 5,800 | 378 | 0.17 s | 10,710,774 / 86.43 s |
  | interKP-500-100-7-3 | 7,042 | 256 | 0.15 s | 13,346,235 / 137.19 s |

  In every case BOBILib's value was already optimal; the proof is new.
  All 16 lower bounds are confirmed by `scripts/ss_check.py`, an
  independent prover (own parser, plain search, the per-node rule written
  from its definition, no code shared with the Rust search), and every
  solution file passes `bobi verify`. See `results/ss/`. One change at a
  time on interKP-200-100-6-2 (nodes): previous search 3,926,223; same
  rules with threshold tables 3,926,223 (memory 478 to 229 MB); every band
  width 1..64 3,278,153; root tables played largest value first 1,128,566,
  or cheapest per unit of mass first 513,834; per-node tables 303. With
  100 items, the class 6 and 7 instances with budget index 4-10, which the
  previous version did not finish in two minutes, now close at the root,
  and all 20 match Weninger and Fukasawa's optima.

### Multidimensional knapsack interdiction (954; 862 not optimal)

* **imkp (144).** Fischetti, Ljubić, Monaci and Sinnl (*INFORMS JoC* 2019)
  solved all 144; BOBILib lists 52 as only feasible, with values up to 23%
  above the optimum.
* **OR (810).** Built from Chu and Beasley's multidimensional knapsacks
  (100-500 items, 5-30 rows, budgets 25/50/75%). BOBILib has no optimum.
  Lozano, Bergman and Cire (*Operations Research* 2026) proved 87 (all
  budget 75%); gaps elsewhere are 13-47%, and 139 had no certified point.
* **New, one follower knapsack (270).** Dualising the follower's LP gives
  U(x) = min over l >= 0 of l C + sum_j max(0, p_j - l w_j)(1 - x_j); with
  binary x the product linearises exactly, so min over x of U(x) is one
  MILP. A knapsack's optimum is within one item's profit of its LP value,
  which brackets the bilevel optimum. Weninger and Fukasawa's online
  relaxation, generalised to several leader rows by merging them with
  tuned multipliers and to large integer data by rounding in the safe
  direction, gives a second lower bound that is usually stronger. With
  local search on the leader, median gaps fall to about 1.6% (budget 25%)
  and 3.5% (budget 50%), against 17% and 31% in the literature, and the
  certified points beat the best published ones on most instances
  (`runs/or_compare.csv`).
* **New, several follower knapsacks (540).** The follower's LP at the
  minimiser of U is a valid upper bound on the optimum for every instance,
  and whenever HiGHS solves the follower exactly the point is certified.
  All 180 budget-75% instances are certified (the follower takes almost
  everything left), including the n = 500, 30-row ones that had no point
  anywhere. At budget 50% the smaller instances also certify (running).
  Lower bounds here remain the literature's.
* One case to flag: for OR10x100-0.75_4.dat-0 the published lower bound
  8,814 comes from runs its authors marked numerically inconsistent; our
  point 8,813 (verified with SCIP) meets their branch-and-cut bound of 8,813,
  closing it.

### Edge clique interdiction (220; 60 not optimal)

The 60 `clique` instances (40-60 vertices, density 0.7 or 0.9, budget a
quarter of the edges) are unsolved in the literature; BOBILib's values are
5 to 13. After deleting k edges, m - k edges remain:

* density 0.7: m - k > floor(n^2/4), so by Mantel's theorem a triangle
  always survives (value >= 3); a 3-way vertex partition with at most k
  edges inside its parts gives value <= 3. 27 of 30 closed at 3.
* density 0.9: m - k >= t_3(n) - floor(n/3) + 2, so by Brouwer's theorem
  (1981) a remaining graph without K4 would be 3-partite; exact solvers
  (CP-SAT, SCIP and a dedicated branch and bound) show every 3-partition
  has more than k inside edges, so value >= 4; a 4-partition gives <= 4.
  All 30 closed at 4.
* Open: normalClique60-70-2, -3, -5 (between 3 and 4).

`results/clique/` holds the certificates (partitions, solution files) and
the arguments.

### Generalized and network interdiction

* **Generalized (12 not optimal).** Lozano et al. prove 4. BOBILib's value
  for general20-30-20-20-20-2 rests on a follower response that is 19 units
  suboptimal, accepted only because its relative tolerance of 1e-4 is large
  next to follower objectives near 1e8. Enumerating the linking bits
  reproduces the published optimum 299,577 for general30-30-10-20-20-8 (N
  = 10); with N = 20 the same takes about 58 CPU hours per instance, so the
  rest need a pruned search.
* **Network, attack-protect on trees (2 not optimal).** Enumerating every
  5-node attack and solving the protection problem on the tree by dynamic
  programming certifies both at 26.

### General MIPLIB-derived instances (266 not optimal)

* **Literature.** 8 are proven optimal elsewhere: 5 MIPLIB 3 instances by
  Lozano et al. (decision diagrams), air03-0.100000 by Fischetti et al.
  (2017), p0282-0.500000 by Vasquez, Lozano and van Hoeve (2025), and
  neos-950242_0_100 in an earlier BOBILib release.
* **No bilevel feasible point at all (40 open instances).** For these the
  follower's own rows admit a direction d with follower objective e d < 0.
  Leader variables only shift right-hand sides, so d is a recession
  direction for every leader choice, and with rational data an unbounded LP
  relaxation makes a feasible mixed-integer follower unbounded (Meyer
  1974): no leader choice has an optimal response. 38 directions are
  checked in exact rational arithmetic (`scripts/verify_ray.py`), 2
  (supportcase42) in floating point. Most are `_50_50` variants, where
  moving half the rows to the leader leaves the follower unbounded.
* **Feasible points (running).** Without coupling rows, solve the high
  point relaxation (HPR: both levels' rows, the leader's objective, no
  optimality condition on the follower), fix the leader, solve the
  follower to optimality and break ties optimistically. This works with
  continuous linking variables, which neither BOBILib solver accepts; no
  per-instance results were published for those 137 instances.
* **Optimal at the MIPLIB bound.** The HPR of these instances is the MIPLIB
  problem they were built from (checked in exact arithmetic by
  `scripts/compare_hpr.py`), so its published optimum is a lower bound. Two
  open instances, pigeon-10_0_100 (-9,000) and timtab1_0_100 (764,772),
  get points at that bound from the pipeline above, and BOBILib's own
  points for noswot_0_100 and noswot_50_50 (-41) already met it. All four
  are optimal (`results/general/`). neos-950242_0_100 meets its bound too,
  but an earlier BOBILib release already listed it as optimal.

## Proofs in Lean

`lean/` (Lean 4 with Mathlib) proves the lemmas the new methods rest on:

* monotonicity of interdiction and the cheapest-copies count formulation;
* window extension and the stall test;
* Weninger and Fukasawa's online lower bound, for any item order;
* soundness of the robust online table, of the band tables and of both
  pruning rules (the per-node tables of the subset-sum search use the same
  induction with an arbitrary base set; that step and the threshold form
  are argued in the code and tested against brute force, not formalised);
* for clique interdiction, Mantel's theorem in deletion form (more than
  n^2/4 edges left after deleting k means a triangle survives, checked for
  the three density-0.7 sizes) and the partition upper bound (deleting the
  edges inside an r-colouring leaves no clique on r + 1 vertices).

There is no sorry and no axiom beyond Lean's standard three.
`lean/README.md` maps each theorem to the code and lists what is not
formalised: scaling, the LP bracket, the search code itself, Brouwer's
stability theorem and the solver runs behind the density-0.9 bounds.

## Reproduce

```sh
cd bobilib
cargo build --release
# subset-sum knapsack interdiction
./target/release/bobi ss-solve data/collection/interdiction/knapsack-interdiction/kp/interKP-200-100-6-3.mps.gz --out runs/ss_sol
python3 -I scripts/ss_check.py data/collection/interdiction/knapsack-interdiction/kp/interKP-200-100-6-3.{mps.gz,aux}
# OR set: bounds and certified points
./target/release/bobi bounds  data/.../or/OR10x100-0.25_1.dat-100.mps.gz --out runs/or100
./target/release/bobi upper-multi data/.../or/OR10x250-0.75_1.dat-0.mps.gz --out runs/ormulti
# general instances: unbounded followers and feasible points
./target/release/bobi ray data/.../gen-ip054_0_100.mps.gz --out runs/rays
python3 -I scripts/verify_ray.py INSTANCE.mps.gz INSTANCE.aux runs/rays/gen-ip054_0_100.ray.json
./target/release/bobi general data/.../mik-250-1-100-1_0_100.mps.gz --out runs/general
# check any solution file the way BOBILib does
./target/release/bobi verify --instances data/collection results/kip runs/ss_sol
```

## Caveats

* "Literature" counts use the per-instance data the authors publish
  (Weninger and Fukasawa's answer files, Fischetti et al.'s tables, Lozano
  et al.'s solution files); where only a value is published, the instance
  is counted as solved but its file here is ours.
* Every solution file in `results/` passes `bobi verify` (rows, objective,
  follower optimality with an exact re-solve). Optimality proofs are either
  published (cited), ours by an exact search (subset-sum), or arguments
  with certificates (clique, rays).
* Machine: 4 shared cores, so times are indicative only.
