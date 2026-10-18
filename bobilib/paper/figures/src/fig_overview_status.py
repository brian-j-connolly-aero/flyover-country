"""Where BOBILib's 1,594 unsolved instances went.

Data: results/summary.json (per_instance: BOBILib status, outcome, source).
"""

import collections
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))  # style.py (works under python3 -I)
import style as S  # noqa: E402
from matplotlib.patches import Rectangle  # noqa: E402

S.setup()
pi = S.load_summary()
U = S.unsolved(pi)
cnt = collections.Counter(S.outcome_class(v) for v in U.values())
st = collections.Counter(v["bobilib"] for v in U.values())

S.check(len(U), 1594, "feasible or open in BOBILib")
S.check(dict(cnt), {"lit": 527, "new": 81, "none": 40, "first": 230, "unchanged": 716},
        "outcomes of the 1,594")
S.check(dict(st), {"feasible": 1175, "open": 419}, "BOBILib composition")
settled = cnt["lit"] + cnt["new"] + cnt["none"]
S.check(settled, 648, "settled")

N = len(U)
fig, ax = S.plt.subplots(figsize=(S.FULL, 1.4))

Y1, H1 = 1.0, 0.50     # outcome bar
Y0, H0 = 0.22, 0.22    # BOBILib's own composition (thin bar)

left = 0
seg = {}
for k in S.ORDER:
    w = cnt[k]
    S.hbar_segment(ax, Y1, left, w, H1, k)
    seg[k] = (left, w)
    left += w
# thin white gaps between segments
for k in S.ORDER[1:]:
    ax.plot([seg[k][0]] * 2, [Y1 - H1 / 2, Y1 + H1 / 2], color="white", lw=1.0,
            solid_capstyle="butt", zorder=3)

# wide segments: labels inside
inside = {
    "lit": f"optimal in published work  {S.fmt(cnt['lit'])}",
    "first": f"first verified\npoint  {S.fmt(cnt['first'])}",
    "unchanged": f"unchanged  {S.fmt(cnt['unchanged'])}",
}
for k, txt in inside.items():
    l, w = seg[k]
    box = None
    if S.OUTCOMES[k][2]:  # textured face: put the label on a solid patch
        box = dict(boxstyle="square,pad=0.25", facecolor=S.OUTCOMES[k][1], edgecolor="none")
    ax.text(l + w / 2, Y1, txt, ha="center", va="center", fontsize=7.5,
            color=S.OUTCOMES[k][4], linespacing=1.0, zorder=4, bbox=box)

# narrow segments: labels above with short leader lines, one to each side
top = Y1 + H1 / 2
ty = top + 0.22
for k, ha, txt in (("new", "right", f"proven optimal here  {S.fmt(cnt['new'])}"),
                   ("none", "left", f"no feasible point  {S.fmt(cnt['none'])}")):
    l, w = seg[k]
    x = l + w / 2
    dx = 5 if ha == "left" else -5
    ax.plot([x, x, x + dx], [top + 0.02, ty, ty], color=S.INK2, lw=0.5)
    ax.text(x + 1.8 * dx, ty, txt, ha=ha, va="center", fontsize=7.5)

# settled bracket below the outcome bar
yb = Y1 - H1 / 2 - 0.07
ax.plot([0, 0, settled, settled], [yb + 0.05, yb, yb, yb + 0.05], color=S.INK2, lw=0.6)
ax.text(settled / 2, yb - 0.03, f"settled  {S.fmt(settled)} ({100 * settled / N:.0f}%)",
        ha="center", va="top", fontsize=7)

# BOBILib's composition of the same 1,594
fe, op = st["feasible"], st["open"]
ax.barh(Y0, fe, left=0, height=H0, color=S.MUTED, linewidth=0)
ax.add_patch(Rectangle((fe, Y0 - H0 / 2), op, H0, facecolor="white", edgecolor=S.MUTED, lw=0.6))
ax.text(fe / 2, Y0, f"feasible (a point known)  {S.fmt(fe)}", ha="center", va="center", fontsize=7)
ax.text(fe + op / 2, Y0, f"open (no point known)  {S.fmt(op)}", ha="center", va="center", fontsize=7)

ax.set_yticks([Y1, Y0])
ax.set_yticklabels(["now", "BOBILib"], fontsize=8)
ax.tick_params(axis="y", length=0, pad=4)
ax.spines["left"].set_visible(False)
ax.set_xlim(0, N)
ax.set_ylim(Y0 - H0 / 2 - 0.03, ty + 0.12)
ticks = [0, 400, 800, 1200, N]
ax.set_xticks(ticks)
ax.set_xticklabels([S.fmt(t) for t in ticks])
ax.set_xlabel("instances", labelpad=2)
S.save(fig, "fig_overview_status")
