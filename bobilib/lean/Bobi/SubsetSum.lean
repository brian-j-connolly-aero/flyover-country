module

public import Mathlib.Algebra.BigOperators.Group.List.Basic

/-!
# Contiguous windows of subset sums

The subset-sum search in `robust_ss.rs` prunes a node when the sums the
follower can already reach contain a contiguous window `[a, b]` that the
remaining values can only extend (`Search::window_safe`). The facts behind
it:

* `window_extend` (theorem 3): adding a value `v ≤ b - a + 1` to a multiset
  whose subset sums cover `[a, b]` makes them cover `[a, b + v]`.
* `window_extend_all`: adding values one after another, each at most the
  width of the window so far, keeps the window contiguous; it ends at
  `b + (sum of the values)`.
* `window_survivors`: the form `window_safe` checks. The leader deletes some
  of the values; if at every value the mass deleted before it plus the value
  is at most the width plus the mass before it, the survivors still extend
  the window by their total.
-/

@[expose] public section

namespace Bobi

/-- `t` is a subset sum of the multiset `M` (a list whose order does not
matter): some sub-multiset of `M` sums to `t`. -/
def SubsetSum (M : List ℕ) (t : ℕ) : Prop :=
  ∃ s : List ℕ, s.Sublist M ∧ s.sum = t

namespace SubsetSum

theorem zero (M : List ℕ) : SubsetSum M 0 :=
  ⟨[], List.nil_sublist M, rfl⟩

theorem cons_skip {M : List ℕ} {t : ℕ} (v : ℕ) (h : SubsetSum M t) : SubsetSum (v :: M) t :=
  let ⟨s, hs, hsum⟩ := h
  ⟨s, hs.cons v, hsum⟩

theorem cons_take {M : List ℕ} {t : ℕ} (v : ℕ) (h : SubsetSum M t) :
    SubsetSum (v :: M) (v + t) :=
  let ⟨s, hs, hsum⟩ := h
  ⟨v :: s, hs.cons_cons v, by simp [hsum]⟩

/-- Subset sums depend on the multiset only, not on the order of the list. -/
theorem perm {M M' : List ℕ} (h : M.Perm M') {t : ℕ} (ht : SubsetSum M t) : SubsetSum M' t := by
  obtain ⟨s, hs, hsum⟩ := ht
  obtain ⟨s', hperm, hs'⟩ := h.subperm_left.1 hs.subperm
  exact ⟨s', hs', hperm.sum_eq ▸ hsum⟩

/-- Sums of two parts add up. -/
theorem append {L₁ L₂ : List ℕ} {s t : ℕ} (h₁ : SubsetSum L₁ s) (h₂ : SubsetSum L₂ t) :
    SubsetSum (L₁ ++ L₂) (s + t) :=
  let ⟨a, ha, hsa⟩ := h₁
  let ⟨b, hb, hsb⟩ := h₂
  ⟨a ++ b, ha.append hb, by simp [hsa, hsb]⟩

/-- `j` of `m` copies of `v` contribute `j * v`. -/
theorem replicate_append {L : List ℕ} {s j m v : ℕ} (hj : j ≤ m) (h : SubsetSum L s) :
    SubsetSum (List.replicate m v ++ L) (j * v + s) :=
  append ⟨List.replicate j v, (List.replicate_sublist_replicate v).2 hj, by simp⟩ h

end SubsetSum

/-- Every integer in `[a, b]` is a subset sum of `M`. -/
def Window (M : List ℕ) (a b : ℕ) : Prop :=
  ∀ t, a ≤ t → t ≤ b → SubsetSum M t

theorem Window.perm {M M' : List ℕ} (h : M.Perm M') {a b : ℕ} (hw : Window M a b) :
    Window M' a b :=
  fun t hat htb => (hw t hat htb).perm h

