"""OR set (multidimensional knapsack interdiction): empirical distribution of the
relative gap (best upper - best lower) / best upper, literature alone vs
literature plus this work, over the instances with both a published lower and a
published upper bound (paired), leaving out the instances whose published
lower bound exceeds the published point (inconsistent sources; listed in the
supplement). A gap of 0 (closed instance) is drawn at a separate "0" tick
left of an axis break; no paired gap is negative.

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

conflict = {r["name"] for r in rows if r["lit_lb"] and r["lit_ub"] and float(r["lit_lb"]) > float(r["lit_ub"])}
S.check(len(conflict), 4, "OR instances with a published lower bound above the published point")
pairs = collections.defaultdict(list)       # (follower type, budget) -> [(lit, now)]
by_variant = collections.defaultdict(list)  # (variant, budget) -> [(lit, now)]
for r in rows:
    if r["gap"] and r["lit_gap"] and r["name"] not in conflict:
        lit, now = float(r["lit_gap"]), float(r["gap"])
        ft = "one" if r["variant"] == "100" else "several"
        pairs[(ft, r["alpha"])].append((lit, now))
        by_variant[(r["variant"], r["alpha"])].append((lit, now))

# Paired medians quoted in the paper (upper median, as scripts/compare_or.py), in %.
quoted = {("100", "0.25"): (24.0, 4.1), ("100", "0.50"): (34.0, 7.1), ("100", "0.75"): (0.14, 0.14),
          ("0", "0.25"): (33.3, 31.8), ("0", "0.50"): (36.6, 29.1), ("0", "0.75"): (5.3, 0.05),
          ("50", "0.25"): (30.6, 28.3), ("50", "0.50"): (41.3, 34.8), ("50", "0.75"): (11.8, 1.4)}
for k, (ql, qn) in quoted.items():
    v = by_variant[k]
    ml = 100 * S.upper_median([a for a, _ in v])
    mn = 100 * S.upper_median([b for _, b in v])
    nd = 2 if ql < 1 else 1
    S.check(round(ml, nd), ql, f"median literature gap, variant -{k[0]}, budget {k[1]}")
    nd = 2 if qn < 1 else 1
    S.check(round(mn, nd), qn, f"median gap now, variant -{k[0]}, budget {k[1]}")

XMIN, XMAX = 0.01, 100.0   # percent, log scale
# Gaps of 0 (closed instances) are drawn at XZ, a separate "0" tick left of an
# axis break. The smallest positive gap is 0.012%, so nothing lies between the
# break and 0.01.
XZ = 0.002
XL = 0.0013                # left end of the axis
S.check(min(x for v in pairs.values() for p in v for x in p) >= 0, True, "no negative paired gap")
pos = [float(r[c]) * 100 for r in rows for c in ("gap", "lit_gap") if r[c] and float(r[c]) > 0]
S.check(min(pos) > XMIN, True, "every positive gap is right of the axis break")


def ecdf(ax, vals, **kw):
    v = np.asarray(vals, dtype=float) * 100
    v = np.sort(np.where(v <= 0, XZ, v))
    n = len(v)
    x = np.concatenate([[XL * 0.5], v, [XMAX * 2]])
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
        ax.set_xlim(XL, XMAX * 1.8)
        ax.set_ylim(0, 1.02)
        ax.xaxis.set_major_locator(FixedLocator([XZ, 0.01, 0.1, 1, 10, 100]))
        ax.xaxis.set_minor_locator(FixedLocator(
            [k * 10.0 ** e for e in range(-2, 3) for k in range(2, 10) if k * 10.0 ** e < XMAX * 1.8]))
        ax.xaxis.set_minor_formatter(NullFormatter())
        ax.set_xticklabels(["0", "0.01", "0.1", "1", "10", "100"])
        ax.set_yticks([0, 0.5, 1])
        ax.set_yticklabels(["0", "0.5", "1"])
        for g in (0.01, 0.1, 1, 10, 100):
            ax.axvline(g, color=S.FILL, lw=0.6, zorder=0)
        # axis break between the "0" tick and 0.01
        xb = (XZ * XMIN) ** 0.5
        ax.plot([xb / 1.25, xb * 1.25], [0, 0], transform=ax.get_xaxis_transform(), color="white",
                lw=2.5, solid_capstyle="butt", clip_on=False, zorder=5)
        ax.plot([xb / 1.25, xb * 1.25], [0, 0], transform=ax.get_xaxis_transform(), ls="none",
                marker=[(-1, -1.6), (1, 1.6)], ms=5, mew=0.6, color=S.INK2, clip_on=False, zorder=6)
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
