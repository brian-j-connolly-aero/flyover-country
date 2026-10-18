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

## Methods

* `robust_ss`: exact search for subset-sum knapsack interdiction. Followers
  only see how many copies of each weight survive, so the leader's choice
  reduces to removal counts per weight class (cheapest copies first). The
  search prunes with contiguous windows of reachable sums, a stall analysis
  that prices how much the leader must delete to stop a window growing, and a
  precomputed table of sums the follower can guarantee in an online
  relaxation that favours the leader.
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
