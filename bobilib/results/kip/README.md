# Optimal solutions for BOBILib's knapsack interdiction instances

One BOBILib solution file (`<name>.res.json`, status `optimal`) for each of
the 583 knapsack interdiction (KIP) instances in BOBILib (sets `kp`, `cclw`
and `inter-kp`, 599 instances) that has a published optimum. BOBILib itself
lists 376 of them as not solved to optimality:

| BOBILib status | Instances | With a file here | BOBILib value already optimal | BOBILib value above the optimum |
| --- | ---: | ---: | ---: | ---: |
| optimal | 207 | 207 | 207 | 0 |
| feasible | 377 | 361 | 26 | 335 |
| open (no point known) | 15 | 15 | - | - |

The 16 instances without a file are subset-sum instances with no published
optimum. `summary.csv` lists all 599, with columns `name`, `set`,
`bobilib_status`, `bobilib_value` (BOBILib's best known objective),
`literature_optimum`, `verified_value` (objective of the file here) and
`source` (see below).

## Provenance

The optimal values are Weninger and Fukasawa's: N. Weninger and R. Fukasawa,
"A fast combinatorial algorithm for the bilevel knapsack problem with
interdiction constraints", Mathematical Programming 210 (2025) 847-879,
doi [10.1007/s10107-024-02133-9](https://doi.org/10.1007/s10107-024-02133-9).
Their code, instance files (`.ki`), optima (`.ans`) and run records
(`full_results/all_results.json`) are at
[github.com/nwoeanhinnogaehr/bkpsolver](https://github.com/nwoeanhinnogaehr/bkpsolver)
(MIT licence; read at commit bc1d985). None of their code is used here: only
the optimal values and, where readable, the leader vectors of their runs.

Each BOBILib instance is matched to their file by its capacities and the
multiset of item triples (profit, follower weight, leader cost). Their files
leave out items with zero leader cost, which the leader always blocks for
free; items with equal triples are interchangeable. Every match is unique
and BOBILib's values never undercut a published optimum.

The leader vector (`source` column) comes from

* `wf-comb:<file>`: their Comb run, bits in file order (230 instances);
* `wf-comb-sorted:<file>`: their Comb run, bits in their solver's internal
  order (decreasing p / w^L, then decreasing profit, increasing leader cost,
  increasing follower weight), which some records print without mapping
  back; their `up_util` field then exceeds the budget (127 instances);
* `greedy+ls`: the remaining records (all `interKP-*` instances of set `kp`,
  their FMS groups) fit neither order; they appear to use the internal
  order with another tie-break among equally efficient but different
  items, which the data cannot recover. For these the
  leader vector is recomputed by the greedy leader plus local search in
  this crate (226 instances).

Every leader vector, plus all zero-cost items, must fit the budget, and the
follower's exact best response (`kp::solve`) must equal the published optimum;
the file then holds that leader vector and response. Bilevel feasibility is
checked here; the optimality proof is theirs (the value equals their proven
optimum).

## Reproduce and verify

```sh
cd bobilib
cargo build --release
WF=/path/to/bkpsolver            # their repository
./target/release/bobi kip-import data/collection/interdiction/knapsack-interdiction \
    --meta /path/to/collection.json --wf $WF --out results/kip
./target/release/bobi verify --instances data/collection/interdiction/knapsack-interdiction results/kip
./target/release/bobi verify --generic --instances data/collection/interdiction/knapsack-interdiction results/kip
```

`collection.json` is BOBILib's instance metadata. Both verifier runs pass all
583 files: rows, bounds and integrality hold exactly, the objective matches,
and the follower part is optimal for the leader part, by the exact knapsack
oracle and, with `--generic`, by a HiGHS MILP of the follower problem.
