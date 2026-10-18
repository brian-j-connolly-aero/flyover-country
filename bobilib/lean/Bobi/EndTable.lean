module

public import Bobi.BandTable

/-!
# Tables with an arbitrary end set (the per-node tables)

`Search::node_table_threshold` in `robust_ss.rs` builds, at a search node, the
robust table of the undecided classes with the recursion of `H` and `G_w` but
an arbitrary *end set* `A` as its base case: the residual left once every
class has been played must lie in `A`. The node's end set is
`A = {a | some decided sum s has s ≤ a ≤ s + (hi - lo)}`, and the node is
safe when `hi` is in the table.

* `GA A cs B t`: the table with end set `A`. `G_iff_GA` and `H_iff_GA`: the
  band table `G_w` is `A = [0, w - 1]` and `H` is `A = {0}`.
* `GA_sound`, `GA_sound_any`: the analogue of `G_sound`. If `t ∈ G_A(B)`,
  then against every deletion within `B` the survivors have a subset sum `s`
  with `s + a = t` for some `a ∈ A`.
* `GA_antitone`, `G_antitone`, `H_antitone`: tables shrink as the budget
  grows (the basis of the threshold form, `Bobi/Threshold.lean`).
* `GA_sound_perm`: the classes may be played in any order; a table built
  for a permutation `play` of the undecided classes is sound for deletions
  given in the search order.
* `node_rule`, `node_table_sound`: the per-node pruning rule.
-/

@[expose] public section

namespace Bobi

/-- The robust table `G_A(k, B)` with end set `A`, for the classes `k..` given
(in play order) as the list `cs`: the recursion of `H` and `G_w`,

`t ∈ G_A(k, B)` iff for every `r` copies deleted from class `k` (cost
`c_k(r) ≤ B`) some `j ≤ n_k - r` has `t - j v_k ∈ G_A(k + 1, B - c_k(r))`,

with `G_A(m, B) = A` once no class is left. -/
def GA (A : ℕ → Prop) : List Class → ℕ → ℕ → Prop
  | [], _, t => A t
  | c :: cs, B, t => ∀ r ≤ c.copies, c.delCost r ≤ B →
      ∃ j ≤ c.copies - r, j * c.value ≤ t ∧ GA A cs (B - c.delCost r) (t - j * c.value)

/-! ## Soundness -/

/-- **Tables with an end set are sound.** If `t ∈ G_A(cs, B)`, then for every
deletion within budget `B` (`rs[k] ≤ n_k` copies of each class), the
remaining copies have a subset sum `s` with `s + a = t` for some `a ∈ A`.
The proof is the induction of `G_sound`. -/
theorem GA_sound (A : ℕ → Prop) :
    ∀ (cs : List Class) (rs : List ℕ), List.Forall₂ (fun c r => r ≤ c.copies) cs rs →
      ∀ B t, GA A cs B t → totalCost cs rs ≤ B →
        ∃ s a, SubsetSum (remaining cs rs) s ∧ A a ∧ s + a = t
  | [], [], .nil, _, t, h, _ => ⟨0, t, SubsetSum.zero _, by simpa [GA] using h, Nat.zero_add t⟩
  | c :: cs, r :: rs, .cons hr hrs, B, t, h, hB => by
    simp only [totalCost] at hB
    -- The table names the follower's answer `j` to this deletion.
    obtain ⟨j, hj, hjt, hG⟩ := h r hr (by omega)
    obtain ⟨s, a, hs, ha, hsa⟩ := GA_sound A cs rs hrs _ _ hG (by omega)
    exact ⟨j * c.value + s, a, hs.replicate_append hj, ha, by omega⟩

