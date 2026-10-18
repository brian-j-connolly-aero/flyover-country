# Subset-sum knapsack interdiction: the 16 instances open in the literature

BOBILib's knapsack interdiction set has 16 instances with no published
optimum: `interKP-{200,300,400,500}-100-{6,7}-{2,3}`. In all of them the
follower's profit equals its weight (class 6: subset sum; class 7: even
weights and an odd capacity). Weninger and Fukasawa's branch and bound
(16 threads, 1 h), their reimplementation of Della Croce and Scatamacchia,
and Lozano, Bergman and Cire's decision diagrams (2 h) leave them open.

Each file here (`<name>.res.json`, status `optimal`) is BOBILib's solution
format for the leader choice found by `bobi ss-solve`, with the follower's
best response. In every case the objective equals BOBILib's own value; what
is new is the proof that nothing better exists. `proofs.csv` lists:

| Column | Meaning |
| --- | --- |
| `optimum` | proven optimal value (= `bobilib_value`) |
| `nodes`, `cpu_seconds` | `bobi ss-solve` on one thread (CPU time; the machine was shared, so wall times were 2-3 times longer) |
| `independent_check_nodes` | nodes of `scripts/ss_check.py`, which proves the same lower bound independently |

## How each value is established

* **Upper bound.** The file's leader choice is within budget and the
  follower's best response is worth `optimum`. `bobi verify` checks this the
  way BOBILib's checker does (rows, objective, follower optimality by an
  exact knapsack re-solve):

  ```sh
  ./target/release/bobi verify --instances data/collection results/ss
  ```

* **Lower bound.** For every leader deletion within budget, the follower
  still reaches a total in `[optimum, C]`. `bobi ss-solve` proves this by
  search over how many copies of each weight survive, pruned by robust
  online tables computed at each node from its decided sums (see
  `src/robust_ss.rs`; the rules have written proofs in their doc comments
  and randomized brute-force tests).

  `scripts/ss_check.py` repeats the lower-bound proof without sharing code
  with the Rust solver: it parses the MPS and AUX files itself, checks that
  the instance is a KIP with profits equal to weights, and runs a plain
  search with only the direct rule and the per-node online table, written
  from its definition. Its `selftest` compares it with brute force over
  every subset of items on random small instances.

  ```sh
  python3 -I scripts/ss_check.py selftest
  python3 -I scripts/ss_check.py INSTANCE.mps.gz INSTANCE.aux - OPTIMUM
  ```

  Both provers agree on all 16 (and on every class 6/7 instance with 100
  items and every 200-item one, where they also match Weninger and
  Fukasawa's optima).
