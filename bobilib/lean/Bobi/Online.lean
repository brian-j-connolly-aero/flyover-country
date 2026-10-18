module

public import Bobi.Knapsack
public import Mathlib.Data.Fin.VecNotation

/-!
# The online (2n-round) relaxation lower bound

Weninger and Fukasawa's lower bound ω, computed by `mkip::omega_root`
(N. Weninger and R. Fukasawa, "A Fast Combinatorial Algorithm for the Bilevel
Knapsack Problem with Interdiction Constraints", Section 3: the recursion ω,
Lemma 5 and Theorem 2; numbering as in their preprint). Items are taken in a
fixed order; at each item the leader first decides whether to block it, then
the follower decides whether to take it. Because the leader decides item by
item, it may react to what the follower did on earlier items, which only helps
the leader, so the value of this game is at most the bilevel optimum.

We state it for any processing order of any subset of the items (the Rust code
sorts by efficiency and skips items without profit), and for every leader set
`X` within budget, including items outside the order.
-/

@[expose] public section

namespace Bobi

open Finset

variable {n : ℕ} (p wU wL : Fin n → ℕ)

/-- The online relaxation `ω(order, cU, cL)`: `cU` is the leader's remaining
budget, `cL` the follower's remaining capacity. A branch that the budget or
the capacity cannot afford is not offered, which is the paper's `∞` / `-∞`
convention (and the `if u <= b` / `if l <= c` tests in `omega_root`). -/
def online : List (Fin n) → ℕ → ℕ → ℕ
  | [], _, _ => 0
  | j :: rest, cU, cL =>
    let follower :=
      if wL j ≤ cL then max (online rest cU cL) (online rest cU (cL - wL j) + p j)
      else online rest cU cL
    if wU j ≤ cU then min (online rest (cU - wU j) cL) follower else follower

