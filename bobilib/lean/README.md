# Lean proofs for bobi

Machine-checked proofs (Lean 4 with Mathlib) of the lemmas that the solvers in
`../src` rely on. Nothing here is run by the Rust code; it backs up why its
pruning rules and bounds are sound.

## Build

```sh
cd bobilib/lean
lake exe cache get          # prebuilt Mathlib: about 9000 files, 7 GB unpacked
lake build                  # checks every proof
lake env lean Check.lean    # prints the axioms each main theorem uses
```

The toolchain is Lean 4.35.0 (`lean-toolchain`, installed with elan) and Mathlib
is pinned to its `v4.35.0` tag (`lake-manifest.json`). With Mathlib in place, a
clean `lake build` takes about a minute and under 2 GB of memory. There is no
`sorry`: `Check.lean` shows only Lean's standard axioms (`propext`,
`Classical.choice`, `Quot.sound`) for each theorem it lists; the helper
lemmas are checked through the theorems that use them.

## The model

Items are `Fin n`. The leader blocks a set `X : Finset (Fin n)`, paying
`∑ j ∈ X, cost j`. The follower's best value with the items of `S` available
and capacity `c` is

```
K p w S c = max { p(Y) : Y ⊆ S, w(Y) ≤ c }        -- Knapsack.lean, Finset.sup
```

so against `X` the follower gets `K p w (univ \ X) C`. This is
`Kip::follower`, which runs `kp::solve` on the items with `x[j] == false`.
All data are natural numbers, as in `structure.rs`.

## Theorems

### 1. Monotonicity of interdiction (`Bobi/Knapsack.lean`)

Blocking more items never helps the follower.

* `follower_antitone`: `X ⊆ X'` implies `K (univ \ X') C ≤ K (univ \ X) C`.
* The same for any packing follower. `bestAvoiding F p X` is the best profit
  of a feasible set (a member of the family `F`) that avoids `X`.
  `bestAvoiding_antitone` holds for any family. `bestAvoiding_eq_sup_sdiff`
  says that when `F` is closed under taking subsets, blocking an item is the
  same as making it worthless (`max over Y ∈ F of p(Y \ X)`).
  `packings_downward_closed` and `K_eq_bestAvoiding` show that the knapsack
  follower is such a follower.

Rust: `Kip::free_items` lists the items that cost the leader nothing;
`Kip::greedy` blocks them, and `Kip::subset_sum_classes` drops them as always
blocked. In `robust_ss::Search::node`, the copies the leader cannot afford to
delete survive whatever it does, and so do their sums (the `guaranteed`
test).

### 2. Count formulation (`Bobi/Knapsack.lean`)

Items with the same follower weight and profit are interchangeable, so a
leader can block the cheapest copies first and lose nothing.

* `K_map_equiv`: renaming items by a permutation that keeps every profit and
  weight leaves `K` unchanged.
* `swap_block` (the theorem as stated): say `p i = p j`, `w i = w j` and
  `cost j ≤ cost i`, and `X` blocks `i` but not `j`. Then blocking `j`
  instead (`insert j (X.erase i)`) gives the follower the same best value and
  costs no more.
* `cheapest_first` (the consequence): for a class `cls` of identical items
  and any leader set `X`, there is an `X'` that agrees with `X` outside the
  class and blocks as many copies. Every copy it blocks is no costlier than
  every copy it leaves. It costs no more and gives the follower the same best
  value. The proof is strong induction on the leader's cost, since each swap
  that helps lowers it.

Rust: `Kip::subset_sum_classes` groups items by follower weight and sorts
each class's costs. `robust_ss` then branches on how many copies `r` of each
class are deleted, at cost `prefix[r]`, the sum of the `r` cheapest.

### 3. Window extension (`Bobi/SubsetSum.lean`)

`SubsetSum M t` says some sub-multiset of `M : List ℕ` sums to `t`.
`Window M a b` says every integer in `[a, b]` is such a sum.

* `window_extend` (the theorem as stated): if `a ≤ b`, `Window M a b` and
  `v ≤ b - a + 1`, then `Window (v :: M) a (b + v)`. Sums up to `b` do not
  need `v`. A sum `t` above `b` is `v` plus `t - v`, which lies in `[a, b]`.
* `window_extend_all` (iterated): `Fits w vs` says each value of `vs`, added
  in order, is at most the current width. The width starts at `w` and grows
  by each value added. If `Fits (b - a + 1) vs`, then
  `Window (vs ++ M) a (b + vs.sum)`.
* `window_survivors` (the stall test): the leader deletes some of the
  values. At each value, take the mass deleted before it plus the value
  itself. If that is at most `b - a + 1` plus all the mass before the value,
  the survivors fit, and the window grows to `b +` (surviving mass).
  `SubsetSum.perm` shows that the order of the list does not matter.

Rust: `Search::window_safe`. Its test that no class `u` is stallable says
that every deletion within budget removes less than
`D_u = mass_below_u - v_u + w + 1` from the classes below `u`. That is the
hypothesis of `window_survivors` at the first copy of `u`. At a later copy of
`u`, each earlier copy of `u` adds `v_u` to the mass before it and at most
`v_u` to the deleted mass, so the hypothesis still holds. The code checks the
classes in ascending value order, but the lemma holds for any order.

