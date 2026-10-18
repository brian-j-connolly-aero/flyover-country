"""Outcome composition per BOBILib family, over the instances BOBILib lists as
feasible or open (1,594; the 33 infeasible general instances are not counted).

Data: results/summary.json (per_instance: family, BOBILib status, outcome, source).
"""

import collections
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))  # style.py (works under python3 -I)
import style as S  # noqa: E402

S.setup()
U = S.unsolved(S.load_summary())

FAMILIES = [  # summary.json family -> row label
    ("knapsack", "Knapsack interdiction"),
    ("multidim-knapsack", "Multidim. knapsack interdiction"),
    ("clique", "Edge clique interdiction"),
    ("network", "Network interdiction"),
    ("generalized", "Generalized interdiction"),
    ("general-pure", "MIPLIB-derived, pure integer"),
    ("general-mixed", "MIPLIB-derived, mixed integer"),
]
cnt = {f: collections.Counter() for f, _ in FAMILIES}
for v in U.values():
    cnt[v["family"]][S.outcome_class(v)] += 1

# Check against the numbers quoted in the paper.
expect = {
    "knapsack": {"lit": 376, "new": 16},
    "multidim-knapsack": {"lit": 139, "new": 1, "first": 140, "unchanged": 582},
    "clique": {"new": 57, "unchanged": 3},
    "network": {"new": 2},
    "generalized": {"lit": 4, "unchanged": 8},
}
for f, e in expect.items():
    S.check(dict(cnt[f]), e, f"outcomes, {f}")
gen = cnt["general-pure"] + cnt["general-mixed"]
S.check(dict(gen), {"lit": 8, "new": 5, "none": 40, "first": 90, "unchanged": 123},
        "outcomes, MIPLIB-derived (266)")
S.check(sum(sum(c.values()) for c in cnt.values()), 1594, "total")

fig, ax = S.plt.subplots(figsize=(S.FULL, 2.4))
H = 0.62
MIN_IN = 0.11  # narrowest segment (inches) that gets its count inside
bar_in = 4.25  # approximate bar length in inches, for the fit test
for i, (f, _) in enumerate(FAMILIES):
    y = len(FAMILIES) - 1 - i
    c = cnt[f]
    n = sum(c.values())
    left = 0.0
    for k in S.ORDER:
        if not c[k]:
            continue
        w = c[k] / n
        S.hbar_segment(ax, y, left, w, H, k)
        if left > 0:
            ax.plot([left, left], [y - H / 2, y + H / 2], color="white", lw=1.0,
                    solid_capstyle="butt", zorder=3)
        txt = S.fmt(c[k])
        if w * bar_in >= MIN_IN:
            box = None
            if S.OUTCOMES[k][2]:  # striped face: put the number on a solid patch
                box = dict(boxstyle="square,pad=0.3", facecolor=S.OUTCOMES[k][1], edgecolor="none")
            ax.text(left + w / 2, y, txt, ha="center", va="center", fontsize=7,
                    color=S.OUTCOMES[k][4], zorder=4, bbox=box)
        else:  # too narrow: count just above the bar
            ax.text(left + w / 2, y + H / 2 + 0.02, txt, ha="center", va="bottom",
                    fontsize=6.5, color=S.INK, zorder=4)
        left += w
    ax.text(1.015, y, S.fmt(n), ha="left", va="center", fontsize=7.5)
ax.text(1.015, len(FAMILIES) - 1 + H / 2 + 0.03, "total", ha="left", va="bottom",
        fontsize=7, color=S.INK2)

ax.set_yticks(range(len(FAMILIES)))
ax.set_yticklabels([lab for _, lab in FAMILIES][::-1], fontsize=7.5)
ax.tick_params(axis="y", length=0, pad=4)
ax.spines["left"].set_visible(False)
ax.set_xlim(0, 1)
ax.set_ylim(-0.5, len(FAMILIES) - 0.25)
ax.set_xticks([0, 0.25, 0.5, 0.75, 1])
ax.set_xticklabels(["0", "25", "50", "75", "100"])
ax.set_xlabel("share of the family's unsolved instances (%)", labelpad=2)
hs, hmap = S.legend_handles()
ax.legend(handles=hs, handler_map=hmap, ncol=5, loc="lower right", bbox_to_anchor=(1.0, 1.0),
          handlelength=1.4, handleheight=0.9, columnspacing=1.2, handletextpad=0.5,
          borderaxespad=0.3)
S.save(fig, "fig_family_outcomes")
