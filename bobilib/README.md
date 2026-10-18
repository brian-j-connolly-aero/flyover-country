# bobi: solvers for BOBILib interdiction instances

Research code for the [BOBILib](https://bobilib.org) bilevel instance library
(Bilevel Optimization (Benchmark) Instance Library). It is a separate Cargo
workspace and does not touch the game in the rest of this repository.

BOBILib's statuses come from two general-purpose bilevel solvers run for one
hour each. About 79% of the instances it does not list as optimal are
*packing interdiction* games: the leader removes items within one or more
budgets, then the follower packs the remaining items into one or more
knapsacks. This crate exploits that structure.

## Build

```sh
cd bobilib
cargo build --release      # builds HiGHS from source the first time (about 2 minutes)
ln -s /path/to/bobilib-data data   # collection/ and collection-solutions/ from bobilib.org
```

## Commands

| Command | What it does |
| --- | --- |
| `bobi survey DIR` | Parse every instance and report which ones are packing interdiction. |
| `bobi check MPS SOL` | Check a BOBILib solution file's constraints and objective. |
| `bobi verify MPS SOL` | Verify a solution like BOBILib's feasibility checker: rows, bounds and integrality within 1e-6, the stated objective, and follower optimality with the leader fixed (relative gap at most 1e-4). |
| `bobi verify --instances DIR SOL...` | The same for many solution files or directories, matching instances by name. |
| `bobi kip-import DIR --meta --wf --out` | Write verified optimal solution files for BOBILib's KIP instances from Weninger and Fukasawa's published optima (see `results/kip`). |
| `bobi ss-solve MPS...` | Exact solver for subset-sum knapsack interdiction (profit = follower weight). |
| `bobi lp-bound MPS...` | Bracket min over leader choices of the follower's LP value: a lower and an upper bound on the bilevel optimum. |
| `bobi mkip MPS...` | Interdiction-cut loop with a HiGHS master (baseline). |
| `bobi fire DIR\|MPS... [--only tree-]` | Attack-protect games (`inter-fire`): every attack, the follower solved exactly by a tree DP (HiGHS on graphs with cycles), certified optimum and solution file. See [results/network](results/network/README.md). |
| `bobi general MPS...` | Feasible points for general instances without coupling rows: HPR, follower, tie-break. |
| `bobi coupled MPS...` | Verified feasible points for general instances *with* coupling rows (time-limited portfolio, see below). |
| `bobi coupled-diag MPS...` | Why the HPR's leader choice fails: least coupling violation over the follower's optima, and the rows that stay violated. |
| `bobi certify MPS SOL` | Re-check a solution file the way BOBILib's checker does (rows, follower optimality, optimistic tie-break) with proven dual bounds. |

## Methods

* `robust_ss`: exact search for subset-sum knapsack interdiction. Followers
  only see how many copies of each weight survive, so the leader's choice
  reduces to removal counts per weight class (cheapest copies first). The
  search prunes with contiguous windows of reachable sums, a stall analysis
  that prices how much the leader must delete to stop a window growing, and
  robust online tables: sums the follower can guarantee in a relaxation
  where the leader deletes class by class and reacts to the follower's
  earlier choices. The tables are antitone in the budget, so each is stored
  as one budget threshold per (class, sum). That makes them cheap enough to
  rebuild at every node that would branch, for the undecided classes in any
  play order and with the node's decided sums as the set the game must end
  in. Playing the classes that are cheapest to delete per unit of mass first
  is strongest. Options of `ss-solve` and `ss`: `--node-order` (per-node
  tables, default `ratioasc`; `none` turns them off), `--widths` and
  `--extra-kind`/`--extra-widths` (tables built once at the root),
  `--cache-mb`, `--progress`.
* `verify`: the follower problem keeps the follower's rows only (as
  BOBILib's checker does) with the leader's values moved into the row
  bounds. Packing interdiction uses the exact knapsack oracle for one
  follower row and a HiGHS MILP for several; anything else is a HiGHS MILP
  built from the model. A MILP stopped by `--time-limit` still gives a
  verdict when its incumbent beats the file (not optimal) or its bound is
  within the tolerance (optimal); otherwise the verdict is `unproven`.
* `mkip::lp_follower_bound`: dualising the follower's LP turns
  min over x of U(x) into one MILP (the product of duals and binary leader
  variables linearises exactly). Since a knapsack's integer optimum is within
  one item's profit of its LP value per knapsack row, this brackets the
  bilevel optimum.
* `fire`: BOBILib's attack-protect set (`inter-fire`, Baggio et al. 2021).
  The leader attacks at most Phi nodes and the follower protects at most
  Lambda; an unprotected node is saved only if all its neighbours are. On a
  forest the follower's MILP is a dynamic program over the four values of
  (protect, saved) per node, with the budget as a knapsack dimension. Every
  attack is enumerated and every DP follower point is checked against the
  raw model, so the minimum over attacks is a proven lower bound; HiGHS
  re-solves the follower at the minimiser.
* `coupled`: feasible points when leader rows contain follower variables.
  The follower depends on the leader only through the linking variables
  x_L, so for fixed x_L one MILP (all rows, follower value at most its
  optimum, leader objective, the other leader variables free) gives the
  leader's best point or proves x_L *blocked*. The portfolio searches for an
  unblocked x_L: the HPR's (`hpr`); best-response dynamics (`follow`); the
  HPR with F + rho f (`penalty`); a penalty alternating direction method
  with follower LP duals after Kleinert and Schmidt (`padm`); steps of the
  continuous linking variables into the HPR's slice (`perturb`);
  value-function cuts "the follower's last response is cut off at x_L, or
  f(y) <= its value" (`vcut`, exact when the linking rows are integral); and
  no-good cuts on integer linking variables (`nogood`). Every point is
  re-certified from scratch; `vcut` and `nogood` can also prove that no
  bilevel feasible point exists. Results: `results/general-coupled/`.

## Proofs

`lean/` is a Lean 4 project (with Mathlib) that proves the lemmas these
methods rely on. It covers monotonicity of interdiction, the count
formulation, window extension and the stall test, Weninger and Fukasawa's
online lower bound, and the soundness of the robust online table. See
`lean/README.md` for how each theorem maps to the code and how to build it.
