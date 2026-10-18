module

public import Mathlib.Algebra.Order.BigOperators.Group.Finset
public import Mathlib.Data.Finset.Lattice.Fold
public import Mathlib.Data.Finset.Powerset
public import Mathlib.Data.Fintype.Powerset
public import Mathlib.Logic.Equiv.Basic

/-!
# The follower's knapsack, and two facts about blocking items

A packing interdiction game (`structure.rs`): the leader blocks a set `X` of
items, paying `cost` per item, then the follower packs the remaining items
into a knapsack. Items are `Fin n`; the follower's best value is the knapsack
optimum over the items outside `X` (`Kip::follower`).

* `follower_antitone` (theorem 1): blocking more items never helps the
  follower. This is why blocking free items is harmless (`Kip::free_items`).
* `swap_block` and `cheapest_first` (theorem 2): items with the same follower
  weight and profit are interchangeable, so a leader may always block the
  cheapest copies of each such class. This is the count formulation of
  `robust_ss`.
-/

@[expose] public section

namespace Bobi

open Finset

/-! ## Any packing follower -/

section Packing

variable {ι : Type*} [DecidableEq ι]

/-- The follower's best value when the leader blocks `X`: the most profit of a
feasible set (a member of `F`) that avoids `X`. -/
def bestAvoiding (F : Finset (Finset ι)) (p : ι → ℕ) (X : Finset ι) : ℕ :=
  (F.filter fun Y => Disjoint Y X).sup fun Y => ∑ j ∈ Y, p j