The code bounds the cheapest deletion of `D_u` mass with a fractional
(cheapest per unit) bound, `Cumulative::min_cost`. That bound is a standard
LP fact and is not formalised here.

### 4. Online relaxation lower bound (`Bobi/Online.lean`)

This is Theorem 2 of N. Weninger and R. Fukasawa, "A Fast Combinatorial
Algorithm for the Bilevel Knapsack Problem with Interdiction Constraints"
(Section 3; theorem, lemma and table numbers as in their preprint).
`online p wU wL order cU cL` is their recursive ω. Items are taken in the
order `order`. At each item the leader may block it if the budget `cU`
allows. If the leader does not, the follower may take it if the capacity `cL`
allows. A branch that cannot be afforded is not offered, which is the paper's
`±∞` convention.

* `online_le_K`: suppose the items of `order` are distinct, `X` is drawn
  from them and `wU(X) ≤ cU`. Then `ω ≤ K(order \ X, cL)`. The proof is
  induction on the order, the paper's four cases folded into two.
* `online_le_K_any`: the same for any `X` with `wU(X) ≤ cU`, since blocked
  items outside the order do not matter to ω. With `order = [i, …, n]` this
  is the paper's statement at index `i`, where `K(X ∪ {1, …, i - 1}, cL)` is
  the knapsack over `{i, …, n} \ X`.
* `online_lower_bound`: the root form. For any order of distinct items,
  possibly only some of the items, `ω(order, C^U, C^L) ≤ K(univ \ X, C^L)`
  for every `X` with `wU(X) ≤ C^U`. So ω is at most the minimum over leader
  sets, a lower bound on the bilevel optimum.
* The paper's Table 1 is checked by `decide`: `table1_online` (ω = 1),
  `table1_opt` (every leader choice within budget leaves the follower at
  least 2) and `table1_block` (blocking item 3 achieves 2).

Rust: `mkip::omega_root` computes this recursion in its loop over `order`
(items with positive profit, in decreasing efficiency). The formal statement
covers it with one leader row and no scaling (`mu = 1`, `su = sl = 1`). The
code also merges several leader rows with multipliers, rounds the merged
costs down, and rounds the follower's weights up and its capacity down.
These steps only enlarge the leader's options and shrink the follower's.
That argument is in the doc comment of `omega_root`; it is not formalised.

### 4b. The robust table (`Bobi/RobustTable.lean`)

This is the threshold version of the same argument, for subset-sum
interdiction in the count model. A `Class` has a `value` and an ascending
list of `costs`. `H cs B t` is the recursion of `RobustTable` (see its doc
comment): for every count `r` of copies deleted from the first class within
budget, some number `j` of the remaining copies leaves `t - j·v` in `H` for
the next classes, with the budget reduced by what was spent.

* `H_sound`: if `t ∈ H(cs, B)`, then for every deletion of `rs[k] ≤ n_k`
  copies per class that costs at most `B` (cheapest first), `t` is a subset
  sum of the remaining copies.
* `H_sound_any`: the same for deletions of arbitrary copies. With costs
  ascending, `take_sum_le` shows that deleting the `r` cheapest copies costs
  no more than deleting any `r`.
* `covers_sound`: the pruning rule `RobustTable::covers` (`safe_online`). If
  the decided survivors reach `s` and `t ∈ H` for the classes still
  undecided, the follower reaches `s + t` against every deletion within
  budget.

The Rust bitsets hold `H(k, b) ∩ [0, hi]` for every `b ≤ budget`. Truncating
at `hi` only drops elements, so it keeps the bitsets sound.

### 4c. Band tables (`Bobi/BandTable.lean`)

`G w cs B t` is the table of `RobustTable::build_band(classes, hi, budget, w)`.
It has the recursion of `H` with the base case `G_w(m, B) = [0, w - 1]`
instead of `{0}`. So `t ∈ G_w` says the undecided classes can be made to
contribute a sum in `[t - w + 1, t]`, not exactly `t`.

* `G_sound`: if `t ∈ G_w(cs, B)`, then for every deletion of `rs[k] ≤ n_k`
  copies per class that costs at most `B`, the remaining copies have a subset
  sum `s` with `s ≤ t < s + w`. The proof is the induction of `H_sound`.
* `G_sound_any`: the same for deletions of arbitrary copies, as in
  `H_sound_any`.
