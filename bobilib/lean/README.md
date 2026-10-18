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
clean `lake build` takes about 40 s and under 2 GB of memory. There is no
`sorry`: `Check.lean` shows only Lean's standard axioms (`propext`,
`Classical.choice`, `Quot.sound`).

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

Rust: `Kip::free_items` blocks items that cost the leader nothing and
`Kip::greedy` blocks them too. In `robust_ss::Search::node`, the copies the
leader cannot afford to delete survive whatever it does, and so do their sums
(the `guaranteed` test).

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

This is Weninger and Fukasawa's Theorem 2. `online p wU wL order cU cL` is
their recursive ω. Items are taken in the order `order`. At each item the
leader may block it if the budget `cU` allows. If the leader does not, the
follower may take it if the capacity `cL` allows. A branch that cannot be
afforded is not offered, which is the paper's `±∞` convention.

* `online_le_K`: suppose `X` is drawn from `order`, its items are distinct,
  and `wU(X) ≤ cU`. Then `ω ≤ K(order \ X, cL)`. The proof is induction on
  the order, the paper's four cases folded into two.
* `online_lower_bound`: for any order of distinct items, possibly only some
  of the items, `ω(order, C^U, C^L) ≤ K(univ \ X, C^L)` for every `X` with
  `wU(X) ≤ C^U`. So ω is at most the minimum over leader sets, a lower bound
  on the bilevel optimum. With `order = [i, …, n]` this is the paper's
  statement at index `i`.
* The paper's Table 1 is checked by `decide`. There ω = 1, while every leader
  choice within budget leaves the follower at least 2.

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

## Not formalised

These parts have no Lean proof:

* the search itself: caching, node limits and the bitset code;
* `restrict_copies`;
* the LP bracket in `mkip::lp_follower_bound`;
* the scaling, rounding and fractional relaxations noted above.

## Files

| File | Contents |
| --- | --- |
| `Bobi/Knapsack.lean` | follower model `K`, theorem 1, theorem 2 |
| `Bobi/SubsetSum.lean` | subset sums, windows, theorem 3 and the stall test |
| `Bobi/Online.lean` | ω and theorem 4, Table 1 example |
| `Bobi/RobustTable.lean` | robust table `H` and its soundness |
| `Check.lean` | `#print axioms` for the main theorems |
