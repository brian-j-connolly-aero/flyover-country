module

public import Bobi.EndTable

/-!
# The threshold form of the robust tables

`robust_ss.rs` stores each robust table as one number per `(k, t)` instead of
one set per budget. The tables are antitone in the budget (`GA_antitone`), so
`t ∈ G_A(k, B)` holds exactly for the budgets `B` up to a threshold. With the
budgets capped at `cap` (the root budget), `thr A cap cs t` is that threshold,
computed by the min-max recursion of `RobustTable::step`:

```
thr(t) = min(cap, min over r ≤ n with P(r) ≤ cap of
                    P(r) + max over j ≤ n - r with j v ≤ t of thr'(t - j v))
```

with `P(r)` the cost of the `r` cheapest copies of the first class played and
`thr'` the threshold of the classes after it; the base is `cap` on the end set
`A` and `-1` off it (`RobustTable::base`, and the base array of
`Search::node_table_threshold`).

* `thr_iff`: `B ≤ thr(t)` iff `B ≤ cap` and `t ∈ G_A(B)`.
* `thr_bounds`, `thr_eq_neg_one_iff`, `thr_greatest`: `thr(t) ∈ [-1, cap]`;
  `-1` means `t` is in the table at no budget, and otherwise `thr(t)` is the
  largest budget at which it is.
* `G_iff_thr`, `H_iff_thr`: the same for the band tables and `H`;
  `covers_threshold_sound(_perm)`, `band_covers_threshold_sound(_perm)`: the
  root rules on stored thresholds, in the search order or another play order.
* `node_table_threshold_sound_counts`: the node rule in the form the search
  uses (count formulation, any play order, any cost order);
  `node_table_threshold_sound` for deletions of arbitrary copies.
-/

@[expose] public section

namespace Bobi

/-- `min(cap, min over x ∈ l of f x)`. -/
def minOver (cap : ℤ) (l : List ℕ) (f : ℕ → ℤ) : ℤ :=
  l.foldr (fun x m => min (f x) m) cap

/-- `max(d, max over x ∈ l of f x)`. -/
def maxOver (d : ℤ) (l : List ℕ) (f : ℕ → ℤ) : ℤ :=
  l.foldr (fun x m => max (f x) m) d

theorem le_minOver {B cap : ℤ} {f : ℕ → ℤ} (l : List ℕ) :
    B ≤ minOver cap l f ↔ B ≤ cap ∧ ∀ x ∈ l, B ≤ f x := by
  induction l with
  | nil => simp [minOver]
  | cons x l ih =>
    simp only [minOver, List.foldr_cons] at ih ⊢
    have e : ∀ a m : ℤ, B ≤ min a m ↔ B ≤ a ∧ B ≤ m := fun a m => by omega
    rw [e, ih]
    simp only [List.mem_cons, forall_eq_or_imp]
    tauto

theorem le_maxOver {B d : ℤ} {f : ℕ → ℤ} (l : List ℕ) :
    B ≤ maxOver d l f ↔ B ≤ d ∨ ∃ x ∈ l, B ≤ f x := by
  induction l with
  | nil => simp [maxOver]
  | cons x l ih =>
    simp only [maxOver, List.foldr_cons] at ih ⊢
    have e : ∀ a m : ℤ, B ≤ max a m ↔ B ≤ a ∨ B ≤ m := fun a m => by omega
    rw [e, ih]
    simp only [List.mem_cons, exists_eq_or_imp]
    tauto

