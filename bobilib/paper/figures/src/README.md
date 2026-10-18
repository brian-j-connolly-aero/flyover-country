# Figure sources

Each figure is one file in `figures/` built from one source here.

## Explainer and method figures (TikZ)

Standalone TikZ documents (`\documentclass[tikz,border=2pt]{standalone}`,
Times via `mathptmx`, the paper's palette as `\definecolor`). Each page has a
fixed size, so the figures can be included at 100% and their 8 pt text
matches the paper. The toy numbers in each figure are listed, with the check
that was run on them, in the comment block at the top of its source.

| Source | Figure | Size (in) |
| --- | --- | --- |
| `fig_bilevel_game.tex` | the leader-follower game; the leader's value as a staircase in `x` | 6.5 x 1.6 |
| `fig_knapsack_interdiction.tex` | toy knapsack interdiction: 8 items, the leader removes 2, the follower packs the rest (value 13, or 21 with no removal) | 6.5 x 1.8 |
| `fig_subset_sum_search.tex` | the subset-sum search: (a) branching on survivor counts, (b) windows of reachable sums, (c) the per-node online game and its end set | 6.5 x 2.2 |
| `fig_clique_interdiction.tex` | clique interdiction on 8 vertices: (a) one deletion and the largest clique left, (b) the 3-partition and Mantel certificate | 6.5 x 2.0 |
| `fig_unbounded_follower.tex` | recession-ray certificate: the follower's region moves with `x`, its direction `d` does not | 3.15 x 2.0 |
| `fig_hpr_bound.tex` | HPR optimum <= bilevel optimum <= any bilevel feasible point | 3.15 x 1.0 |
| `fig_lp_bracket.tex` | LP bracket for one follower knapsack: `min U - p_max <= OPT <= Phi(x*)` | 3.15 x 1.6 |
| `fig_network_attack_protect.tex` | attack-protect game on a 15-node tree: 2 attacks, 2 protections, 8 saved | 3.15 x 1.8 |

Build (pdflatex with TikZ; run from this folder):

```sh
tmp=$(mktemp -d)
for f in fig_bilevel_game fig_knapsack_interdiction fig_subset_sum_search \
         fig_clique_interdiction fig_unbounded_follower fig_hpr_bound \
         fig_lp_bracket fig_network_attack_protect; do
    pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$tmp" "$f.tex" \
        > /dev/null && cp "$tmp/$f.pdf" "../$f.pdf"
done
rm -rf "$tmp"
```

To check one visually: `pdftoppm -r 150 -png ../fig_hpr_bound.pdf /tmp/fig_hpr_bound`.

## Data figures (matplotlib)

The `fig_*.py` scripts read only files under `bobilib/results/` and write
`figures/<name>.pdf`; `make_all.sh` runs them all (shared style in `style.py`).
