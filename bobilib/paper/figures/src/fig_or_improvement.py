"""OR set: relative improvement of our verified point over the best published
point, (published - ours) / published, where 'published' is the lower of the
literature's best point (lit_ub) and BOBILib's value. Instances with no
published point (88 first points) have no ratio; their count is printed. The
instances whose published lower bound exceeds the published point
(inconsistent sources) are left out of the curves.

Data: results/or/compare.csv (columns variant, alpha, bobilib, lit_ub, our_point).
"""

import collections
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))  # style.py (works under python3 -I)
import numpy as np  # noqa: E402
import style as S  # noqa: E402
from matplotlib.lines import Line2D  # noqa: E402

S.setup()
rows = S.read_csv("or/compare.csv")
imp = collections.defaultdict(list)    # (follower type, budget) -> [improvement %]
first = collections.Counter()           # budget -> first points
better = n_points = left_out = 0
for r in rows:
    p = S.num(r["our_point"])
    if p is None:
        continue
    n_points += 1
    pubs = [x for x in (S.num(r["lit_ub"]), S.num(r["bobilib"])) if x is not None]
    if not pubs:
        first[r["alpha"]] += 1
        continue
    pub = min(pubs)
    better += p < pub
    if S.num(r["lit_lb"]) is not None and S.num(r["lit_lb"]) > S.num(r["lit_ub"]):
        left_out += 1   # published bounds contradict each other: no baseline
        continue
    ft = "one" if r["variant"] == "100" else "several"
    imp[(ft, r["alpha"])].append(100 * (pub - p) / pub)

S.check(n_points, 690, "verified OR points")
S.check(better, 474, "points strictly better than every published value")
S.check(sum(first.values()), 88, "first points anywhere")
S.check(dict(first), {"0.50": 51, "0.75": 37}, "first points per budget (31+20, 27+10)")
S.check(left_out, 1, "points left out: inconsistent published bounds (the closure)")

XL, XR = -10.0, 40.0
BUDGETS = ["0.25", "0.50", "0.75"]
# Black and grey (not the orange/blue of the gap figure, where they mean "this work"/"literature").
STY = {"one": dict(color=S.INK, lw=1.3, label="one follower knapsack"),
       "several": dict(color=S.MUTED, lw=1.3, ls=(0, (3.5, 1.8)), label="several follower knapsacks")}

fig, axs = S.plt.subplots(1, 3, figsize=(S.FULL, 1.95), sharey=True, gridspec_kw={"wspace": 0.12})
for ax, a in zip(axs, BUDGETS):
    ax.axvline(0, color=S.MUTED, lw=0.7, zorder=0)
    ax.axhline(0.5, color=S.FILL, lw=0.6, zorder=0)
    below = 0
    for ft in ("one", "several"):
        v = np.sort(np.asarray(imp[(ft, a)]))
        n = len(v)
        below += int((v < XL).sum())
        x = np.concatenate([[XL - 5], np.clip(v, XL - 1, None), [XR + 5]])
        y = np.concatenate([[0], np.arange(1, n + 1) / n, [1]])
        ax.step(x, y, where="post", **STY[ft])
    ax.set_xlim(XL - 2.5, XR + 2.5)
    ax.set_ylim(0, 1.02)
    ax.set_xticks([-10, 0, 10, 20, 30, 40])
    ax.set_yticks([0, 0.5, 1])
    ax.set_yticklabels(["0", "0.5", "1"])
    ax.grid(axis="x", color=S.FILL, lw=0.6)
    ax.set_axisbelow(True)
    ax.set_title(f"budget {int(float(a) * 100)}%", fontsize=8, pad=3)
    n1, n2 = len(imp[("one", a)]), len(imp[("several", a)])
    txt = f"compared: {n1} + {n2}\nfirst points: {first[a]}"
    if below:
        txt += f"\nleft of −10%: {below}"
    ax.text(0.97, 0.05, txt, transform=ax.transAxes, ha="right", va="bottom", fontsize=7,
            color=S.INK2, linespacing=1.15)
axs[0].set_ylabel("share of instances")
axs[1].set_xlabel("improvement over the best published point (%)", labelpad=2)
hs = [Line2D([], [], **STY[k]) for k in ("one", "several")]
axs[0].legend(handles=hs, ncol=2, loc="lower left", bbox_to_anchor=(0.0, 1.13),
              handlelength=2.6, columnspacing=1.6, borderaxespad=0)
S.save(fig, "fig_or_improvement")
