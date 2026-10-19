"""The 16 knapsack interdiction instances open in the literature: search nodes
before and after the per-node tables, and CPU seconds after.

Data: after = results/ss/proofs.csv (nodes, cpu_seconds; one thread).
      before = bobilib/REPORT.md, table "Instance | Optimum | Nodes | Time |
      Before (nodes / time)" (previous version of the same search, same machine).
      results/ss/summary.csv is NOT used: it holds an older run whose numbers
      differ from REPORT.md's 'before' column.
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))  # style.py (works under python3 -I)
import style as S  # noqa: E402
from matplotlib.lines import Line2D  # noqa: E402
from matplotlib.ticker import LogLocator, NullFormatter  # noqa: E402

S.setup()
after = {r["name"]: r for r in S.read_csv("ss/proofs.csv")}
before = S.report_kip_before()

S.check(len(after), 16, "instances in ss/proofs.csv")
S.check(sorted(before), sorted(after), "instances in REPORT.md 'before' table")
for r in after.values():
    S.check(r["optimum"], r["bobilib_value"], f"{r['name']} optimum = BOBILib value")
hard = after["interKP-500-100-6-3"]
S.check((hard["optimum"], int(hard["nodes"]), round(float(hard["cpu_seconds"]), 1)),
        ("6976", 10079, 10.0), "hardest instance")
nofin = sorted(k for k, v in before.items() if not isinstance(v, tuple))
S.check(nofin, [f"interKP-{n}-100-6-3" for n in (200, 300, 400, 500)], "no 'before' result")

# rows top to bottom: class 6 then 7; budget index 2 then 3; n = 200..500
rows = [f"interKP-{n}-100-{c}-{b}" for c in (6, 7) for b in (2, 3) for n in (200, 300, 400, 500)]
ypos, y = {}, 0.0
for i, name in enumerate(rows):
    if i and i % 4 == 0:
        y += 0.6 if i % 8 else 1.2   # small gap between budget indices, larger between classes
    ypos[name] = y
    y += 1
ymax = y - 1

fig, (ax, bx) = S.plt.subplots(1, 2, figsize=(S.FULL, 2.4), sharey=True,
                               gridspec_kw={"width_ratios": [3.3, 1], "wspace": 0.08})
# No node count exists for these four 'before' runs, so nothing is plotted for
# them (a line or arrow would suggest a count); a text label says why.
NOTE = {"not finished in 2 h": "before: not finished", "not finished in 3 h": "before: not finished",
        "not finished in 4 h": "before: not finished",
        "not run to the end": "before: not finished", "not run": "before: not run"}
XMAX = 6e7
for name in rows:
    yy = ypos[name]
    na = int(after[name]["nodes"])
    b = before[name]
    if isinstance(b, tuple):
        ax.plot([na, b[0]], [yy, yy], color=S.MUTED, lw=1.0, zorder=1)
        ax.plot(b[0], yy, "o", ms=4.5, mfc="white", mec=S.INK2, mew=0.9, zorder=2)
    else:
        ax.text(6e4, yy, NOTE[b], va="center", ha="left", fontsize=7, color=S.INK2,
                bbox=dict(boxstyle="square,pad=0.05", facecolor="white", edgecolor="none"))
    ax.plot(na, yy, "o", ms=4.5, color=S.AQUA, mec="white", mew=0.5, zorder=3)
    bx.plot(float(after[name]["cpu_seconds"]), yy, "o", ms=4.5, color=S.AQUA, mec="white",
            mew=0.5, zorder=3)

for a in (ax, bx):
    a.set_xscale("log")
    a.grid(axis="x", which="major", color=S.FILL, lw=0.6)
    a.set_axisbelow(True)
    a.tick_params(axis="y", length=0)
ax.set_xlim(0.6, XMAX)
ax.xaxis.set_major_locator(LogLocator(base=10, numticks=12))
ax.xaxis.set_minor_formatter(NullFormatter())
ax.set_xticks([1, 10, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7])
ax.set_xlabel("search nodes (log scale)", labelpad=2)
bx.set_xlim(0.025, 30)
bx.set_xticks([0.1, 1, 10])
bx.set_xticklabels(["0.1", "1", "10"])
bx.set_xlabel("CPU seconds after (log scale)", labelpad=2)

ax.set_yticks([ypos[n] for n in rows])
ax.set_yticklabels([n.replace("interKP-", "").replace("-100-", "-") for n in rows], fontsize=7)
ax.set_ylim(ymax + 0.7, -0.7)
ax.spines["left"].set_visible(False)
bx.spines["left"].set_visible(False)

# class brackets at the far left
for c, lab in ((6, "class 6\n(subset sum)"), (7, "class 7\n(even-odd)")):
    ys = [ypos[n] for n in rows if f"-{c}-" in n.replace("interKP-", "-")]
    x = -0.155
    ax.plot([x + 0.012, x, x, x + 0.012], [min(ys) - 0.3, min(ys) - 0.3, max(ys) + 0.3, max(ys) + 0.3],
            transform=ax.get_yaxis_transform(), color=S.INK2, lw=0.6, clip_on=False)
    ax.text(x - 0.012, (min(ys) + max(ys)) / 2, lab, transform=ax.get_yaxis_transform(),
            rotation=90, ha="right", va="center", fontsize=7, linespacing=1.0)

hs = [Line2D([], [], marker="o", ls="", ms=4.5, mfc="white", mec=S.INK2, mew=0.9,
             label="before (previous search)"),
      Line2D([], [], marker="o", ls="", ms=4.5, color=S.AQUA, mec="white", mew=0.5,
             label="after (per-node tables)")]
ax.legend(handles=hs, ncol=2, loc="lower left", bbox_to_anchor=(0.0, 1.0), handletextpad=0.3,
          columnspacing=1.5, borderaxespad=0.2)
S.save(fig, "fig_kip_search")
