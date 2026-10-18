module

public import Bobi.RobustTable

/-!
# Band tables

`RobustTable::build_band` generalises the robust table `H` from a single
target to a band of `w` targets: `t ∈ G_w(k, B)` says the follower can make
the classes `k..` contribute a sum in `[t - w + 1, t]` whatever the leader
deletes. The recursion is that of `H`; only the base case changes, to
`G_w(m, B) = [0, w - 1]`.

* `G_sound`: the band version of `H_sound`.
* `band_covers_sound`: the pruning rule `RobustTable::covers_run`. A run `[a, b]`
  of decided sums absorbs whatever the undecided classes contribute inside
  the band, so the target is hit exactly.
* `G_one`, `G_sound_one`, `covers_sound_of_band`: with `w = 1` the band
  table is `H` (`RobustTable::build` is `build_band(.., 1)`), and the band
  theorems give back `H_sound` and `covers_sound`.
-/

@[expose] public section

namespace Bobi

/-- The band table `G_w(k, B)` of `RobustTable::build_band`, for the classes
`k..` given as the list `cs`: the recursion of `H`, with base case
`G_w(m, B) = [0, w - 1]`. -/
def G (w : ℕ) : List Class → ℕ → ℕ → Prop
  | [], _, t => t < w
  | c :: cs, B, t => ∀ r ≤ c.copies, c.delCost r ≤ B →
      ∃ j ≤ c.copies - r, j * c.value ≤ t ∧ G w cs (B - c.delCost r) (t - j * c.value)

/-- **Band tables are sound.** If `t ∈ G_w(cs, B)`, then for every deletion
within budget `B` (`rs[k] ≤ n_k` copies of each class), the remaining copies
have a subset sum `s` in the band: `s ≤ t < s + w`. -/
theorem G_sound (w : ℕ) :
    ∀ (cs : List Class) (rs : List ℕ), List.Forall₂ (fun c r => r ≤ c.copies) cs rs →
      ∀ B t, G w cs B t → totalCost cs rs ≤ B →
        ∃ s, SubsetSum (remaining cs rs) s ∧ s ≤ t ∧ t < s + w
  | [], [], .nil, _, _, h, _ => ⟨0, SubsetSum.zero _, Nat.zero_le _, by simpa [G] using h⟩
  | c :: cs, r :: rs, .cons hr hrs, B, t, h, hB => by
    simp only [totalCost] at hB
    -- The table names the follower's answer `j` to this deletion.
    obtain ⟨j, hj, hjt, hG⟩ := h r hr (by omega)
    obtain ⟨s, hs, hst, hts⟩ := G_sound w cs rs hrs _ _ hG (by omega)
    exact ⟨j * c.value + s, hs.replicate_append hj, by omega, by omega⟩

/-- The same for deletions of arbitrary copies, as in `H_sound_any`. -/
theorem G_sound_any (w : ℕ) (cs : List Class) (hsorted : ∀ c ∈ cs, c.costs.Pairwise (· ≤ ·))
    (ds : List (List ℕ)) (hds : List.Forall₂ (fun c d => d.Sublist c.costs) cs ds)
    (B t : ℕ) (h : G w cs B t) (hB : (ds.map List.sum).sum ≤ B) :
    ∃ s, SubsetSum (remaining cs (ds.map List.length)) s ∧ s ≤ t ∧ t < s + w := by
  obtain ⟨h1, h2⟩ := counts_of_deletion hsorted hds
  exact G_sound w cs _ h1 B t h (Nat.le_trans h2 hB)

/-- **The pruning rule `RobustTable::covers_run`.** Say every sum in `[a, b]` is
reachable from the decided survivors, and `t ∈ G_w` for the undecided classes
with `[target - t, target - t + w - 1]` inside `[a, b]` (written without
subtraction: `a + t ≤ target` and `target + w ≤ b + t + 1`; these imply the
code's test `w ≤ b - a + 1`). Then the follower hits `target` against every
deletion within budget: the undecided classes give some `s ∈ [t - w + 1, t]`,
and `target - s` lies in `[a, b]`. -/
theorem band_covers_sound {decided : List ℕ} {a b : ℕ} (hdec : Window decided a b) {w : ℕ}
    (cs : List Class) (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs)
    {B t target : ℕ} (h : G w cs B t) (hB : totalCost cs rs ≤ B)
    (hlo : a + t ≤ target) (hhi : target + w ≤ b + t + 1) :
    SubsetSum (decided ++ remaining cs rs) target := by
  obtain ⟨s, hs, hst, hts⟩ := G_sound w cs rs hrs B t h hB
  have := (hdec (target - s) (by omega) (by omega)).append hs
  rwa [Nat.sub_add_cancel (by omega)] at this

/-! ## Width one -/

/-- The band table of width 1 is the robust table `H`. -/
theorem G_one : ∀ (cs : List Class) (B t : ℕ), G 1 cs B t ↔ H cs B t
  | [], _, _ => by simp [G, H]
  | c :: cs, B, t => by simp only [G, H, G_one cs]

/-- With `w = 1`, `G_sound` is `H_sound`: a sum `s ≤ t < s + 1` is `t`. -/
theorem G_sound_one (cs : List Class) (rs : List ℕ)
    (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs) (B t : ℕ) (h : H cs B t)
    (hB : totalCost cs rs ≤ B) : SubsetSum (remaining cs rs) t := by
  obtain ⟨s, hs, hst, hts⟩ := G_sound 1 cs rs hrs B t ((G_one cs B t).2 h) hB
  rwa [show t = s by omega]

/-- With `w = 1` and the one-point run `[s, s]`, `band_covers_sound` is
`covers_sound`. -/
theorem covers_sound_of_band {decided : List ℕ} {s : ℕ} (hs : SubsetSum decided s)
    (cs : List Class) (rs : List ℕ) (hrs : List.Forall₂ (fun c r => r ≤ c.copies) cs rs)
    {B t : ℕ} (h : H cs B t) (hB : totalCost cs rs ≤ B) :
    SubsetSum (decided ++ remaining cs rs) (s + t) :=
  band_covers_sound (a := s) (b := s) (fun u h1 h2 => (show u = s by omega) ▸ hs) cs rs hrs
    ((G_one cs B t).2 h) hB (Nat.le_refl _) (Nat.le_refl _)

end Bobi