* `band_covers_sound`: the pruning rule `BandTables::covers`. Suppose the
  decided survivors reach every sum in a run `[a, b]`, and `t ∈ G_w` for the
  undecided classes with `[target - t, target - t + w - 1]` inside `[a, b]`
  (stated as `a + t ≤ target` and `target + w ≤ b + t + 1`, which are the
  code's `t ≤ t_hi` and `t ≥ t_lo`). Then `target` is a subset sum of the
  decided values plus the survivors against every deletion within budget.
  The undecided classes give some `s ∈ [t - w + 1, t]`, and the run supplies
  `target - s`. The code's test `w ≤ b - a + 1` follows from the two
  inequalities, so the theorem does not need it.
* `G_one`: `G 1 = H`, since `t < 1` means `t = 0`. This is why
  `RobustTable::build` is `build_band(.., 1)`. `G_sound_one` and
  `covers_sound_of_band` get `H_sound` and `covers_sound` back from the band
  theorems (a one-point run `[s, s]` and `w = 1`).

As for `H`, truncating the bitsets at `hi` only drops elements. The
recursion is monotone in the next table, so it stays sound.

### 4d. Threshold form, play orders and per-node tables (not formalised)

`robust_ss.rs` now stores each table as one number per `(k, t)`: the
largest budget `B` with `t ∈ G_w(k, B)`. This relies on `G_w` being antitone
in `B` (induction on the classes; written out, with the min-max recursion it
gives, in the doc comment of `RobustTable`), and the test
`threshold_matches_bitset` compares it with the bitset recursion. Tables may
play the undecided classes in any order: `G_sound` is stated for any list
`cs`, so it covers every order. The per-node tables
(`Search::node_table_threshold`) replace the base case `[0, w - 1]` by an
arbitrary set `A` (the node's decided sums, widened by `hi - lo` for a band
of targets); the statement and proof are those of `G_sound` with `t < w`
replaced by `t ∈ A`. None of this is in Lean yet; the randomized tests
`tables_sound_by_enumeration`, `node_table_matches_definition_and_is_sound`
and `matches_brute_force` check it against exhaustive enumeration.

### 5. Clique interdiction bounds (`Bobi/Clique.lean`)

In BOBILib's edge clique interdiction instances (`clique`), the leader
deletes at most `k` of the `m` edges of a graph on `n` vertices, and the
follower takes a largest clique of what is left. These theorems back the
clique section of `../REPORT.md` (the density-0.7 instances closed at 3 and
the upper bounds from partitions). The Rust code has no clique solver; the report
keeps the certificates in `results/clique/`. `G.CliqueFree 3` is Mathlib's "no
triangle".

* `mantel` (Mantel's theorem): a graph on `n` vertices with more than
  `⌊n² / 4⌋` edges has a triangle. It is Mathlib's Turán bound
  `CliqueFree.card_edgeFinset_le` for `r = 2` with
  `turanNumber_two : turanNumber n 2 = n ^ 2 / 4`. `mantel'` states it as
  `4 · #edges > n²`.
* `triangle_survives` (lower bound 3): if `G` has `m` edges on `n` vertices,
  `D` is any finite set of at most `k` edges (of `G` or not) and
  `m - k > ⌊n² / 4⌋`, then `G.deleteEdges D` has a triangle, whatever the
  leader deletes. At least `m - k` edges remain.
* `cliqueFree_of_partition` (upper bound `r`): if `f : V → Fin r` colours the
  vertices and `D` contains every edge of `G` whose ends have the same
  colour, then `G.deleteEdges D` has no clique on `r + 1` vertices: `f` is a
  proper colouring of what is left. A partition with at most `k` inside
  edges therefore certifies value `≤ r`.
* `mantel_40`, `mantel_50`, `mantel_60` check by `decide` that
  `m - k > ⌊n² / 4⌋` for `(n, m, k) = (40, 546, 136)`, `(50, 857, 214)` and
  `(60, 1239, 309)` (410 > 400, 643 > 625, 930 > 900). `clique40`,
  `clique50` and `clique60` apply `triangle_survives`: in any graph with
  those parameters a triangle survives every deletion of at most `k` edges.

## Not formalised

These parts have no Lean proof:

* the search itself: caching, node limits and the bitset code;
* the threshold form of the tables and the per-node tables with an
  arbitrary base set (section 4d);
* `restrict_copies`;
* the LP bracket in `mkip::lp_follower_bound`;
* the scaling, rounding and fractional relaxations noted above;
* for the clique instances: Brouwer's 1981 stability theorem (a graph with
  at least `t_3(n) - ⌊n/3⌋ + 2` edges and no `K4` is 3-partite), which gives
  the density-0.9 lower bounds of 4. It is not in Mathlib and is neither
  proved nor assumed here. The solver runs (CP-SAT, SCIP and the dedicated
  branch and bound) showing that every 3-partition of those graphs has more
  than `k` inside edges are not formalised either;
* that the instance files have the stated `n`, `m` and `k`, and that the
  certificate partitions have at most `k` inside edges. Lean does not read
  the instance files; the theorems take these numbers as hypotheses.

## Files

| File | Contents |
| --- | --- |
| `Bobi/Knapsack.lean` | follower model `K`, theorem 1, theorem 2 |
| `Bobi/SubsetSum.lean` | subset sums, windows, theorem 3 and the stall test |
| `Bobi/Online.lean` | ω and theorem 4, Table 1 example |
| `Bobi/RobustTable.lean` | robust table `H` and its soundness |
| `Bobi/BandTable.lean` | band tables `G_w`, their soundness, `G 1 = H` |
| `Bobi/Clique.lean` | Mantel, triangle after deletions, partition bound |
| `Check.lean` | `#print axioms` for the main theorems |