/-- **Theorem 3 (window extension).** If the subset sums of `M` cover
`[a, b]` and `v ≤ b - a + 1`, those of `v :: M` cover `[a, b + v]`: sums up to
`b` need not use `v`, and a sum `t` above `b` is `v` plus `t - v ∈ [a, b]`. -/
theorem window_extend {M : List ℕ} {a b v : ℕ} (hab : a ≤ b) (hv : v ≤ b - a + 1)
    (h : Window M a b) : Window (v :: M) a (b + v) := by
  intro t hat htb
  by_cases htb' : t ≤ b
  · exact (h t hat htb').cons_skip v
  · have := (h (t - v) (by omega) (by omega)).cons_take v
    rwa [show v + (t - v) = t by omega] at this

/-- `Fits w vs`: adding the values `vs` in order to a window of width `w`,
each value is at most the width of the window so far (which grows by every
value added). -/
def Fits : ℕ → List ℕ → Prop
  | _, [] => True
  | w, v :: vs => v ≤ w ∧ Fits (w + v) vs

/-- **Theorem 3, iterated.** Adding values that each fit the current width
keeps the window contiguous, all the way up to `b + vs.sum`. -/
theorem window_extend_all :
    ∀ {vs M : List ℕ} {a b : ℕ}, a ≤ b → Fits (b - a + 1) vs → Window M a b →
      Window (vs ++ M) a (b + vs.sum)
  | [], _, _, _, _, _, h => by simpa using h
  | v :: vs, M, a, b, hab, ⟨hv, hfits⟩, h => by
    have h1 := window_extend hab hv h
    have h2 := window_extend_all (vs := vs) (b := b + v) (by omega)
      (by rwa [show b + v - a + 1 = b - a + 1 + v by omega]) h1
    rw [List.sum_cons, ← Nat.add_assoc]
    exact h2.perm List.perm_middle

/-! ## The stall test of `window_safe` -/

/-- The values that survive: `keep = false` marks a copy the leader deleted. -/
def survivors (l : List (ℕ × Bool)) : List ℕ :=
  (l.filter (·.2)).map (·.1)

/-- Total value of the copies in `l`. -/
def mass (l : List (ℕ × Bool)) : ℕ :=
  (l.map (·.1)).sum

/-- Total value of the copies in `l` the leader deleted. -/
def deleted (l : List (ℕ × Bool)) : ℕ :=
  ((l.filter (!·.2)).map (·.1)).sum

/-- No stall: if at every copy, the mass the leader deleted before it plus its
value is at most the starting width `w` plus all the mass before it, the
surviving copies fit (the running width is `w` plus the surviving mass). -/
theorem fits_survivors :
    ∀ (l : List (ℕ × Bool)) (w : ℕ),
      (∀ k (hk : k < l.length), deleted (l.take k) + l[k].1 ≤ w + mass (l.take k)) →
        Fits w (survivors l)
  | [], _, _ => trivial
  | (v, keep) :: rest, w, h => by
    have h0 := h 0 (by simp)
    simp only [List.take_zero, deleted, mass, List.filter_nil, List.map_nil, List.sum_nil,
      List.getElem_cons_zero, zero_add, add_zero] at h0
    -- The condition for the rest, at each later copy.
    have hrest : ∀ k (hk : k < rest.length),
        deleted (rest.take k) + (if keep then 0 else v) + rest[k].1 ≤
          w + v + mass (rest.take k) := by
      intro k hk
      have := h (k + 1) (by simp; omega)
      cases keep <;> simp_all [deleted, mass, List.take_succ_cons] <;> omega
    cases keep with
    | true =>
      refine ⟨h0, fits_survivors rest (w + v) fun k hk => ?_⟩
      simpa using hrest k hk
    | false =>
      refine fits_survivors rest w fun k hk => ?_
      have := hrest k hk
      simp only [Bool.false_eq_true, ite_false] at this
      omega

/-- What `window_safe` concludes: with no stall, the window of decided sums
`[a, b]` grows to `[a, b + surviving mass]`. -/
theorem window_survivors {M : List ℕ} {a b : ℕ} (hab : a ≤ b) (h : Window M a b)
    (l : List (ℕ × Bool))
    (hl : ∀ k (hk : k < l.length),
      deleted (l.take k) + l[k].1 ≤ (b - a + 1) + mass (l.take k)) :
    Window (survivors l ++ M) a (b + (survivors l).sum) :=
  window_extend_all hab (fits_survivors l _ hl) h

end Bobi