/-- The induction behind Theorem 2: against any leader set `X` drawn from the
items of `order` that fits the budget, ω is at most the follower's knapsack
optimum on the items of `order` that `X` leaves. -/
theorem online_le_K :
    ∀ (order : List (Fin n)), order.Nodup → ∀ (cU cL : ℕ) (X : Finset (Fin n)),
      X ⊆ order.toFinset → ∑ j ∈ X, wU j ≤ cU →
        online p wU wL order cU cL ≤ K p wL (order.toFinset \ X) cL
  | [], _, _, _, _, _, _ => by simp [online]
  | j :: rest, hnd, cU, cL, X, hXs, hXc => by
    rw [List.nodup_cons] at hnd
    have ih := online_le_K rest hnd.2
    simp only [online]
    by_cases hjX : j ∈ X
    · -- The leader blocks `j`: follow the leader's branch.
      have hcost := Finset.add_sum_erase X wU hjX
      have hXs' : X.erase j ⊆ rest.toFinset := fun k hk => by
        have := hXs (Finset.mem_of_mem_erase hk)
        simp only [List.toFinset_cons, Finset.mem_insert, List.mem_toFinset] at this
        exact List.mem_toFinset.2 (this.resolve_left (Finset.ne_of_mem_erase hk))
      have hset : (j :: rest).toFinset \ X = rest.toFinset \ X.erase j := by
        ext k
        simp only [List.toFinset_cons, Finset.mem_sdiff, Finset.mem_insert, Finset.mem_erase]
        constructor
        · rintro ⟨rfl | hk, hkX⟩
          · exact absurd hjX hkX
          · exact ⟨hk, fun h => hkX h.2⟩
        · rintro ⟨hk, hkX⟩
          exact ⟨Or.inr hk, fun h => hkX ⟨fun e => hnd.1 (e ▸ List.mem_toFinset.1 hk), h⟩⟩
      have hu : wU j ≤ cU := by omega
      simp only [hu, ↓reduceIte]
      rw [hset]
      exact (min_le_left _ _).trans (ih _ _ _ hXs' (by omega))
    · -- `j` is available: follow the follower's branch.
      have hXs' : X ⊆ rest.toFinset := fun k hk => by
        have := hXs hk
        simp only [List.toFinset_cons, Finset.mem_insert, List.mem_toFinset] at this
        exact List.mem_toFinset.2 (this.resolve_left fun e => hjX (e ▸ hk))
      have hset : (j :: rest).toFinset \ X = insert j (rest.toFinset \ X) := by
        rw [List.toFinset_cons, Finset.insert_sdiff_of_notMem _ hjX]
      have hj : j ∉ rest.toFinset \ X := fun h =>
        hnd.1 (List.mem_toFinset.1 (Finset.mem_sdiff.1 h).1)
      have hF : (if wL j ≤ cL then
            max (online p wU wL rest cU cL) (online p wU wL rest cU (cL - wL j) + p j)
          else online p wU wL rest cU cL) ≤ K p wL (insert j (rest.toFinset \ X)) cL := by
        split_ifs with hw
        · exact max_le ((ih _ _ _ hXs' hXc).trans (K_mono (Finset.subset_insert _ _) _))
            ((Nat.add_le_add_right (ih _ _ _ hXs' hXc) _).trans (K_insert hj hw))
        · exact (ih _ _ _ hXs' hXc).trans (K_mono (Finset.subset_insert _ _) _)
      rw [hset]
      by_cases hu : wU j ≤ cU
      · simp only [hu, ↓reduceIte]
        exact (min_le_right _ _).trans hF
      · simp only [hu, ↓reduceIte]
        exact hF

/-- `online_le_K` for any leader set within budget: blocked items outside the
order do not matter to ω, and the follower is still restricted to the order. -/
theorem online_le_K_any (order : List (Fin n)) (hnd : order.Nodup) (cU cL : ℕ)
    (X : Finset (Fin n)) (hX : ∑ j ∈ X, wU j ≤ cU) :
    online p wU wL order cU cL ≤ K p wL (order.toFinset \ X) cL := by
  have hle : ∑ j ∈ X ∩ order.toFinset, wU j ≤ cU :=
    (Finset.sum_le_sum_of_subset Finset.inter_subset_left).trans hX
  have h := online_le_K p wU wL order hnd cU cL _ Finset.inter_subset_right hle
  have hset : order.toFinset \ (X ∩ order.toFinset) = order.toFinset \ X := by
    ext k
    simp only [Finset.mem_sdiff, Finset.mem_inter]
    tauto
  rwa [hset] at h

/-- **Theorem 4 (Weninger and Fukasawa, Theorem 2).** For any order of
distinct items, `ω(order, C^U, C^L) ≤ min { K(items \ X, C^L) : wU(X) ≤ C^U }`:
whatever the leader blocks within budget, the follower can still collect ω.
The paper's statement at index `i` is `online_le_K_any` with
`order = [i, …, n]`; this is the root form, with every item available. -/
theorem online_lower_bound (order : List (Fin n)) (hnd : order.Nodup) (CU CL : ℕ)
    (X : Finset (Fin n)) (hX : ∑ j ∈ X, wU j ≤ CU) :
    online p wU wL order CU CL ≤ K p wL (univ \ X) CL :=
  (online_le_K_any p wU wL order hnd CU CL X hX).trans
    (K_mono (Finset.sdiff_subset_sdiff (Finset.subset_univ _) (subset_refl _)) CL)

/-! ## The paper's example: the bound is not tight

Table 1 of the paper: three items with `p = (1, 1, 2)`, `wU = (1, 1, 2)`,
`wL = (1, 1, 3)`, `C^U = 2`, `C^L = 3`. The online game is worth 1, while the
leader cannot hold the follower below 2: blocking items 1 and 2 leaves item 3,
blocking item 3 leaves items 1 and 2. -/

section Example

/-- Profits, leader costs and follower weights of Table 1 (items `0, 1, 2`). -/
def exP : Fin 3 → ℕ := ![1, 1, 2]
def exWU : Fin 3 → ℕ := ![1, 1, 2]
def exWL : Fin 3 → ℕ := ![1, 1, 3]

/-- The online game on Table 1 is worth 1. -/
theorem table1_online : online exP exWU exWL [0, 1, 2] 2 3 = 1 := by decide

/-- Blocking item 3 holds the follower to 2. -/
theorem table1_block : K exP exWL (univ \ {2}) 3 = 2 := by decide

/-- No leader choice within budget holds the follower below 2, so the bilevel
optimum of Table 1 is 2, twice ω. -/
theorem table1_opt :
    ∀ X : Finset (Fin 3), ∑ j ∈ X, exWU j ≤ 2 → 2 ≤ K exP exWL (univ \ X) 3 := by
  decide

end Example

end Bobi
