# inter-fire: certified optima for BOBILib's attack-protect games

BOBILib's network-interdiction set `inter-fire` holds 72 attack-protect
games from Baggio, Carvalho, Lodi and Tramontani (2021), on trees and random
graphs. BOBILib lists 70 as optimal; `tree-50_1-5-5_006` and
`tree-50_1-5-5_010` are only `feasible`, with value 26.

**Result.** Both open instances have optimum **26**: BOBILib's feasible
value is optimal. Of BOBILib's 70 optimal values, 46 are re-derived here
from the model files with no disagreement: all 34 trees (tree DP) and the
12 `rndgraph-25` random graphs (HiGHS as the follower at every attack). The
other 24 (`rndgraph-50`, `rndgraph-80`) were not enumerated; BOBILib's
solution files for them pass `bobi verify`, which shows their values are
attained but does not re-prove optimality. An independent review (last
section) confirms all of this with separate code and also re-derives 8 of
those 24 by brute force (`rndgraph-50_1-3-3`, `rndgraph-80_2-2-2`), so 54
of the 70 are re-derived in all.

| Instances | BOBILib | This work |
| --- | --- | --- |
| 34 trees | optimal | the same value on all 34, certified (DP; 16 of them also with HiGHS at every attack) |
| `tree-50_1-5-5_006` | feasible, 26 | **optimal, 26**: 4 of the 2,138,410 attacks reach it |
| `tree-50_1-5-5_010` | feasible, 26 | **optimal, 26**: 28 of the 2,138,410 attacks reach it |
| 12 `rndgraph-25` | optimal | the same value on all 12, certified (HiGHS at every attack) |
| 24 `rndgraph-50`, `rndgraph-80` | optimal | not enumerated by `bobi fire`; BOBILib's points verified feasible with the stated value; the review's brute force re-derives the 8 `rndgraph-50_1-3-3` and `rndgraph-80_2-2-2` values |

