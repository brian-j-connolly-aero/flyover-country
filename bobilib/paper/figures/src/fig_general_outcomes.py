"""The 266 MIPLIB-derived instances BOBILib lists as feasible or open: outcomes,
split by BOBILib status and, for the open ones, by coupling rows (leader rows
that contain follower variables).

Data: results/summary.json (status, outcome per instance);
      results/general/points.csv (one row per open instance without coupling rows, 98);
      results/general-coupled/summary.csv (open instances with coupling rows that were run).
"""

import collections
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))  # style.py (works under python3 -I)
import style as S  # noqa: E402

S.setup()
U = S.unsolved(S.load_summary())
G = {k: v for k, v in U.items() if v["family"] in ("general-pure", "general-mixed")}
nocpl = {r["name"] for r in S.read_csv("general/points.csv")}
cpl_run = {r["name"] for r in S.read_csv("general-coupled/summary.csv")}

S.check(len(G), 266, "MIPLIB-derived instances, feasible or open")
S.check(len(nocpl), 98, "open instances without coupling rows (points.csv)")
S.check(all(G.get(k, {}).get("bobilib") == "open" for k in nocpl | cpl_run), True,
        "points.csv / general-coupled rows are open MIPLIB-derived instances")
S.check(len(nocpl & cpl_run), 0, "no instance in both points.csv and general-coupled")

BARS = [("open-nocpl", "open, no\ncoupling rows"), ("open-cpl", "open, with\ncoupling rows"),
        ("feasible", "feasible\n(point known)")]
cnt = {b: collections.Counter() for b, _ in BARS}
for k, v in G.items():
    b = "feasible" if v["bobilib"] == "feasible" else ("open-nocpl" if k in nocpl else "open-cpl")
    cnt[b][S.outcome_class(v)] += 1

S.check(sum(cnt["open-cpl"].values()), 87, "open instances with coupling rows")
S.check(cnt["open-nocpl"]["first"], 60, "first points without coupling rows")
S.check(cnt["open-cpl"]["first"], 28, "first points with coupling rows")
tot = sum(cnt.values(), collections.Counter())
S.check(dict(tot), {"lit": 8, "new": 7, "none": 40, "first": 88, "unchanged": 123},
        "outcomes of the 266")

fig, ax = S.plt.subplots(figsize=(S.HALF, 1.75))
H = 0.46  # thinner bars leave room for the labels of narrow segments above them
XMAX = max(sum(c.values()) for c in cnt.values())
per_in = 1.75 / XMAX  # approximate inches per instance, for the label fit test
for i, (b, _) in enumerate(BARS):
    y = len(BARS) - 1 - i
    c = cnt[b]
    left = 0
    above = []
    for k in S.ORDER:
        w = c[k]
        if not w:
            continue
        S.hbar_segment(ax, y, left, w, H, k)
        if left:
            ax.plot([left, left], [y - H / 2, y + H / 2], color="white", lw=0.8,
                    solid_capstyle="butt", zorder=3)
        if w * per_in >= 0.13:
            box = None
            if S.OUTCOMES[k][2]:
                box = dict(boxstyle="square,pad=0.25", facecolor=S.OUTCOMES[k][1], edgecolor="none")
            ax.text(left + w / 2, y, str(w), ha="center", va="center", fontsize=7,
                    color=S.OUTCOMES[k][4], zorder=4, bbox=box)
        else:
            above.append((left + w / 2, w))
        left += w
    for x, w in above:  # narrow segments: count above the bar, on a leader line from the segment
        ax.plot([x, x + 1.2], [y + H / 2 - 0.02, y + H / 2 + 0.13], color=S.INK, lw=0.7,
                solid_capstyle="butt", zorder=4)
        ax.text(x + 1.4, y + H / 2 + 0.09, str(w), ha="left", va="bottom", fontsize=7, zorder=4)
    ax.text(left + 1.5, y, str(left), ha="left", va="center", fontsize=7, color=S.INK2)

ax.set_yticks(range(len(BARS)))
ax.set_yticklabels([lab for _, lab in BARS][::-1], fontsize=7, linespacing=1.0)
ax.tick_params(axis="y", length=0, pad=3)
ax.spines["left"].set_visible(False)
ax.set_xlim(0, XMAX + 9)
ax.set_ylim(-0.5, len(BARS) - 0.3)
ax.set_xticks([0, 25, 50, 75, 100])
ax.set_xlabel("instances", labelpad=2)
hs, hmap = S.legend_handles()
ax.legend(handles=hs, handler_map=hmap, ncol=2, loc="lower left", bbox_to_anchor=(-0.36, 1.0),
          handlelength=1.3, handleheight=0.9, columnspacing=1.0, handletextpad=0.4,
          borderaxespad=0.2, fontsize=7)
S.save(fig, "fig_general_outcomes")