/-- Blocking more items leaves the follower fewer options. -/
theorem bestAvoiding_antitone (F : Finset (Finset ι)) (p : ι → ℕ) {X X' : Finset ι}
    (h : X ⊆ X') : bestAvoiding F p X' ≤ bestAvoiding F p X :=
  Finset.sup_mono fun Y hY => by
    rw [Finset.mem_filter] at hY ⊢
    exact ⟨hY.1, hY.2.mono_right h⟩

/-- For a packing follower (feasible sets closed under taking subsets),
blocking an item is the same as making it worthless: the follower may pick any
feasible set and simply lose the blocked part. In this reading monotonicity is
plain too, since blocking more only lowers profits. -/
theorem bestAvoiding_eq_sup_sdiff (F : Finset (Finset ι)) (p : ι → ℕ) (X : Finset ι)
    (hF : ∀ Y ∈ F, ∀ Z ⊆ Y, Z ∈ F) :
    bestAvoiding F p X = F.sup fun Y => ∑ j ∈ Y \ X, p j := by
  apply le_antisymm
  · refine Finset.sup_le fun Y hY => ?_
    rw [Finset.mem_filter] at hY
    rw [← Finset.sdiff_eq_self_of_disjoint hY.2]
    exact Finset.le_sup (f := fun Y => ∑ j ∈ Y \ X, p j) hY.1
  · refine Finset.sup_le fun Y hY => ?_
    refine Finset.le_sup (f := fun Y => ∑ j ∈ Y, p j) ?_
    rw [Finset.mem_filter]
    exact ⟨hF Y hY _ Finset.sdiff_subset, Finset.sdiff_disjoint⟩

end Packing

/-! ## The knapsack follower -/

section Knapsack

variable {n : ℕ} (p w : Fin n → ℕ)

/-- The knapsack optimum over the items of `S` with capacity `c`: the most
profit `p` of a subset of `S` whose weight `w` is at most `c`. -/
def K (S : Finset (Fin n)) (c : ℕ) : ℕ :=
  (S.powerset.filter fun Y => ∑ j ∈ Y, w j ≤ c).sup fun Y => ∑ j ∈ Y, p j

/-- The follower's feasible sets: those of weight at most `C`. -/
def packings (C : ℕ) : Finset (Finset (Fin n)) :=
  univ.filter fun Y => ∑ j ∈ Y, w j ≤ C

variable {p w}

theorem le_K {S Y : Finset (Fin n)} {c : ℕ} (hY : Y ⊆ S) (hw : ∑ j ∈ Y, w j ≤ c) :
    ∑ j ∈ Y, p j ≤ K p w S c :=
  Finset.le_sup (f := fun Y => ∑ j ∈ Y, p j)
    (Finset.mem_filter.2 ⟨Finset.mem_powerset.2 hY, hw⟩)

/-- Some feasible packing attains the optimum. -/
theorem exists_K (S : Finset (Fin n)) (c : ℕ) :
    ∃ Y ⊆ S, ∑ j ∈ Y, w j ≤ c ∧ ∑ j ∈ Y, p j = K p w S c := by
  have hne : (S.powerset.filter fun Y => ∑ j ∈ Y, w j ≤ c).Nonempty :=
    ⟨∅, Finset.mem_filter.2 ⟨Finset.empty_mem_powerset S, by simp⟩⟩
  obtain ⟨Y, hY, hsup⟩ := Finset.exists_mem_eq_sup _ hne fun Y => ∑ j ∈ Y, p j
  rw [Finset.mem_filter, Finset.mem_powerset] at hY
  exact ⟨Y, hY.1, hY.2, hsup.symm⟩

/-- More available items can only help the follower. -/
theorem K_mono {S T : Finset (Fin n)} (h : S ⊆ T) (c : ℕ) : K p w S c ≤ K p w T c := by
  obtain ⟨Y, hYS, hYw, hYp⟩ := exists_K (p := p) (w := w) S c
  exact hYp ▸ le_K (hYS.trans h) hYw

/-- Taking an item `j` on top of a packing of the rest (the knapsack recursion,
as a lower bound). -/
theorem K_insert {S : Finset (Fin n)} {j : Fin n} {c : ℕ} (hj : j ∉ S) (hc : w j ≤ c) :
    K p w S (c - w j) + p j ≤ K p w (insert j S) c := by
  obtain ⟨Y, hYS, hYw, hYp⟩ := exists_K (p := p) (w := w) S (c - w j)
  have hjY : j ∉ Y := fun h => hj (hYS h)
  have := le_K (p := p) (Finset.insert_subset_insert j hYS)
    (show ∑ k ∈ insert j Y, w k ≤ c by rw [Finset.sum_insert hjY]; omega)
  rw [Finset.sum_insert hjY] at this
  omega

/-- **Theorem 1 (monotonicity of interdiction).** The follower's best value
`K (univ \ X) C` can only go down when the leader blocks more items. -/
theorem follower_antitone (C : ℕ) {X X' : Finset (Fin n)} (h : X ⊆ X') :
    K p w (univ \ X') C ≤ K p w (univ \ X) C :=
  K_mono (Finset.sdiff_subset_sdiff (subset_refl _) h) C

/-- Knapsack packings are closed under taking subsets (weights are
non-negative), so the knapsack follower is a packing follower. -/
theorem packings_downward_closed (C : ℕ) : ∀ Y ∈ packings w C, ∀ Z ⊆ Y, Z ∈ packings w C := by
  intro Y hY Z hZ
  simp only [packings, Finset.mem_filter, Finset.mem_univ, true_and] at hY ⊢
  exact (Finset.sum_le_sum_of_subset hZ).trans hY

/-- The knapsack follower's value is `bestAvoiding` over its packings. -/
theorem K_eq_bestAvoiding (C : ℕ) (X : Finset (Fin n)) :
    K p w (univ \ X) C = bestAvoiding (packings w C) p X := by
  unfold K bestAvoiding packings
  congr 1
  ext Y
  simp [Finset.subset_sdiff, and_comm]

/-- Relabelling the items by a permutation that keeps every profit and weight
does not change the knapsack optimum. -/
theorem K_map_equiv (σ : Fin n ≃ Fin n) (hp : ∀ j, p (σ j) = p j) (hw : ∀ j, w (σ j) = w j)
    (S : Finset (Fin n)) (c : ℕ) : K p w (S.map σ.toEmbedding) c = K p w S c := by
  -- Moving a packing along a profit- and weight-preserving permutation.
  have key : ∀ τ : Fin n ≃ Fin n, (∀ j, p (τ j) = p j) → (∀ j, w (τ j) = w j) →
      ∀ S, K p w S c ≤ K p w (S.map τ.toEmbedding) c := by
    intro τ hp hw S
    obtain ⟨Y, hYS, hYw, hYp⟩ := exists_K (p := p) (w := w) S c
    have hsum : ∀ f : Fin n → ℕ, (∀ j, f (τ j) = f j) →
        ∑ j ∈ Y.map τ.toEmbedding, f j = ∑ j ∈ Y, f j := by
      intro f hf
      simp [Finset.sum_map, hf]
    rw [← hYp, ← hsum p hp]
    exact le_K (Finset.map_subset_map.2 hYS) (by rw [hsum w hw]; exact hYw)
  refine le_antisymm ?_ (key σ hp hw S)
  have hp' : ∀ j, p (σ.symm j) = p j := fun j => by rw [← hp (σ.symm j), σ.apply_symm_apply]
  have hw' : ∀ j, w (σ.symm j) = w j := fun j => by rw [← hw (σ.symm j), σ.apply_symm_apply]
  have := key σ.symm hp' hw' (S.map σ.toEmbedding)
  have hS : (S.map σ.toEmbedding).map σ.symm.toEmbedding = S := by ext k; simp
  rwa [hS] at this

/-- **Theorem 2 (count formulation).** If items `i` and `j` have the same
follower profit and weight, a leader that blocks `i` but not `j` can block `j`
instead: the follower's best value is unchanged, and the leader spends no more
when `j` is the cheaper copy. -/
theorem swap_block (cost : Fin n → ℕ) (C : ℕ) {i j : Fin n} (hp : p i = p j) (hw : w i = w j)
    (hc : cost j ≤ cost i) {X : Finset (Fin n)} (hi : i ∈ X) (hj : j ∉ X) :
    K p w (univ \ insert j (X.erase i)) C = K p w (univ \ X) C ∧
      ∑ k ∈ insert j (X.erase i), cost k ≤ ∑ k ∈ X, cost k := by
  constructor
  · -- The swapped leader set is the image of `X` under the transposition.
    have hσp : ∀ k, p (Equiv.swap i j k) = p k := fun k => by
      rw [Equiv.swap_apply_def]; split_ifs <;> simp_all
    have hσw : ∀ k, w (Equiv.swap i j k) = w k := fun k => by
      rw [Equiv.swap_apply_def]; split_ifs <;> simp_all
    have himage : (univ \ X).map (Equiv.swap i j).toEmbedding = univ \ insert j (X.erase i) := by
      ext k
      simp only [Finset.mem_map_equiv, Equiv.symm_swap, Finset.mem_sdiff, Finset.mem_univ,
        true_and, Finset.mem_insert, Finset.mem_erase, Equiv.swap_apply_def]
      split_ifs with h1 h2
      · subst h1; simp [hj, ne_of_mem_of_not_mem hi hj]
      · subst h2; simp [hi, Ne.symm (ne_of_mem_of_not_mem hi hj)]
      · simp [h1, h2]
    rw [← himage, K_map_equiv _ hσp hσw]
  · have hj' : j ∉ X.erase i := fun h => hj (Finset.mem_of_mem_erase h)
    rw [Finset.sum_insert hj', ← Finset.add_sum_erase X cost hi]
    omega

/-- Repeating `swap_block`: inside a class of identical copies, the leader may
block the cheapest ones. For every leader set `X` there is `X'` that agrees
with `X` outside the class, blocks as many copies, blocks only copies no
costlier than the copies it leaves, costs no more, and leaves the follower the
same best value. -/
theorem cheapest_first (cost : Fin n → ℕ) (C : ℕ) (cls : Finset (Fin n))
    (hcls : ∀ a ∈ cls, ∀ b ∈ cls, p a = p b ∧ w a = w b) (X : Finset (Fin n)) :
    ∃ X' : Finset (Fin n), X' \ cls = X \ cls ∧ (X' ∩ cls).card = (X ∩ cls).card ∧
      (∀ a ∈ X' ∩ cls, ∀ b ∈ cls \ X', cost a ≤ cost b) ∧
      ∑ k ∈ X', cost k ≤ ∑ k ∈ X, cost k ∧ K p w (univ \ X') C = K p w (univ \ X) C := by
  -- Strong induction on the leader's cost: each improving swap lowers it.
  induction hm : ∑ k ∈ X, cost k using Nat.strong_induction_on generalizing X with
  | _ m ih =>
    by_cases hbad : ∃ a ∈ X ∩ cls, ∃ b ∈ cls \ X, cost b < cost a
    · obtain ⟨a, ha, b, hb, hlt⟩ := hbad
      rw [Finset.mem_inter] at ha
      rw [Finset.mem_sdiff] at hb
      obtain ⟨hpab, hwab⟩ := hcls a ha.2 b hb.1
      obtain ⟨hK, hcost⟩ := swap_block (p := p) (w := w) cost C hpab hwab hlt.le ha.1 hb.2
      -- The swap strictly lowers the cost.
      have hlt' : ∑ k ∈ insert b (X.erase a), cost k < m := by
        have hb' : b ∉ X.erase a := fun h => hb.2 (Finset.mem_of_mem_erase h)
        rw [Finset.sum_insert hb', ← hm, ← Finset.add_sum_erase X cost ha.1]
        omega
      obtain ⟨X', h1, h2, h3, h4, h5⟩ := ih _ hlt' (insert b (X.erase a)) rfl
      refine ⟨X', ?_, ?_, h3, by omega, h5.trans hK⟩
      · rw [h1]
        ext k
        simp only [Finset.mem_sdiff, Finset.mem_insert, Finset.mem_erase]
        constructor
        · rintro ⟨rfl | ⟨-, hk⟩, hkc⟩
          · exact absurd hb.1 hkc
          · exact ⟨hk, hkc⟩
        · rintro ⟨hk, hkc⟩
          exact ⟨Or.inr ⟨fun h => hkc (h ▸ ha.2), hk⟩, hkc⟩
      · rw [h2, Finset.insert_inter_of_mem hb.1, Finset.erase_inter,
          Finset.card_insert_of_notMem
            (fun h => hb.2 (Finset.mem_inter.1 (Finset.mem_of_mem_erase h)).1),
          Finset.card_erase_add_one (Finset.mem_inter.2 ha)]
    · simp only [not_exists, not_and, not_lt] at hbad
      exact ⟨X, rfl, rfl, fun a ha b hb => hbad a ha b hb, hm.le, rfl⟩

end Knapsack

end Bobi