/-- The same for deletions of arbitrary copies, as in `H_sound_any`. -/
theorem GA_sound_any (A : ℕ → Prop) (cs : List Class)
    (hsorted : ∀ c ∈ cs, c.costs.Pairwise (· ≤ ·))
    (ds : List (List ℕ)) (hds : List.Forall₂ (fun c d => d.Sublist c.costs) cs ds)
    (B t : ℕ) (h : GA A cs B t) (hB : (ds.map List.sum).sum ≤ B) :
    ∃ s a, SubsetSum (remaining cs (ds.map List.length)) s ∧ A a ∧ s + a = t := by
  obtain ⟨h1, h2⟩ := counts_of_deletion hsorted hds
  exact GA_sound A cs _ h1 B t h (Nat.le_trans h2 hB)

/-! ## Monotonicity -/

/-- **Antitone in the budget.** With less budget the leader has fewer
deletions, so the table can only grow: `B' ≤ B` gives
`G_A(cs, B) ⊆ G_A(cs, B')`. -/
theorem GA_antitone (A : ℕ → Prop) :
    ∀ (cs : List Class) {B B' : ℕ} (t : ℕ), B' ≤ B → GA A cs B t → GA A cs B' t
  | [], _, _, _, _, h => by simpa [GA] using h
  | c :: cs, B, B', t, hBB, h => by
    simp only [GA] at h ⊢
    intro r hr hc
    obtain ⟨j, hj, hjt, hG⟩ := h r hr (Nat.le_trans hc hBB)
    exact ⟨j, hj, hjt, GA_antitone A cs _ (Nat.sub_le_sub_right hBB _) hG⟩

/-- A larger end set gives a larger table. -/
theorem GA_mono {A A' : ℕ → Prop} (hA : ∀ a, A a → A' a) :
    ∀ (cs : List Class) (B t : ℕ), GA A cs B t → GA A' cs B t
  | [], _, _, h => by simp only [GA] at h ⊢; exact hA _ h
  | c :: cs, B, t, h => by
    simp only [GA] at h ⊢
    intro r hr hc
    obtain ⟨j, hj, hjt, hG⟩ := h r hr hc
    exact ⟨j, hj, hjt, GA_mono hA cs _ _ hG⟩

/-! ## `G_w` and `H` are special cases -/

/-- The band table `G_w` is the table with end set `[0, w - 1]`. -/
theorem G_iff_GA (w : ℕ) : ∀ (cs : List Class) (B t : ℕ), G w cs B t ↔ GA (· < w) cs B t
  | [], _, _ => by simp [G, GA]
  | c :: cs, B, t => by simp only [G, GA, G_iff_GA w cs]

/-- The robust table `H` is the table with end set `{0}`. -/
theorem H_iff_GA : ∀ (cs : List Class) (B t : ℕ), H cs B t ↔ GA (· = 0) cs B t
  | [], _, _ => by simp [H, GA]
  | c :: cs, B, t => by simp only [H, GA, H_iff_GA cs]

/-- `G_w` is antitone in the budget. -/
theorem G_antitone (w : ℕ) (cs : List Class) {B B' : ℕ} (t : ℕ) (hBB : B' ≤ B)
    (h : G w cs B t) : G w cs B' t :=
  (G_iff_GA w cs B' t).2 (GA_antitone _ cs t hBB ((G_iff_GA w cs B t).1 h))

/-- `H` is antitone in the budget. -/
theorem H_antitone (cs : List Class) {B B' : ℕ} (t : ℕ) (hBB : B' ≤ B)
    (h : H cs B t) : H cs B' t :=
  (H_iff_GA cs B' t).2 (GA_antitone _ cs t hBB ((H_iff_GA cs B t).1 h))

/-- `G_sound` again, from `GA_sound` with `A = [0, w - 1]`. -/
theorem G_sound_of_GA (w : ℕ) (cs : List Class) (rs : List ℕ)
    (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs) (B t : ℕ) (h : G w cs B t)
    (hB : totalCost cs rs ≤ B) : ∃ s, SubsetSum (remaining cs rs) s ∧ s ≤ t ∧ t < s + w := by
  obtain ⟨s, a, hs, ha, hsa⟩ := GA_sound _ cs rs hrs B t ((G_iff_GA w cs B t).1 h) hB
  exact ⟨s, hs, by omega, by omega⟩

/-- `H_sound` again, from `GA_sound` with `A = {0}`. -/
theorem H_sound_of_GA (cs : List Class) (rs : List ℕ)
    (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs) (B t : ℕ) (h : H cs B t)
    (hB : totalCost cs rs ≤ B) : SubsetSum (remaining cs rs) t := by
  obtain ⟨s, a, hs, ha, hsa⟩ := GA_sound _ cs rs hrs B t ((H_iff_GA cs B t).1 h) hB
  rwa [show t = s by omega]

/-! ## Any play order -/

/-- Deletion counts follow the classes through a reordering: for a
permutation `cs'` of `cs` and counts `rs` for `cs`, there are counts `rs'`
for `cs'` that pair every class with the same count. -/
theorem counts_perm {cs cs' : List Class} (hp : cs'.Perm cs) :
    ∀ {rs : List ℕ}, List.Forall₂ (fun c r => r ≤ c.copies) cs rs →
      ∃ rs', List.Forall₂ (fun c r => r ≤ c.copies) cs' rs' ∧ (cs'.zip rs').Perm (cs.zip rs) := by
  induction hp with
  | nil =>
    intro rs h
    cases h
    exact ⟨[], .nil, .nil⟩
  | cons x _ ih =>
    intro rs h
    cases h with
    | cons hr hrs =>
      obtain ⟨rs', h', hp'⟩ := ih hrs
      exact ⟨_, .cons hr h', hp'.cons _⟩
  | swap x y l =>
    intro rs h
    cases h with
    | cons hx h =>
      cases h with
      | cons hy hl => exact ⟨_, .cons hy (.cons hx hl), .swap _ _ _⟩
  | trans _ _ ih₁ ih₂ =>
    intro rs h
    obtain ⟨rs₂, h₂, hp₂⟩ := ih₂ h
    obtain ⟨rs₁, h₁, hp₁⟩ := ih₁ h₂
    exact ⟨rs₁, h₁, hp₁.trans hp₂⟩

/-- `remaining` class by class. -/
theorem remaining_eq_flatten : ∀ (cs : List Class) (rs : List ℕ),
    remaining cs rs =
      ((cs.zip rs).map fun p => List.replicate (p.1.copies - p.2) p.1.value).flatten
  | [], _ => by simp [remaining]
  | _ :: _, [] => by simp [remaining]
  | c :: cs, r :: rs => by simp [remaining, remaining_eq_flatten cs rs]

/-- `totalCost` class by class. -/
theorem totalCost_eq_sum : ∀ (cs : List Class) (rs : List ℕ),
    totalCost cs rs = ((cs.zip rs).map fun p => p.1.delCost p.2).sum
  | [], _ => by simp [totalCost]
  | _ :: _, [] => by simp [totalCost]
  | c :: cs, r :: rs => by simp [totalCost, totalCost_eq_sum cs rs]

/-- Reordering the (class, count) pairs reorders the survivors. -/
theorem remaining_perm {cs cs' : List Class} {rs rs' : List ℕ}
    (h : (cs'.zip rs').Perm (cs.zip rs)) : (remaining cs' rs').Perm (remaining cs rs) := by
  rw [remaining_eq_flatten, remaining_eq_flatten]
  exact (h.map _).flatten

/-- Reordering the (class, count) pairs keeps the cost. -/
theorem totalCost_perm {cs cs' : List Class} {rs rs' : List ℕ}
    (h : (cs'.zip rs').Perm (cs.zip rs)) : totalCost cs' rs' = totalCost cs rs := by
  rw [totalCost_eq_sum, totalCost_eq_sum]
  exact (h.map _).sum_eq

/-- **Any play order.** A table built for the undecided classes played in the
order `play`, any permutation of the search-order list `cs`, is sound for
deletions given in the search order. -/
theorem GA_sound_perm (A : ℕ → Prop) {cs play : List Class} (hplay : play.Perm cs)
    (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs)
    {B t : ℕ} (h : GA A play B t) (hB : totalCost cs rs ≤ B) :
    ∃ s a, SubsetSum (remaining cs rs) s ∧ A a ∧ s + a = t := by
  obtain ⟨rs', h', hp⟩ := counts_perm hplay hrs
  obtain ⟨s, a, hs, ha, hsa⟩ :=
    GA_sound A play rs' h' B t h (by rw [totalCost_perm hp]; exact hB)
  exact ⟨s, a, hs.perm (remaining_perm hp), ha, hsa⟩

/-- `GA_sound_perm` for deletions of arbitrary copies. -/
theorem GA_sound_perm_any (A : ℕ → Prop) {cs play : List Class} (hplay : play.Perm cs)
    (hsorted : ∀ c ∈ cs, c.costs.Pairwise (· ≤ ·))
    (ds : List (List ℕ)) (hds : List.Forall₂ (fun c d => d.Sublist c.costs) cs ds)
    {B t : ℕ} (h : GA A play B t) (hB : (ds.map List.sum).sum ≤ B) :
    ∃ s a, SubsetSum (remaining cs (ds.map List.length)) s ∧ A a ∧ s + a = t := by
  obtain ⟨h1, h2⟩ := counts_of_deletion hsorted hds
  exact GA_sound_perm A hplay _ h1 h (Nat.le_trans h2 hB)

/-! ## The per-node rule -/

/-- The end set of `Search::node_table_threshold`: the residuals `a ≤ hi`
(the table's array is indexed `0..=hi`) with some decided sum `s ∈ S` in
`[a - (hi - lo), a]`. The code sets `a[t] = cap` exactly when the largest
`s ≤ t` with `sums.get(s)` has `s + (hi - lo) ≥ t`. -/
def nodeEnd (S : ℕ → Prop) (lo hi a : ℕ) : Prop :=
  a ≤ hi ∧ ∃ s, S s ∧ s ≤ a ∧ a ≤ s + (hi - lo)

instance (S : ℕ → Prop) [DecidablePred S] (lo hi : ℕ) : DecidablePred (nodeEnd S lo hi) :=
  fun a => decidable_of_iff (a ≤ hi ∧ ∃ s, s < a + 1 ∧ S s ∧ a ≤ s + (hi - lo)) (by
    unfold nodeEnd
    constructor
    · rintro ⟨h1, s, h2, h3, h4⟩
      exact ⟨h1, s, h3, by omega, h4⟩
    · rintro ⟨h1, s, h2, h3, h4⟩
      exact ⟨h1, s, by omega, h2, h4⟩)

/-- **The node rule, for any end set inside the node's.** Let `S` be sums the
decided survivors reach, `lo ≤ hi`, and `A` any set of residuals `a` with
some `s ∈ S` in `[a - (hi - lo), a]`. If `hi ∈ G_A(cs, B)` for the undecided
classes, then every deletion within `B` leaves a subset of the decided values
plus the survivors with sum in `[lo, hi]`: the undecided survivors give `x`
with `x + a = hi`, and `s + x` lies in `[lo, hi]`. -/
theorem node_rule {decided : List ℕ} {S : ℕ → Prop} (hS : ∀ s, S s → SubsetSum decided s)
    {lo hi : ℕ} (hlh : lo ≤ hi) {A : ℕ → Prop}
    (hA : ∀ a, A a → ∃ s, S s ∧ s ≤ a ∧ a ≤ s + (hi - lo))
    (cs : List Class) (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs)
    {B : ℕ} (h : GA A cs B hi) (hB : totalCost cs rs ≤ B) :
    ∃ u, SubsetSum (decided ++ remaining cs rs) u ∧ lo ≤ u ∧ u ≤ hi := by
  obtain ⟨x, a, hx, ha, hxa⟩ := GA_sound A cs rs hrs B hi h hB
  obtain ⟨s, hs, hsa, has⟩ := hA a ha
  exact ⟨s + x, (hS s hs).append hx, by omega, by omega⟩

/-- **The node rule of `Search::node_table_threshold`.** With `S` the decided
classes' reachable sums (or any of them, such as the bitset truncated at
`hi`) and the code's end set `nodeEnd S lo hi`: if `hi` is in the table of
the undecided classes at budget `B`, every deletion of the undecided classes
within `B` leaves a subset of `decided ++ survivors` with sum in `[lo, hi]`. -/
theorem node_table_sound {decided : List ℕ} {S : ℕ → Prop}
    (hS : ∀ s, S s → SubsetSum decided s) {lo hi : ℕ} (hlh : lo ≤ hi)
    (cs : List Class) (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs)
    {B : ℕ} (h : GA (nodeEnd S lo hi) cs B hi) (hB : totalCost cs rs ≤ B) :
    ∃ u, SubsetSum (decided ++ remaining cs rs) u ∧ lo ≤ u ∧ u ≤ hi :=
  node_rule hS hlh (fun _ ha => ha.2) cs rs hrs h hB

/-- The node rule for a table in any play order and a deletion of arbitrary
copies: `play` is a permutation of the undecided classes `cs` (search order),
`ds[k]` the costs of the copies deleted from `cs[k]`. -/
theorem node_table_sound_any {decided : List ℕ} {S : ℕ → Prop}
    (hS : ∀ s, S s → SubsetSum decided s) {lo hi : ℕ} (hlh : lo ≤ hi)
    {cs play : List Class} (hplay : play.Perm cs) (hsorted : ∀ c ∈ cs, c.costs.Pairwise (· ≤ ·))
    (ds : List (List ℕ)) (hds : List.Forall₂ (fun c d => d.Sublist c.costs) cs ds)
    {B : ℕ} (h : GA (nodeEnd S lo hi) play B hi) (hB : (ds.map List.sum).sum ≤ B) :
    ∃ u, SubsetSum (decided ++ remaining cs (ds.map List.length)) u ∧ lo ≤ u ∧ u ≤ hi := by
  obtain ⟨x, a, hx, ⟨_, s, hs, hsa, has⟩, hxa⟩ := GA_sound_perm_any _ hplay hsorted ds hds h hB
  exact ⟨s + x, (hS s hs).append hx, by omega, by omega⟩

/-- The node rule contains the rule of `H` (`RobustTable::covers`, in the
same play order): if a decided sum `s` and a target `target ∈ [lo, hi]` have
`target - s ∈ H(cs, B)`, then `hi ∈ G_A(cs, B)` for the node's end set.
(Shift the follower's strategy up by `hi - target + s`.) -/
theorem node_table_of_H {S : ℕ → Prop} {lo hi s target : ℕ} (hs : S s) (hst : s ≤ target)
    (hlo : lo ≤ target) (hhi : target ≤ hi) :
    ∀ (cs : List Class) (B : ℕ), H cs B (target - s) → GA (nodeEnd S lo hi) cs B hi := by
  -- The statement for every residual: `t ∈ H` gives `t + d ∈ G_A` for the
  -- shift `d = hi - target + s`, which ends in `A`.
  suffices key : ∀ (cs : List Class) (B t : ℕ), H cs B t → t ≤ target - s →
      GA (nodeEnd S lo hi) cs B (t + (hi - (target - s))) by
    intro cs B h
    have := key cs B _ h (Nat.le_refl _)
    rwa [show target - s + (hi - (target - s)) = hi by omega] at this
  intro cs
  induction cs with
  | nil =>
    intro B t h ht
    simp only [H] at h
    subst h
    exact ⟨by omega, s, hs, by omega, by omega⟩
  | cons c cs ih =>
    intro B t h ht
    simp only [H, GA] at h ⊢
    intro r hr hc
    obtain ⟨j, hj, hjt, hH⟩ := h r hr hc
    refine ⟨j, hj, by omega, ?_⟩
    have := ih _ _ hH (by omega)
    rwa [show t - j * c.value + (hi - (target - s)) = t + (hi - (target - s)) - j * c.value by
      omega] at this

end Bobi
