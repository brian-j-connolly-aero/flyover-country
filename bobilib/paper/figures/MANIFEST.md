# Figure manifest

Twenty figures, one PDF each, all built from `figures/src/`. Of the first fifteen the paper uses 13:
`fig_family_outcomes` (it repeats Table 1) and `fig_hpr_bound` (it only restates
HPR <= OPT) were dropped in the review round; they are still built (`make figures`
from `bobilib/paper/`; rebuilds are byte-identical). Widths are the page
widths of the PDFs: include them at 100% (`\includegraphics{figures/<file>}`)
so the 7-8 pt text keeps its size. The section names are the ones planned for
the paper; numbers are not fixed yet.

| File | Caption draft | Section | Width (in) | Data source |
| --- | --- | --- | --- | --- |
| `fig_bilevel_game.pdf` | The bilevel game: the leader chooses x and pays c^T x + d^T y(x), where y(x) is the follower's best response, so the leader's cost jumps wherever the follower's answer changes (right: schematic). | Introduction / bilevel problems and BOBILib | 6.5 (h 1.6) | none (schematic) |
| `fig_overview_status.pdf` | Where BOBILib's 1,594 feasible or open instances stand: 648 (41%) are settled (527 already optimal in published work, 81 proven optimal here, 40 with no bilevel feasible point), 230 get a verified point where BOBILib lists none (168 of them the first point anywhere) and 716 are unchanged; the thin bar shows BOBILib's own split (1,175 feasible, 419 open). | Introduction / summary of results | 6.5 (h 1.4) | `results/summary.json` |
| `fig_family_outcomes.pdf` | Outcomes by BOBILib family, as shares of each family's feasible or open instances, with counts in the segments and family totals at the right. | Summary of results | 6.5 (h 2.4) | `results/summary.json` |
| `fig_knapsack_interdiction.pdf` | Knapsack interdiction on a toy instance (box width = weight, number = profit, capacity 11): removing the items with profits 8 and 6 cuts the follower's best packing from 21 to 13, and no other pair of removals does as well. | Knapsack interdiction (problem) | 6.5 (h 1.8) | toy instance, brute-force checked (numbers in the source header) |
| `fig_subset_sum_search.pdf` | The search that proves the 16 new knapsack interdiction optima, on toy numbers: (a) it branches on how many copies of each weight survive, (b) a window of consecutive reachable sums only grows as copies are added, and (c) at each node an online game, in which the leader deletes one weight class at a time, decides whether the follower can always end in the set A; if so the node is pruned. | Knapsack interdiction (method) | 6.5 (h 2.2) | toy numbers; panel (c) checked exhaustively; panel (a) schematic |
| `fig_kip_search.pdf` | Search nodes for the 16 knapsack interdiction instances open in the literature, before and after the per-node tables (left, log scale), and CPU seconds after (right, log scale); all 16 are proven optimal, the hardest (500-6-3, optimum 6,976) in 10,079 nodes and 10.0 s. | Knapsack interdiction (results) | 6.5 (h 2.4) | `results/ss/proofs.csv` (after); `bobilib/REPORT.md` table (before) |
| `fig_lp_bracket.pdf` | The LP bracket for one follower knapsack: the follower's LP value U(x) exceeds its integer value Phi(x) by at most p_max, so the optimum lies between min_x U - p_max and Phi(x^), where x^ (x-hat) minimises U (made-up numbers). | Multidimensional knapsack interdiction (method) | 3.15 (h 1.6) | none (numbers obey the rule) |
| `fig_or_gaps.pdf` | Relative optimality gap on the OR set over the instances with both published bounds, leaving out the 4 whose published lower bound exceeds the published point (n = 90, 90, 90 and 122, 129, 142), literature alone against literature plus this work, by follower type and budget; cumulative share, gaps of 0 at the separate "0" tick. | Multidimensional knapsack interdiction (results) | 6.5 (h 3.0) | `results/or/compare.csv` (`gap`, `lit_gap`) |
| `fig_or_improvement.pdf` | Improvement of the verified OR points over the best published point (literature or BOBILib), by budget and follower type (cumulative share; the 3 points of instances with conflicting published bounds left out); 469 of the 690 points are strictly better, 72 equal, 61 worse, and 88 instances get their first point anywhere. | Multidimensional knapsack interdiction (results) | 6.5 (h 1.95) | `results/or/compare.csv` (`our_point`, `lit_ub`, `bobilib`) |
| `fig_clique_interdiction.pdf` | Clique interdiction on an 8-vertex graph with budget 5: (a) a deletion that still leaves a 4-clique; (b) why the optimum is 3: deleting the 5 edges inside the groups V1-V3 leaves a 3-partite graph (omega <= 3), and any 5 deletions leave 17 > floor(8^2/4) = 16 edges, so a triangle survives (Mantel). | Clique interdiction (method) | 6.5 (h 2.0) | toy graph, checked over all C(22,5) deletions |
| `fig_clique_values.pdf` | The 60 clique interdiction instances: BOBILib's value (5-13, open circle) against the proven optimum (3 at density 0.7, 4 at density 0.9); the three open instances lie in [3, 4]. | Clique interdiction (results) | 6.5 (h 2.2) | `results/clique/summary.csv` |
| `fig_network_attack_protect.pdf` | The attack-protect game on a 15-node tree: the leader attacks 2 nodes, the follower then protects 2, and the attack spreads through unprotected neighbors; against the optimal attack shown, the follower saves at most 8 of the 15 nodes. | Network interdiction | 3.15 (h 1.8) | toy tree, enumerated in full |
| `fig_hpr_bound.pdf` | The bound for the MIPLIB-derived instances: the high point relaxation (HPR) is the original MIPLIB model, so its published optimum is a lower bound, and a verified bilevel feasible point with that value is optimal. | MIPLIB-derived instances (method) | 3.15 (h 1.0) | none (schematic) |
| `fig_unbounded_follower.pdf` | Why some MIPLIB-derived instances have no bilevel feasible point: the follower's region moves with the leader's choice x but keeps the same direction r, and f^T r < 0 leaves the follower without a best response for every x (Meyer 1974). | MIPLIB-derived instances (method) | 3.15 (h 2.0) | none (schematic) |
| `fig_general_outcomes.pdf` | Outcomes for the 266 MIPLIB-derived instances BOBILib lists as feasible or open, by BOBILib status and, for the open ones, by whether leader rows contain follower variables (coupling rows). | MIPLIB-derived instances (results) | 3.15 (h 1.75) | `results/summary.json`, `results/general/points.csv`, `results/general-coupled/summary.csv` |
| `fig_family_map.pdf` | BOBILib at a glance: one tile per family, with its number of instances and the number BOBILib lists as feasible or open (unsolved; the bar shows their share); BOBILib's other 33 non-optimal instances are proven infeasible and belong to the general families. | The problems (section opening) | 6.5 (h 2.0) | `generated/facts.tex` (`\Fam...All`, `\Fam...Unsolved`), read at build time |
| `fig_mkip.pdf` | Multidimensional knapsack interdiction on a toy instance: every item has a profit, a weight and a volume, and the follower's knapsack has a limit for each; the leader blocks two items and the follower packs the most valuable set within both limits (15 instead of 23; blocking the two most valuable items would leave 17). | The problems / multidimensional knapsack interdiction | 6.5 (h 1.9) | toy instance, brute-force checked (numbers in the source header) |
| `fig_generalized.pdf` | Generalized interdiction: leader variables x, follower variables y; for the interdiction pairs x_j = 1 forces y_j = 0; the leader's rows contain x and y (coupling), the follower's only y; the objectives differ. | The problems / generalized interdiction | 3.15 (h 1.75) | none (schematic, structure checked on all 90 files) |
| `fig_assignment.pdf` | Assignment interdiction on a toy instance (4 workers, 4 jobs, costs in the cells): the leader bans two pairs and the follower's cheapest assignment of the rest costs 21 instead of 8. | The problems / assignment interdiction (pairs with `fig_generalized`) | 3.15 (h 1.75) | toy instance, brute-force checked (numbers in the source header) |
| `fig_miplib_split.pdf` | How BOBILib turns a MILP into a bilevel instance: X_0_100 (leader: first half of the variables; every row at the follower), X_50_50 (leader also gets the first half of the rows, which then contain follower variables: coupling rows), MIPLIB 3 X-k (follower: first fraction k of the variables and every row); the leader keeps the MILP's objective, the follower minimizes its negation on its own variables. | The problems / general instances | 6.5 (h 2.0) | none (schematic; the split rules were checked on all 389 MIPLIB-derived instance files) |