The two open instances have 49 nodes (one of 50 already vaccinated) and
42 and 44 edges (forests of 7 and 5 trees), with Phi = Lambda = 5. The
search covers every attack of 0 to 5 nodes, 2,138,410 per instance. The
least saved count by attack size k = 0, ..., 5 is 49, 48, 44, 38, 32, 26 for
`_006` and 49, 48, 42, 34, 30, 26 for `_010`. BOBILib's own attacks are
among the optimal ones (the follower's optimum there is 26), so BOBILib had
the optimum and only lacked the proof. (An earlier, unreviewed claim from
C code that enumerated the 1,906,884 five-node attacks reached the same
value; none of its code or parsed data is used here.) The certified
solutions:

| Instance | Attack (`y_v = 1`) | Protection (`x_v = 1`) | Saved |
| --- | --- | --- | --- |
| `tree-50_1-5-5_006` | 4, 6, 12, 17, 29 | 5, 18, 25, 35, 36 | 26 |
| `tree-50_1-5-5_010` | 3, 5, 16, 18, 26 | 1, 9, 23, 36, 40 | 26 |

**Runtime** (one thread, on a 4-core machine already running other jobs at
a load of about 17; under 20 MB of memory): 80 s and 70 s for the full
enumeration of `_006` and `_010`, every follower point checked against the
model, then 8 s for the 502 HiGHS follower solves each. These are wall
times on a loaded machine: a review re-run of `_006` with `--sample 0`
used 32 s of CPU time (75 s wall) and 11 MB peak memory. All 36 trees, 9.34
million attacks in all, took 637 s: 317 s of enumeration and the rest HiGHS
cross-checks.

**Checks**, over all 36 trees: 0 follower points that fail the model, 0
disagreements between the DP and HiGHS in 18,072 follower solves (502 per
instance), and every solution file has max violation 0 and a follower
objective equal to HiGHS's optimum.

* **HiGHS at every attack** (`highs_crosscheck.csv`, 28 instances, 47,496
  attacks, 807 s): the 12 `tree-25` and 4 `tree-80_2-2-2` trees give the
  same optimum and the same number of optimal attacks as the DP, and the 12
  `rndgraph-25` random graphs (8 of which have cycles, where the DP does
  not apply) give BOBILib's optimal values. That also checks the reading of the model
  on graphs with cycles. The 24 larger random graphs were left out: one
  HiGHS follower solve takes 0.7 to 16 s on them here (the `seconds` column
  of `verify_bobilib.tsv`), and they have 3,082 to 2,138,410 attacks each.
* **Independent verifier** (`bobi verify`, modelled on BOBILib's
  feasibility checker: rows, bounds and integrality within 1e-6, the stated
  objective, and follower optimality re-solved with the leader fixed): all
  36 solution files written here pass (`verify_ours.tsv`), and so do all 72
  of BOBILib's own `inter-fire` solution files (`verify_bobilib.tsv`).

## The model, as read from the MPS/AUX files

Nothing here comes from the paper or from earlier code: `bobi fire` reads
every instance with `mps::read_instance` and `fire::Fire::from_model` checks
every row, bound and objective coefficient against the pattern below,
refusing the instance otherwise. Every variable is binary; node labels are
the name suffixes.

| Level | Variables | Objective | Rows |
| --- | --- | --- | --- |
| leader (attacker) | `y_v` (attack v) | min sum_v `a_v` | `Phi`: sum_v `y_v` <= Phi |
| follower (defender) | `a_v` (v saved), `x_v` (protect v) | max sum_v `a_v` (AUX coefficient -1 on `a_v`) | `Lambda`: sum_v `x_v` <= Lambda; `V<v>`: `a_v + y_v <= 1`; `A<u>-<v>`: `a_v - a_u - x_v <= 0` for both arcs of every edge |

So an attacked node is lost, and an unprotected node is saved only if all its
neighbours are: fire spreads from the attacked nodes through unprotected
ones, and protected nodes stop it. The name `tree-N_O-P-L` encodes the
budgets of the original trilevel problem (vaccinate O, attack P, protect L);
the O vaccinated nodes are already removed, so the model has N - O nodes
(the tree becomes a forest) and the rows `Phi` and `Lambda` carry P and L.
`bobi fire` checks this for every instance (`name_budgets_ok`). There are no
coupling rows, and the leader's objective is exactly minus the follower's,
so every optimal follower response gives the leader the same value and the
optimistic tie-break plays no part.

## Method (`src/fire.rs`, `bobi fire`)

1. **Leader.** The leader's feasible set is every subset of at most Phi
   nodes (binary `y`, one row). `bobi fire` enumerates all of them, every
   size from 0 to Phi, not only full-budget attacks, so no monotonicity
   argument is needed; the count per size is asserted to equal C(n, k).
2. **Follower on a forest.** The follower's MILP is solved exactly by
   dynamic programming over the follower's own variables: each node takes
   one of the four values of (`x_v`, `a_v`); an attacked node only those
   with `a_v = 0` (row `V<v>`); an edge allows the pairs that satisfy both of
   its arc rows; the budget row is a knapsack dimension (tables indexed by
   protections used, 0..Lambda). The components hang off a virtual root with
   unconstrained edges. A traceback returns an optimal follower point.
3. **Certificate.** For every attack, the DP's follower point is checked
   against the raw model: `Bilevel::max_violation` (every row, bound and
   integrality) must be 0 and `Bilevel::upper_obj` must equal the DP value.
   So the follower's optimum at each attack is at least the DP value, and
   the minimum over attacks is a lower bound on the bilevel optimum. At the
   minimising attack HiGHS re-solves the follower MILP (built from the raw
   model by `general::check_point`), proving the follower cannot do better
   there; that attack attains the bound. The certificate does not rest on
   the DP being optimal anywhere else.
4. **Cross-checks of the DP.** Per instance, HiGHS re-solves the follower
   at the optimal attack, at BOBILib's attack and at 500 random full-budget
   attacks (fixed seed), and the DP must agree on each. Unit tests compare
   the DP with brute force over every (a, x) on 300 random small forests,
   and the whole enumeration with a brute-force bilevel optimum on 40.
   `--force-highs` replaces the DP with HiGHS for every attack.

Solutions are written as `<name>.res.json` in BOBILib's format, with status
`optimal` only when every check above passed.

## Files

* `summary.csv`: one row per tree instance (DP run). Columns: nodes, edges,
  forest, budgets (`phi`, `lambda`) and whether they match the name,
  attacks enumerated, `value` (the certified optimum), how many attacks
  attain it, witness failures, BOBILib's status and value, the follower's
  optimum at BOBILib's own attack, agreement, HiGHS checks and mismatches,
  the solution's max violation, HiGHS's follower optimum at the solution,
  `certified`, seconds.
* `highs_crosscheck.csv`: the same columns, the follower solved by HiGHS
  for every attack: `rndgraph-25`, `tree-25` and `tree-80_2-2-2` (at most
  5,000 attacks each; trees with `--force-highs`).
* `verify_ours.tsv`, `verify_bobilib.tsv`: `bobi verify` on the 36
  solution files here and on BOBILib's 72 `inter-fire` solution files.
* `<name>.res.json`: certified solutions for the 36 tree instances.
* `review.tsv`: the independent review, one row per instance (method
  `enum` = reviewer's DP with witnesses, `brute` = brute force over
  protection sets at every attack; `dp_equals_treedp` = DP value equal to
  `TreeDp::solve` at every attack; checks of BOBILib's and these solution
  files).

## Reproduce

```sh
cd bobilib
cargo build --release
ln -s /path/to/bobilib-data data      # collection/ and collection-solutions/
mkdir -p results/network
D=data/collection/interdiction/network-interdiction/inter-fire
S=data/collection-solutions/interdiction-solutions/network-interdiction-solutions/inter-fire
# DP certification of all 36 trees (1 thread)
./target/release/bobi fire $D --only tree- --threads 1 --sample 500 \
    --bobilib $S --out results/network --csv results/network/summary.csv
# HiGHS for every attack (about 14 minutes here)
./target/release/bobi fire $D --only rndgraph-25,tree-25,tree-80_2-2-2 \
    --force-highs --highs-max 5000 --threads 1 --sample 0 \
    --bobilib $S --csv results/network/highs_crosscheck.csv
# BOBILib-style verification of these files and of BOBILib's
./target/release/bobi verify --instances $D results/network > results/network/verify_ours.tsv
./target/release/bobi verify --instances $D --time-limit 300 $S > results/network/verify_bobilib.tsv
```

## Independent review

A reviewer re-derived the model from the MPS/AUX files and re-solved the
instances with separate code, `scripts/fire_review/` (C and Python, written
before reading `src/fire.rs`); per-instance results are in `review.tsv`.

* `parse_fire_model.py` asserts the pattern above on all 72 MPS/AUX pairs
  (every column, coefficient, right-hand side, bound and the AUX level
  split) and writes each graph. All 36 `tree-*` graphs are forests, and so
  are 4 of the 12 `rndgraph-25`.
* The follower's optimum at an attack Y is max over protection sets X
  (|X| <= Lambda) of n - |B(Y, X)|, where B is the closure of Y under
  spreading to unprotected neighbours: for fixed X the feasible `a` are
  closed under taking maxima, and the largest one is 0 exactly on B. (Also
  checked against brute force over every (a, x) on 400 random graphs of up
  to 6 nodes.) `fire_check brute` uses only this, with no DP.
* `fire_check enum` (its own tree DP, each traced point checked against
  every row and its X by the closure) gives, on all 36 trees, the same
  optimum, attack count and number of optimal attacks as `summary.csv`;
  at every optimal attack, brute force over all X with |X| <= Lambda gives
  the same value. For `tree-50_1-5-5_006` and `_010`: 2,138,410 attacks
  each, minimum 26, reached by 4 and 28 attacks (BOBILib's attack among
  them), per-size minima as stated above; 86 s and 90 s wall on one thread.
  So both are optimal at 26 by a second, independent computation.
* `fire_check hash`: the DP value at every attack equals `TreeDp::solve`'s
  on all 40 forests (9,344,902 attacks; compared by a hash of the value
  sequence).
* `fire_check brute` agrees with the reviewer's DP at every attack on 32
  forests (the 28 trees other than `tree-50_1-5-5` and `tree-80_3-3-3`,
  where 150 random full-budget attacks each agree, and the 4 forest
  `rndgraph-25`), and re-derives BOBILib's optimal values without any DP
  on 20 random graphs: the 12 `rndgraph-25`, the 4 `rndgraph-50_1-3-3` (7,
  5, 11, 7; 130 to 160 s each) and the 4 `rndgraph-80_2-2-2` (2, 6, 2, 2;
  6 s each). With the 34 trees that makes 54 of BOBILib's 70 optimal values
  re-derived. The other 16 (`rndgraph-50_1-5-3`, `rndgraph-50_3-5-5`,
  `rndgraph-80_1-3-3`, `rndgraph-80_3-3-3`) are only shown to be attained.
* `check_solution.py` with `fire_check eval`: all 36 solution files here
  and all 72 of BOBILib's are feasible, state their sum of `a`, and their
  follower part is optimal by brute force over all X at their attack.
  `bobi verify --generic --time-limit 300` (HiGHS for every follower) also
  passes all 36 and all 72.

```sh
cd bobilib/scripts/fire_review
cc -O2 -o /tmp/fire_check fire_check.c
D=../../data/collection/interdiction/network-interdiction/inter-fire
mkdir -p /tmp/fire_g
for f in $D/*.mps.gz; do b=$(basename $f .mps.gz); python3 -I parse_fire_model.py $f $D/$b.aux /tmp/fire_g; done
/tmp/fire_check enum /tmp/fire_g/tree-50_1-5-5_006.g     # forests: DP + witnesses
/tmp/fire_check brute /tmp/fire_g/rndgraph-50_1-3-3_002.g # any graph: brute force
python3 -I check_solution.py /tmp/fire_g/tree-50_1-5-5_006.g ../../results/network/tree-50_1-5-5_006.res.json
/tmp/fire_check eval /tmp/fire_g/tree-50_1-5-5_006.g 4,6,12,17,29
```
