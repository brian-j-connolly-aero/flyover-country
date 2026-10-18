"""OR set (multidimensional knapsack interdiction): empirical distribution of the
relative gap (best upper - best lower) / best upper, literature alone vs
literature plus this work, over the instances with both a published lower and a
published upper bound (paired).

Data: results/or/compare.csv (columns variant, alpha, lit_gap, gap).
"""

import collections
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))  # style.py (works under python3 -I)
import numpy as np  # noqa: E402
import style as S  # noqa: E402
from matplotlib.lines import Line2D  # noqa: E402
from matplotlib.ticker import FixedLocator, NullFormatter  # noqa: E402

S.setup()
rows = S.read_csv("or/compare.csv")
S.check(len(rows), 810, "OR instances")

pairs = collections.defaultdict(list)       # (follower type, budget) -> [(lit, now)]
by_variant = collections.defaultdict(list)  # (variant, budget) -> [(lit, now)]
for r in rows:
    if r["gap"] and r["lit_gap"]:
        lit, now = float(r["lit_gap"]), float(r["gap"])
        ft = "one" if r["variant"] == "100" else "several"
        pairs[(ft, r["alpha"])].append((lit, now))
        by_variant[(r["variant"], r["alpha"])].append((lit, now))

# Paired medians quoted in the paper (upper median, as scripts/compare_or.py), in %.
quoted = {("100", "0.25"): (24.0, 4.1), ("100", "0.50"): (34.0, 7.1), ("100", "0.75"): (0.14, 0.14),
          ("0", "0.25"): (33.3, 31.4), ("0", "0.50"): (36.6, 29.1), ("0", "0.75"): (5.1, 0.04),
          ("50", "0.25"): (29.6, 28.2), ("50", "0.50"): (41.3, 34.8), ("50", "0.75"): (11.8, 1.4)}
for k, (ql, qn) in quoted.items():
    v = by_variant[k]
    ml = 100 * S.upper_median([a for a, _ in v])
    mn = 100 * S.upper_median([b for _, b in v])
    nd = 2 if ql < 1 else 1
    S.check(round(ml, nd), ql, f"median literature gap, variant -{k[0]}, budget {k[1]}")
    nd = 2 if qn < 1 else 1
    S.check(round(mn, nd), qn, f"median gap now, variant -{k[0]}, budget {k[1]}")

XMIN, XMAX = 0.01, 100.0   # percent, log scale; gaps of 0 sit at the left edge


def ecdf(ax, vals, **kw):
    v = np.sort(np.clip(np.asarray(vals) * 100, XMIN * 0.5, None))
    n = len(v)
    x = np.concatenate([[XMIN * 0.5], v, [XMAX * 2]])
    y = np.concatenate([[0], np.arange(1, n + 1) / n, [1]])
    ax.step(x, y, where="post", **kw)


LIT = dict(color=S.BLUE, lw=1.1, ls=(0, (3.5, 1.8)))
NOW = dict(color=S.ORANGE, lw=1.4)
BUDGETS = ["0.25", "0.50", "0.75"]
TYPES = [("one", "one follower\nknapsack"), ("several", "several follower\nknapsacks")]

fig, axs = S.plt.subplots(2, 3, figsize=(S.FULL, 3.0), sharex=True, sharey=True,
                          gridspec_kw={"hspace": 0.18, "wspace": 0.10})
for i, (ft, flab) in enumerate(TYPES):
    for j, a in enumerate(BUDGETS):
        ax = axs[i, j]
        v = pairs[(ft, a)]
        ax.axhline(0.5, color=S.FILL, lw=0.6, zorder=0)
        ecdf(ax, [x for x, _ in v], **LIT)
        ecdf(ax, [y for _, y in v], **NOW)
        ax.set_xscale("log")
        ax.set_xlim(XMIN * 0.55, XMAX * 1.8)
        ax.set_ylim(0, 1.02)
        ax.xaxis.set_major_locator(FixedLocator([0.01, 0.1, 1, 10, 100]))
        ax.xaxis.set_minor_formatter(NullFormatter())
        ax.set_xticklabels(["0.01", "0.1", "1", "10", "100"])
        ax.set_yticks([0, 0.5, 1])
        ax.set_yticklabels(["0", "0.5", "1"])
        ax.grid(axis="x", which="major", color=S.FILL, lw=0.6)
        ax.set_axisbelow(True)
        ax.text(0.03, 0.95, f"n = {len(v)}", transform=ax.transAxes, ha="left", va="top",
                fontsize=7, color=S.INK2)
        if i == 0:
            ax.set_title(f"budget {int(float(a) * 100)}%", fontsize=8, pad=3)
        if j == 0:
            ax.set_ylabel(f"{flab}\n\nshare of instances", fontsize=7.5, linespacing=1.0)
        if i == 1:
            ax.set_xlabel("relative gap (%, log scale)", labelpad=2)

hs = [Line2D([], [], label="literature alone", **LIT),
      Line2D([], [], label="literature + this work", **NOW)]
axs[0, 0].legend(handles=hs, ncol=2, loc="lower left", bbox_to_anchor=(0.0, 1.13),
                 handlelength=2.6, columnspacing=1.6, borderaxespad=0)
S.save(fig, "fig_or_gaps")