Pairing: the paper is single column, so the half-width figures can share a
row as (a)/(b) panels, as `fig_generalized` and `fig_assignment` do in the problems section
(both 1.75 in high), for example `fig_unbounded_follower` with
`fig_general_outcomes` (heights 2.0 and 1.75 in), or `fig_lp_bracket` with
`fig_network_attack_protect` if those sections are short.

## Glyphs and colours

* `fig_overview_status`: every label sits on a plain face or on white. The label of the
  dotted orange segment ("verified point, none in BOBILib") is below the bar, in black on
  white, with a leader line; the narrow aqua segments keep their labels above the bar.

* Explainer figures: leader blue, follower orange, deleted or interdicted
  grey with a cross, certificates and proofs aqua. The network figure uses the
  status red only for fire, and rings each attacked node in the leader's blue.
* Outcome bars: published optimum blue, proven optimal here solid aqua, no
  feasible point aqua with stripes, verified point where BOBILib lists none orange with dots,
  unchanged light grey. Stripes and dots keep aqua and orange apart in
  greyscale.

## Notes for the paper text

* The "before" node counts in `fig_kip_search` come from `bobilib/REPORT.md`.
  `results/ss/summary.csv`, which the data package describes as the previous
  search, holds an older run with other numbers (200-6-2: 17,415,121 nodes
  there, 3,926,223 in the report). Quote the report's numbers, or reconcile
  the two files first.
* REPORT.md says 200-6-3 "did not finish in 2 h" before the per-node tables;
  the spec says 4 h. The figure only says "not finished"; the text needs one
  value.
* `fig_general_outcomes` splits by BOBILib status and coupling rows. The
  results files do not record coupling rows for the 81 BOBILib-feasible
  instances, so they are not split.
