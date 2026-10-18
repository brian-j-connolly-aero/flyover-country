"""Edge clique interdiction (BOBILib set 'clique', 60 instances): BOBILib's value
against the proven optimum; the three open instances shown as the interval [3, 4].

Data: results/clique/summary.csv (n, density, bobilib_value, lower_bound, upper_bound, status).
"""

import collections
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))  # style.py (works under python3 -I)
import style as S  # noqa: E402
from matplotlib.lines import Line2D  # noqa: E402
from matplotlib.patches import Rectangle  # noqa: E402

S.setup()
rows = S.read_csv("clique/summary.csv")
S.check(len(rows), 60, "clique instances")
st = collections.Counter(r["status"] for r in rows)
S.check(dict(st), {"optimal": 57, "open": 3}, "clique status")
opt = collections.Counter((r["density"], r["upper_bound"]) for r in rows if r["status"] == "optimal")
S.check(dict(opt), {("0.7", "3"): 27, ("0.9", "4"): 30}, "clique optima by density")
openi = sorted(r["name"] for r in rows if r["status"] == "open")
S.check(openi, ["normalClique60-70-2", "normalClique60-70-3", "normalClique60-70-5"], "open three")
S.check(all((r["lower_bound"], r["upper_bound"]) == ("3", "4") for r in rows if r["status"] == "open"),
        True, "open three bracketed by 3 and 4")
bv = [int(r["bobilib_value"]) for r in rows]
S.check((min(bv), max(bv)), (5, 13), "range of BOBILib values")

GROUPS = [(d, n) for d in ("0.7", "0.9") for n in ("40", "50", "60")]
idx = lambda name: int(name.rsplit("-", 1)[1])
fig, ax = S.plt.subplots(figsize=(S.FULL, 2.2))
x0 = 0.0
centers = {}
W = 0.42  # width of the open-interval box
for gi, (d, n) in enumerate(GROUPS):
    if gi:
        x0 += 2.6 if gi == 3 else 1.6
    g = sorted((r for r in rows if r["density"] == d and r["n"] == n), key=lambda r: idx(r["name"]))
    S.check(len(g), 10, f"group n={n} d={d}")
    xs = []
    for r in g:
        x = x0 + idx(r["name"]) - 1
        xs.append(x)
        b = int(r["bobilib_value"])
        lo, hi = int(r["lower_bound"]), int(r["upper_bound"])
        ax.plot([x, x], [hi, b], color=S.MUTED, lw=1.0, zorder=1)
        ax.plot(x, b, "o", ms=4.5, mfc="white", mec=S.INK2, mew=0.9, zorder=3)
        if r["status"] == "optimal":
            ax.plot(x, lo, "o", ms=4.5, color=S.AQUA, mec="white", mew=0.4, zorder=3)
        else:
            ax.add_patch(Rectangle((x - W / 2, lo), W, hi - lo, facecolor="white",
                                   edgecolor=S.AQUA, lw=1.1, zorder=3))
    centers[(d, n)] = (min(xs) + max(xs)) / 2
    x0 = max(xs) + 1

ax.set_ylim(1.6, 14.0)
ax.set_yticks(range(2, 15, 2))
ax.set_yticks(range(2, 15), minor=True)
ax.grid(axis="y", which="major", color=S.FILL, lw=0.6)
ax.set_axisbelow(True)
ax.set_ylabel("clique number after deletion")
ax.set_xlim(-1, x0)
ax.set_xticks([centers[g] for g in GROUPS])
ax.set_xticklabels([f"n = {n}" for _, n in GROUPS], fontsize=7)
ax.tick_params(axis="x", length=0, pad=2)
ax.spines["bottom"].set_visible(False)
for d in ("0.7", "0.9"):
    cs = [centers[g] for g in GROUPS if g[0] == d]
    ax.text((min(cs) + max(cs)) / 2, -0.17, f"edge density {d}", transform=ax.get_xaxis_transform(),
            ha="center", va="top", fontsize=7.5)
    ax.plot([min(cs) - 5, max(cs) + 5], [-0.12, -0.12], transform=ax.get_xaxis_transform(),
            color=S.INK2, lw=0.6, clip_on=False)

hs = [Line2D([], [], marker="o", ls="", ms=4.5, mfc="white", mec=S.INK2, mew=0.9, label="BOBILib value"),
      Line2D([], [], marker="o", ls="", ms=4.5, color=S.AQUA, mec="white", mew=0.4,
             label="proven optimum"),
      Line2D([], [], marker="s", ls="", ms=5, mfc="white", mec=S.AQUA, mew=1.1,
             label="open: 3 or 4")]
ax.legend(handles=hs, ncol=3, loc="lower left", bbox_to_anchor=(0.0, 1.0), handletextpad=0.3,
          columnspacing=1.6, borderaxespad=0.2)
S.save(fig, "fig_clique_values")
