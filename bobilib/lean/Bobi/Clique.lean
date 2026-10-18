module

public import Mathlib.Combinatorics.SimpleGraph.Extremal.Turan
public import Mathlib.Combinatorics.SimpleGraph.Coloring.Vertex
public import Mathlib.Combinatorics.SimpleGraph.DeleteEdges

/-!
# Bounds for edge clique interdiction

In BOBILib's `clique` instances the leader deletes at most `k` of the `m`
edges of a graph on `n` vertices, and the follower takes a largest clique of
what is left. Two certificates bound the value:

* `triangle_survives` (lower bound 3): if `m - k > ⌊n² / 4⌋`, Mantel's
  theorem (`mantel`, Turán's theorem for triangles) leaves a triangle
  whatever the leader deletes. This covers the density-0.7 instances
  (`clique40`, `clique50`, `clique60`).
* `cliqueFree_of_partition` (upper bound `r`): a vertex colouring with `r`
  colours whose inside edges the leader can afford to delete leaves an
  `r`-partite graph, with no clique on `r + 1` vertices.

`G.CliqueFree 3` says `G` has no triangle.
-/

@[expose] public section

namespace Bobi

open Finset SimpleGraph

variable {V : Type*}

/-- **Mantel's theorem.** A graph on `n` vertices with more than `⌊n² / 4⌋`
edges has a triangle. This is Mathlib's Turán bound for `r = 2`. -/
theorem mantel [Fintype V] (G : SimpleGraph V) [DecidableRel G.Adj]
    (h : Fintype.card V ^ 2 / 4 < #G.edgeFinset) : ¬ G.CliqueFree 3 := fun hG =>
  absurd (turanNumber_two (n := Fintype.card V) ▸ hG.card_edgeFinset_le (r := 2)) (not_le.2 h)

/-- Mantel's theorem without division: `4 · #edges > n²`. -/
theorem mantel' [Fintype V] (G : SimpleGraph V) [DecidableRel G.Adj]
    (h : Fintype.card V ^ 2 < 4 * #G.edgeFinset) : ¬ G.CliqueFree 3 :=
  mantel G (by omega)

/-- **Deleting `k` edges leaves a triangle.** If `G` has `m` edges on `n`
vertices, `D` is any set of at most `k` edges (of `G` or not) and
`m - k > ⌊n² / 4⌋`, then `G` minus `D` still has a triangle: at least
`m - k` edges remain. -/
theorem triangle_survives [Fintype V] [DecidableEq V] {n m k : ℕ}
    (G : SimpleGraph V) [DecidableRel G.Adj] (hn : Fintype.card V = n) (hm : #G.edgeFinset = m)
    (D : Finset (Sym2 V)) (hD : #D ≤ k) (h : n ^ 2 / 4 < m - k) :
    ¬ (G.deleteEdges D).CliqueFree 3 := by
  classical
  subst hn hm
  apply mantel
  rw [edgeFinset_deleteEdges]
  have := le_card_sdiff D G.edgeFinset
  omega

/-- **Partition upper bound.** Colour the vertices with `r` colours. If `D`
contains every edge of `G` inside a colour class, then `G` minus `D` is
`r`-partite (the colouring is proper), so it has no clique on `r + 1`
vertices. -/
theorem cliqueFree_of_partition (G : SimpleGraph V) {r : ℕ} (f : V → Fin r)
    (D : Set (Sym2 V)) (hD : ∀ v w, G.Adj v w → f v = f w → s(v, w) ∈ D) :
    (G.deleteEdges D).CliqueFree (r + 1) := by
  refine Colorable.cliqueFree ⟨Coloring.mk f fun {v w} hvw hf => ?_⟩ (Nat.lt_succ_self r)
  rw [deleteEdges_adj] at hvw
  exact hvw.2 (hD v w hvw.1 hf)

/-! ## The density-0.7 instances

`(n, m, k)` for the 40-, 50- and 60-vertex instances, `k` a quarter of the
edges: `m - k` beats `⌊n² / 4⌋` (410 > 400, 643 > 625, 930 > 900). -/

theorem mantel_40 : 40 ^ 2 / 4 < 546 - 136 := by decide
theorem mantel_50 : 50 ^ 2 / 4 < 857 - 214 := by decide
theorem mantel_60 : 60 ^ 2 / 4 < 1239 - 309 := by decide

/-- 40 vertices, 546 edges, 136 deletions: a triangle survives. -/
theorem clique40 [Fintype V] [DecidableEq V] (G : SimpleGraph V) [DecidableRel G.Adj]
    (hn : Fintype.card V = 40) (hm : #G.edgeFinset = 546) (D : Finset (Sym2 V))
    (hD : #D ≤ 136) : ¬ (G.deleteEdges D).CliqueFree 3 :=
  triangle_survives G hn hm D hD mantel_40

/-- 50 vertices, 857 edges, 214 deletions: a triangle survives. -/
theorem clique50 [Fintype V] [DecidableEq V] (G : SimpleGraph V) [DecidableRel G.Adj]
    (hn : Fintype.card V = 50) (hm : #G.edgeFinset = 857) (D : Finset (Sym2 V))
    (hD : #D ≤ 214) : ¬ (G.deleteEdges D).CliqueFree 3 :=
  triangle_survives G hn hm D hD mantel_50

/-- 60 vertices, 1239 edges, 309 deletions: a triangle survives. -/
theorem clique60 [Fintype V] [DecidableEq V] (G : SimpleGraph V) [DecidableRel G.Adj]
    (hn : Fintype.card V = 60) (hm : #G.edgeFinset = 1239) (D : Finset (Sym2 V))
    (hD : #D ≤ 309) : ¬ (G.deleteEdges D).CliqueFree 3 :=
  triangle_survives G hn hm D hD mantel_60

end Bobi
