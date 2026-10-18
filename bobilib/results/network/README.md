# inter-fire: certified optima for BOBILib's attack-protect games

BOBILib's network-interdiction set `inter-fire` holds 72 attack-protect
games from Baggio, Carvalho, Lodi and Tramontani (2021), on trees and random
graphs. BOBILib lists 70 as optimal; `tree-50_1-5-5_006` and
`tree-50_1-5-5_010` are only `feasible`, with value 26.

**Result.** Both open instances have optimum **26**: BOBILib's feasible
value is optimal. All 34 tree instances BOBILib lists as optimal are
reproduced exactly, with no disagreement.

| Instances | BOBILib | `bobi fire` |
| --- | --- | --- |
| 34 trees listed optimal | optimal | the same value on all 34, certified |
| `tree-50_1-5-5_006` | feasible, 26 | **optimal, 26**: 4 of the 2,138,410 attacks reach it |
| `tree-50_1-5-5_010` | feasible, 26 | **optimal, 26**: 28 of the 2,138,410 attacks reach it |

The two open instances have 49 nodes (one of 50 already vaccinated) and
42 and 44 edges (forests of 7 and 5 trees), with Phi = Lambda = 5. The
search covers every attack of 0 to 5 nodes, 2,138,410 per instance. The
least saved count by attack size k = 0, ..., 5 is 49, 48, 44, 38, 32, 26 for
`_006` and 49, 48, 42, 34, 30, 26 for `_010`. BOBILib's own attacks are
among the optimal ones (the follower's optimum there is 26), so BOBILib had
the optimum and only lacked the proof. The certified solutions:

| Instance | Attack (`y_v = 1`) | Protection (`x_v = 1`) | Saved |
| --- | --- | --- | --- |
| `tree-50_1-5-5_006` | 4, 6, 12, 17, 29 | 5, 18, 25, 35, 36 | 26 |
| `tree-50_1-5-5_010` | 3, 5, 16, 18, 26 | 1, 9, 23, 36, 40 | 26 |

**Runtime** (one thread, on a 4-core machine already running other jobs at
a load of about 17; under 20 MB of memory): 80 s and 70 s for the full
enumeration of `_006` and `_010`, every follower point checked against the
model, then 8 s for the 502 HiGHS follower solves each. All 36 trees, 9.34
million attacks in all, took 637 s: 317 s of enumeration and the rest HiGHS
cross-checks.

**Checks**, over all 36 trees: 0 follower points that fail the model, 0
disagreements between the DP and HiGHS in 18,072 follower solves (502 per
instance), and every solution file has max violation 0 and a follower
objective equal to HiGHS's optimum. With HiGHS as the follower for every
attack (`highs_crosscheck.csv`), the 12 `tree-25` instances, the 4
`tree-80_2-2-2` instances and the 12 `rndgraph-25` random graphs (which have
cycles, so the DP does not apply) all give BOBILib's optimal values too.
That also checks the reading of the model on graphs with cycles. The larger
random graphs were left out: after 20 minutes HiGHS had not finished the
3,082 attacks of one `rndgraph-80_2-2-2` instance on this machine.

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
* `<name>.res.json`: certified solutions for the 36 tree instances.

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
# HiGHS for every attack (about 35 minutes here)
./target/release/bobi fire $D --only rndgraph-25,tree-25,tree-80_2-2-2 \
    --force-highs --highs-max 5000 --threads 1 --sample 0 \
    --bobilib $S --csv results/network/highs_crosscheck.csv
```