/-- The threshold of `t` for the table with end set `A` of the classes `cs`
(in play order), with budgets capped at `cap`: the recursion of
`RobustTable::step` (`P(r) = c.delCost r`; the deletions `r` with
`P(r) > cap` are skipped, as the code's prefix array stops there). -/
def thr (A : ℕ → Prop) [DecidablePred A] (cap : ℕ) : List Class → ℕ → ℤ
  | [], t => if A t then (cap : ℤ) else -1
  | c :: cs, t =>
    minOver cap ((List.range (c.copies + 1)).filter fun r => c.delCost r ≤ cap) fun r =>
      (c.delCost r : ℤ) +
        maxOver (-1) ((List.range (c.copies - r + 1)).filter fun j => j * c.value ≤ t)
          fun j => thr A cap cs (t - j * c.value)

/-- **The threshold form.** `B ≤ thr(t)` exactly when `B ≤ cap` and
`t ∈ G_A(cs, B)`. (Deletion `r` defeats the follower at budget `B` iff
`P(r) ≤ B` and `B - P(r) > thr'(t - j v)` for every answer `j`, that is
`B > P(r) + max_j thr'(t - j v)`.) -/
theorem thr_iff (A : ℕ → Prop) [DecidablePred A] (cap : ℕ) :
    ∀ (cs : List Class) (B t : ℕ), (B : ℤ) ≤ thr A cap cs t ↔ B ≤ cap ∧ GA A cs B t
  | [], B, t => by
    by_cases hA : A t
    · simp [thr, GA, hA]
    · simp only [thr, GA, hA, ↓reduceIte, and_false, iff_false]
      omega
  | c :: cs, B, t => by
    simp only [thr, GA]
    rw [le_minOver]
    constructor
    · rintro ⟨hcap, hall⟩
      refine ⟨by exact_mod_cast hcap, fun r hr hc => ?_⟩
      have h1 := hall r (by simp only [List.mem_filter, List.mem_range, decide_eq_true_eq]; omega)
      have h2 : ((B - c.delCost r : ℕ) : ℤ) ≤
          maxOver (-1) ((List.range (c.copies - r + 1)).filter fun j => j * c.value ≤ t)
            (fun j => thr A cap cs (t - j * c.value)) := by
        omega
      rcases (le_maxOver _).1 h2 with h3 | ⟨j, hj, h3⟩
      · omega
      · simp only [List.mem_filter, List.mem_range, decide_eq_true_eq] at hj
        exact ⟨j, by omega, hj.2, ((thr_iff A cap cs _ _).1 h3).2⟩
    · rintro ⟨hcap, hG⟩
      refine ⟨by exact_mod_cast hcap, fun r hr => ?_⟩
      simp only [List.mem_filter, List.mem_range, decide_eq_true_eq] at hr
      by_cases hc : c.delCost r ≤ B
      · obtain ⟨j, hj, hjt, hG'⟩ := hG r (by omega) hc
        have h2 : ((B - c.delCost r : ℕ) : ℤ) ≤
            maxOver (-1) ((List.range (c.copies - r + 1)).filter fun j => j * c.value ≤ t)
              (fun j => thr A cap cs (t - j * c.value)) := by
          refine (le_maxOver _).2 (Or.inr ⟨j, ?_, (thr_iff A cap cs _ _).2 ⟨by omega, hG'⟩⟩)
          simp only [List.mem_filter, List.mem_range, decide_eq_true_eq]
          omega
        omega
      · have h2 : (-1 : ℤ) ≤
            maxOver (-1) ((List.range (c.copies - r + 1)).filter fun j => j * c.value ≤ t)
              (fun j => thr A cap cs (t - j * c.value)) := (le_maxOver _).2 (Or.inl (Int.le_refl _))
        omega

/-- For budgets up to `cap`, the table is the set of residuals whose
threshold is at least the budget (`RobustTable::contains`). -/
theorem GA_iff_thr (A : ℕ → Prop) [DecidablePred A] {cap B : ℕ} (hB : B ≤ cap)
    (cs : List Class) (t : ℕ) : GA A cs B t ↔ (B : ℤ) ≤ thr A cap cs t := by
  rw [thr_iff]
  exact ⟨fun h => ⟨hB, h⟩, fun h => h.2⟩

/-- Thresholds lie in `[-1, cap]`. -/
theorem thr_bounds (A : ℕ → Prop) [DecidablePred A] (cap : ℕ) :
    ∀ (cs : List Class) (t : ℕ), -1 ≤ thr A cap cs t ∧ thr A cap cs t ≤ cap
  | [], t => by
    by_cases hA : A t <;> simp only [thr, hA, ↓reduceIte] <;> omega
  | c :: cs, t => by
    simp only [thr]
    constructor
    · refine (le_minOver _).2 ⟨by omega, fun r _ => ?_⟩
      have : (-1 : ℤ) ≤
          maxOver (-1) ((List.range (c.copies - r + 1)).filter fun j => j * c.value ≤ t)
            (fun j => thr A cap cs (t - j * c.value)) := (le_maxOver _).2 (Or.inl (Int.le_refl _))
      omega
    · exact ((le_minOver _).1 (Int.le_refl _)).1

/-- A threshold of `-1` means the residual is in the table at no budget. -/
theorem thr_eq_neg_one_iff (A : ℕ → Prop) [DecidablePred A] (cap : ℕ) (cs : List Class)
    (t : ℕ) : thr A cap cs t = -1 ↔ ¬ GA A cs 0 t := by
  have hb := thr_bounds A cap cs t
  constructor
  · intro h hG
    have := (thr_iff A cap cs 0 t).2 ⟨Nat.zero_le _, hG⟩
    omega
  · intro h
    have : ¬ ((0 : ℕ) : ℤ) ≤ thr A cap cs t := fun h' => h ((thr_iff A cap cs 0 t).1 h').2
    omega

/-- `thr(t)` is the largest budget `B ≤ cap` with `t ∈ G_A(cs, B)` when it is
not `-1`. -/
theorem thr_greatest (A : ℕ → Prop) [DecidablePred A] (cap : ℕ) (cs : List Class) (t : ℕ)
    (h : 0 ≤ thr A cap cs t) :
    (thr A cap cs t).toNat ≤ cap ∧ GA A cs (thr A cap cs t).toNat t ∧
      ∀ B ≤ cap, GA A cs B t → (B : ℤ) ≤ thr A cap cs t := by
  have hn : ((thr A cap cs t).toNat : ℤ) = thr A cap cs t := Int.toNat_of_nonneg h
  have := (thr_iff A cap cs (thr A cap cs t).toNat t).1 (by omega)
  exact ⟨this.1, this.2, fun B hB hG => (thr_iff A cap cs B t).2 ⟨hB, hG⟩⟩

/-! ## Band tables and `H` -/

/-- The threshold form of the band table `G_w` (`RobustTable::build_band`,
`build_suffix`; base `cap` on `[0, w - 1]`, `RobustTable::base`). -/
theorem G_iff_thr (w cap : ℕ) (cs : List Class) (B t : ℕ) :
    (B : ℤ) ≤ thr (· < w) cap cs t ↔ B ≤ cap ∧ G w cs B t := by
  rw [thr_iff, G_iff_GA]

/-- The threshold form of `H` (`RobustTable::build`, width 1). -/
theorem H_iff_thr (cap : ℕ) (cs : List Class) (B t : ℕ) :
    (B : ℤ) ≤ thr (· = 0) cap cs t ↔ B ≤ cap ∧ H cs B t := by
  rw [thr_iff, H_iff_GA]

/-- `RobustTable::covers` in threshold form: `row[t] ≥ b` (with `b ≤ cap`)
and a decided sum `s` give `s + t` against every deletion within `b`. -/
theorem covers_threshold_sound {decided : List ℕ} {s : ℕ} (hs : SubsetSum decided s)
    (cap : ℕ) (cs : List Class) (rs : List ℕ)
    (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs) {B t : ℕ}
    (h : (B : ℤ) ≤ thr (· = 0) cap cs t) (hB : totalCost cs rs ≤ B) :
    SubsetSum (decided ++ remaining cs rs) (s + t) :=
  covers_sound hs cs rs hrs ((H_iff_thr cap cs B t).1 h).2 hB

/-- `RobustTable::covers_run` in threshold form (`band_covers_sound`). -/
theorem band_covers_threshold_sound {decided : List ℕ} {a b : ℕ} (hdec : Window decided a b)
    {w : ℕ} (cap : ℕ) (cs : List Class) (rs : List ℕ)
    (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs) {B t target : ℕ}
    (h : (B : ℤ) ≤ thr (· < w) cap cs t) (hB : totalCost cs rs ≤ B)
    (hlo : a + t ≤ target) (hhi : target + w ≤ b + t + 1) :
    SubsetSum (decided ++ remaining cs rs) target :=
  band_covers_sound hdec cs rs hrs ((G_iff_thr w cap cs B t).1 h).2 hB hlo hhi

/-- `covers_threshold_sound` for a table whose row plays the undecided classes
`cs` in another order `play` (the width-1 tables of
`RobustTable::build_suffix`, which `RobustTable::covers` also checks). -/
theorem covers_threshold_sound_perm {decided : List ℕ} {s : ℕ} (hs : SubsetSum decided s)
    (cap : ℕ) {cs play : List Class} (hplay : play.Perm cs) (rs : List ℕ)
    (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs) {B t : ℕ}
    (h : (B : ℤ) ≤ thr (· = 0) cap play t) (hB : totalCost cs rs ≤ B) :
    SubsetSum (decided ++ remaining cs rs) (s + t) := by
  obtain ⟨x, a, hx, ha, hxa⟩ :=
    GA_sound_perm _ hplay rs hrs ((thr_iff _ cap play B t).1 h).2 hB
  have ha' : a = 0 := ha
  rw [show t = x by omega]
  exact hs.append hx

/-- `band_covers_threshold_sound` for a table in another play order `play`
(`RobustTable::build_suffix` with `w > 1`, checked by
`RobustTable::covers_run`). -/
theorem band_covers_threshold_sound_perm {decided : List ℕ} {a b : ℕ}
    (hdec : Window decided a b) {w : ℕ} (cap : ℕ) {cs play : List Class} (hplay : play.Perm cs)
    (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs) {B t target : ℕ}
    (h : (B : ℤ) ≤ thr (· < w) cap play t) (hB : totalCost cs rs ≤ B)
    (hlo : a + t ≤ target) (hhi : target + w ≤ b + t + 1) :
    SubsetSum (decided ++ remaining cs rs) target := by
  obtain ⟨s, e, hs, he, hse⟩ :=
    GA_sound_perm _ hplay rs hrs ((thr_iff _ cap play B t).1 h).2 hB
  have he' : e < w := he
  have := (hdec (target - s) (by omega) (by omega)).append hs
  rwa [Nat.sub_add_cancel (by omega)] at this

/-! ## The per-node rule in threshold form -/

/-- **`Search::node_table_threshold`.** Let `S` be sums the decided survivors
reach (the bitset `sums`), `lo ≤ hi`, `play` the node's play order of the
undecided classes `cs` (any permutation), and `cap` the root budget. If
`B ≤ thr(hi)` for the end set `nodeEnd S lo hi`, then every deletion of
undecided copies (`ds[k]` the costs of those deleted from `cs[k]`) of cost at
most `B` leaves a subset of `decided ++ survivors` with sum in `[lo, hi]`.
So the node is safe at its budget when `thr(hi) ≥ budget`, and at every
budget up to `thr(hi)` (what `remember(k, sums, thr)` caches). -/
theorem node_table_threshold_sound {decided : List ℕ} {S : ℕ → Prop} [DecidablePred S]
    (hS : ∀ s, S s → SubsetSum decided s) {lo hi : ℕ} (hlh : lo ≤ hi) (cap : ℕ)
    {cs play : List Class} (hplay : play.Perm cs) (hsorted : ∀ c ∈ cs, c.costs.Pairwise (· ≤ ·))
    (ds : List (List ℕ)) (hds : List.Forall₂ (fun c d => d.Sublist c.costs) cs ds)
    {B : ℕ} (hthr : (B : ℤ) ≤ thr (nodeEnd S lo hi) cap play hi)
    (hB : (ds.map List.sum).sum ≤ B) :
    ∃ u, SubsetSum (decided ++ remaining cs (ds.map List.length)) u ∧ lo ≤ u ∧ u ≤ hi :=
  node_table_sound_any hS hlh hplay hsorted ds hds ((thr_iff _ cap play B hi).1 hthr).2 hB

/-- `node_table_threshold_sound` in the count formulation the search branches
on: `rs[k]` first copies of `cs[k]` deleted, `totalCost cs rs ≤ B` (the sum
of the code's `prefix[r]`). No order on the costs is needed, so this also
holds after `restrict_copies`. This is the statement the prune
`thr ≥ budget` in `Search::node` and the cache entry
`remember(k, sums, thr)` use. -/
theorem node_table_threshold_sound_counts {decided : List ℕ} {S : ℕ → Prop} [DecidablePred S]
    (hS : ∀ s, S s → SubsetSum decided s) {lo hi : ℕ} (hlh : lo ≤ hi) (cap : ℕ)
    {cs play : List Class} (hplay : play.Perm cs)
    (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs)
    {B : ℕ} (hthr : (B : ℤ) ≤ thr (nodeEnd S lo hi) cap play hi)
    (hB : totalCost cs rs ≤ B) :
    ∃ u, SubsetSum (decided ++ remaining cs rs) u ∧ lo ≤ u ∧ u ≤ hi :=
  node_table_sound_perm hS hlh hplay rs hrs ((thr_iff _ cap play B hi).1 hthr).2 hB

/-! ## A check by evaluation

One class of value 2 with deletion costs `[1, 2]`, end set `{0}` (the table
`H`) and `cap = 3`: the follower reaches 2 as long as the leader cannot delete
both copies, that is for budgets up to 2. With a class of value 1 and cost
`[1]` played after it, the target 3 survives only budget 0 (with budget 1 the
leader deletes the 1). -/

example : thr (· = 0) 3 [⟨2, [1, 2]⟩] 2 = 2 := by decide
example : thr (· = 0) 3 [⟨2, [1, 2]⟩, ⟨1, [1]⟩] 3 = 0 := by decide

/-- The node end set on a small node: decided sums `{0, 3}`, band `[4, 5]`,
one undecided copy of value 2 that costs 1. The end set is `{0, 1, 3, 4}`;
the node is safe at budget 0 (`3 + 2 = 5`) but not at budget 1 (deleting the
2 leaves `{0, 3}`), so `thr(hi) = 0`. -/
example : thr (nodeEnd (· ∈ [0, 3]) 4 5) 1 [⟨2, [1]⟩] 5 = 0 := by decide

end Bobi
