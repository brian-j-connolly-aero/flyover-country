module

public import Bobi.SubsetSum

/-!
# The robust online table of `robust_ss.rs`

`RobustTable` is the threshold version of the online relaxation, for
subset-sum interdiction in the count formulation: items come in classes of
equal follower weight, and the leader deletes copies class by class.
`H cs B t` says the follower can hit the target `t` from the classes `cs`
whatever the leader deletes with budget `B`, in the game where the leader
decides one class at a time and the follower answers each class before the
leader moves on. As with ω, letting the leader react only helps it, so `H` is
sound for the real game (`H_sound`): every target in `H` is a subset sum of
the copies the leader leaves. `H_sound_any` extends this to deletions of any
copies, not only the cheapest, and `covers_sound` is the pruning rule
`RobustTable::covers` used by the search.
-/

@[expose] public section

namespace Bobi

/-- A follower weight class (`robust_ss::Class`): `value` is the follower
weight (and profit) of each copy, `costs` the leader's costs of deleting the
copies, ascending. -/
structure Class where
  value : ℕ
  costs : List ℕ

/-- Number of copies in the class. -/
abbrev Class.copies (c : Class) : ℕ := c.costs.length

/-- Cost of deleting the first `r` copies (the cheapest ones). -/
def Class.delCost (c : Class) (r : ℕ) : ℕ := (c.costs.take r).sum

/-- The robust online table `H(k, B)` of `RobustTable::build`, for the
classes `k..` given as the list `cs`:

`t ∈ H(k, B)` iff for every `r` copies deleted from class `k` (cost
`c_k(r) ≤ B`) some `j ≤ n_k - r` has `t - j v_k ∈ H(k + 1, B - c_k(r))`,

with `H(m, B) = {0}`. -/
def H : List Class → ℕ → ℕ → Prop
  | [], _, t => t = 0
  | c :: cs, B, t => ∀ r ≤ c.copies, c.delCost r ≤ B →
      ∃ j ≤ c.copies - r, j * c.value ≤ t ∧ H cs (B - c.delCost r) (t - j * c.value)

/-- The copies left when the leader deletes the `rs[k]` cheapest copies of
class `k` (the count formulation). -/
def remaining : List Class → List ℕ → List ℕ
  | c :: cs, r :: rs => List.replicate (c.copies - r) c.value ++ remaining cs rs
  | _, _ => []

/-- What those deletions cost the leader. -/
def totalCost : List Class → List ℕ → ℕ
  | c :: cs, r :: rs => c.delCost r + totalCost cs rs
  | _, _ => 0

/-- **The robust table is sound.** If `t ∈ H(cs, B)`, then for every
deletion within budget `B` (`rs[k] ≤ n_k` copies of each class), `t` is a
subset sum of the remaining copies. -/
theorem H_sound :
    ∀ (cs : List Class) (rs : List ℕ), List.Forall₂ (fun c r => r ≤ c.copies) cs rs →
      ∀ B t, H cs B t → totalCost cs rs ≤ B → SubsetSum (remaining cs rs) t
  | [], [], .nil, _, _, h, _ => h ▸ SubsetSum.zero _
  | c :: cs, r :: rs, .cons hr hrs, B, t, h, hB => by
    simp only [totalCost] at hB
    -- The table names the follower's answer `j` to this deletion.
    obtain ⟨j, hj, hjt, hH⟩ := h r hr (by omega)
    have := (H_sound cs rs hrs _ _ hH (by omega)).replicate_append (v := c.value) hj
    rwa [show j * c.value + (t - j * c.value) = t by omega] at this

/-- With costs ascending, deleting the `d.length` first copies costs no more
than deleting the copies `d`, whichever they are. -/
theorem take_sum_le : ∀ {l d : List ℕ}, l.Pairwise (· ≤ ·) → d.Sublist l →
    (l.take d.length).sum ≤ d.sum
  | [], d, _, hd => by simp [List.sublist_nil.1 hd]
  | _ :: _, [], _, _ => by simp
  | a :: l, x :: d, hl, hd => by
    rw [List.pairwise_cons] at hl
    cases hd with
    | cons _ h =>
      -- `a` is skipped, so `x` comes later and costs at least `a`.
      have hax := hl.1 x (h.subset List.mem_cons_self)
      have := take_sum_le hl.2 ((List.sublist_cons_self x d).trans h)
      simp only [List.length_cons, List.take_succ_cons, List.sum_cons]
      omega
    | cons_cons _ h =>
      have := take_sum_le hl.2 h
      simp only [List.length_cons, List.take_succ_cons, List.sum_cons]
      omega

/-- A deletion of arbitrary copies (`ds[k]` lists the costs of the copies
deleted from class `k`) has counts that are valid and cost no more in the
count formulation. -/
theorem counts_of_deletion :
    ∀ {cs : List Class} {ds : List (List ℕ)}, (∀ c ∈ cs, c.costs.Pairwise (· ≤ ·)) →
      List.Forall₂ (fun c d => d.Sublist c.costs) cs ds →
        List.Forall₂ (fun c r => r ≤ c.copies) cs (ds.map List.length) ∧
          totalCost cs (ds.map List.length) ≤ (ds.map List.sum).sum
  | [], [], _, .nil => ⟨.nil, by simp [totalCost]⟩
  | c :: cs, d :: ds, hs, .cons hd hds => by
    obtain ⟨h1, h2⟩ := counts_of_deletion (fun c' hc' => hs c' (List.mem_cons_of_mem _ hc')) hds
    have := take_sum_le (hs c List.mem_cons_self) hd
    refine ⟨.cons hd.length_le h1, ?_⟩
    simp only [List.map_cons, totalCost, List.sum_cons, Class.delCost]
    omega

/-- **The robust table, any deletion.** With each class's costs ascending,
every target in `H(cs, B)` is a subset sum of the copies left by any deletion
of total cost at most `B`. -/
theorem H_sound_any (cs : List Class) (hsorted : ∀ c ∈ cs, c.costs.Pairwise (· ≤ ·))
    (ds : List (List ℕ)) (hds : List.Forall₂ (fun c d => d.Sublist c.costs) cs ds)
    (B t : ℕ) (h : H cs B t) (hB : (ds.map List.sum).sum ≤ B) :
    SubsetSum (remaining cs (ds.map List.length)) t := by
  obtain ⟨h1, h2⟩ := counts_of_deletion hsorted hds
  exact H_sound cs _ h1 B t h (Nat.le_trans h2 hB)

/-- The pruning rule `RobustTable::covers`: if the decided survivors reach
`s` and `t ∈ H` for the undecided classes, the follower reaches `s + t`
against every deletion within budget, so a target `s + t` in the band makes
the node safe. -/
theorem covers_sound {decided : List ℕ} {s : ℕ} (hs : SubsetSum decided s)
    (cs : List Class) (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs)
    {B t : ℕ} (h : H cs B t) (hB : totalCost cs rs ≤ B) :
    SubsetSum (decided ++ remaining cs rs) (s + t) :=
  hs.append (H_sound cs rs hrs B t h hB)

end Bobi
