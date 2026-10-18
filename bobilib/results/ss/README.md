# Subset-sum knapsack interdiction: new optimality proofs

Sixteen of BOBILib's 599 knapsack interdiction instances are unsolved in
the literature: classes 6 (subset sum: profit equals weight) and 7 (even-odd)
with budget index 2 or 3 and 200-500 items. `bobi ss-solve` proves 12 of
them optimal (`summary.csv`, one solution file each here):

* all 8 class-7 instances;
* the 4 class-6 instances with budget index 2.

In every case BOBILib's value was already optimal; the proof is new. The
optimum equals the follower's capacity C (class 6) or C minus one (class 7),
so the proof shows that every deletion within the budget leaves a subset of
the survivors with exactly that sum. The search, its pruning rules and the
Lean proofs of their soundness are described in `../../README.md` and
`../../lean/README.md`.

Still open: the 4 class-6 instances with budget index 3
(interKP-{200,300,400,500}-100-6-3).

Times are single-threaded on a shared 4-core machine; `nodes` counts search
nodes. Each solution file passes `bobi verify` (rows, objective, and the
follower re-solved exactly by the knapsack oracle).

```sh
./target/release/bobi ss-solve data/collection/interdiction/knapsack-interdiction/kp/interKP-400-100-6-2.mps.gz --node-limit 100000000000 --out results/ss
./target/release/bobi verify --instances data/collection results/ss
```
