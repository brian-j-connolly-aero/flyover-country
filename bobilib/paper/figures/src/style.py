"""Shared style, palette, data access and checks for the paper's data figures.

Every figure script reads only files under bobilib/results/ (and, for the
'before' search statistics of the knapsack interdiction figure, the table in
bobilib/REPORT.md). Each script checks its totals against the numbers quoted
in the paper and stops with an error instead of plotting if they disagree.
"""

import csv
import json
import re
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

HERE = Path(__file__).resolve().parent
BOBILIB = HERE.parents[2]           # .../bobilib
RESULTS = BOBILIB / "results"
REPORT = BOBILIB / "REPORT.md"
OUT = HERE.parent                   # .../bobilib/paper/figures

# Sizes (inches)
FULL = 6.5
HALF = 3.15

# Palette (validated colour-blind safe, light/print mode)
BLUE = "#2a78d6"     # categorical slot 1: literature
ORANGE = "#eb6834"   # categorical slot 2: first points
AQUA = "#1baf7a"     # categorical slot 3: new optimum / certificates
INK = "#0b0b0b"
INK2 = "#52514e"     # secondary ink
MUTED = "#a9a8a2"
FILL = "#e6e5e0"     # grid / neutral fill
WHITE = "#ffffff"

# The five outcomes for an instance BOBILib lists as feasible or open.
# key -> (legend label, face colour, texture, texture colour, text colour on the face)
# Textures are the second cue for greyscale print: aqua and orange have almost
# the same grey value, so "first verified point" carries white dots (points)
# and "no feasible point" white stripes; "proven optimal here" stays solid.
OUTCOMES = {
    "lit": ("optimal in published work", BLUE, None, None, WHITE),
    "new": ("proven optimal here", AQUA, None, None, INK),
    "none": ("no feasible point", AQUA, "stripes", WHITE, INK),
    "first": ("first verified point", ORANGE, "dots", WHITE, INK),
    "unchanged": ("unchanged", FILL, None, None, INK2),
}
ORDER = ["lit", "new", "none", "first", "unchanged"]


def setup():
    plt.rcParams.update({
        "font.family": "serif",
        "font.serif": ["Liberation Serif", "DejaVu Serif"],
        "mathtext.fontset": "stix",
        "font.size": 8,
        "axes.titlesize": 8,
        "axes.labelsize": 8,
        "xtick.labelsize": 7,
        "ytick.labelsize": 7,
        "legend.fontsize": 7,
        "legend.frameon": False,
        "axes.edgecolor": INK2,
        "axes.labelcolor": INK,
        "axes.linewidth": 0.6,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.grid": False,
        "grid.color": FILL,
        "grid.linewidth": 0.6,
        "xtick.color": INK2,
        "ytick.color": INK2,
        "xtick.labelcolor": INK,
        "ytick.labelcolor": INK,
        "xtick.major.width": 0.6,
        "ytick.major.width": 0.6,
        "xtick.minor.width": 0.4,
        "ytick.minor.width": 0.4,
        "xtick.major.size": 2.5,
        "ytick.major.size": 2.5,
        "xtick.minor.size": 1.5,
        "ytick.minor.size": 1.5,
        "lines.linewidth": 1.2,
        "lines.markersize": 4,
        "text.color": INK,
        "figure.facecolor": "white",
        "axes.facecolor": "white",
        "savefig.facecolor": "white",
        "pdf.fonttype": 42,
        "axes.unicode_minus": True,
    })


PAD = 0.02  # inches, as pad_inches


def fit(fig, rounds=8):
    """Move the subplot edges so that the tight bounding box (labels, legends and
    annotations included) fills the figure size exactly: the saved PDF is then
    the nominal width (6.5 in or 3.15 in), with fonts at their true size."""
    W, H = fig.get_size_inches()
    for _ in range(rounds):
        bb = fig.get_tightbbox(fig.canvas.get_renderer())
        sp = fig.subplotpars
        fig.subplots_adjust(left=sp.left + (PAD - bb.x0) / W, right=sp.right + (W - PAD - bb.x1) / W,
                            bottom=sp.bottom + (PAD - bb.y0) / H, top=sp.top + (H - PAD - bb.y1) / H)


def save(fig, name):
    fit(fig)
    _draw_stripes(fig)
    path = OUT / f"{name}.pdf"
    fig.savefig(path, bbox_inches="tight", pad_inches=PAD,
                metadata={"Creator": None, "Producer": None, "CreationDate": None})
    plt.close(fig)
    print(f"wrote {path}")


def check(actual, expected, what):
    """Stop (no figure) when a plotted number disagrees with the paper's."""
    if actual != expected:
        sys.exit(f"CHECK FAILED: {what}: data give {actual!r}, paper says {expected!r}")


def fmt(n):
    return f"{n:,}"


def upper_median(v):
    """Median as in scripts/compare_or.py (upper middle element), used in the paper."""
    v = sorted(v)
    return v[len(v) // 2]


def read_csv(rel):
    with open(RESULTS / rel, newline="") as f:
        return list(csv.DictReader(f))


def num(s):
    return float(s) if s not in ("", None) else None


def load_summary():
    with open(RESULTS / "summary.json") as f:
        return json.load(f)["per_instance"]


def outcome_class(entry):
    """Map one results/summary.json entry (BOBILib feasible or open) to an outcome key."""
    o, src = entry["outcome"], entry["source"] or ""
    if o is None:
        return "unchanged"
    if o == "optimal":
        return "lit" if src.startswith("literature") else "new"
    if o == "no bilevel feasible point":
        return "none"
    if o == "feasible":
        return "first"
    raise ValueError(f"unknown outcome {o!r}")


def unsolved(per_instance):
    """The 1,594 instances BOBILib lists as feasible or open."""
    return {k: v for k, v in per_instance.items() if v["bobilib"] in ("feasible", "open")}


STRIPE_PITCH = 2.6   # points between stripes (perpendicular)
STRIPE_LW = 0.9      # points
DOT_PITCH = 3.0      # points between dot centres (staggered rows)
DOT_D = 1.1          # dot diameter, points


def _stripe_lines(x0, y0, x1, y1, pitch):
    """45-degree segments covering the box [x0,x1] x [y0,y1] (same units in x and y)."""
    import numpy as np
    step = pitch * 2 ** 0.5
    segs = []
    for c in np.arange(x0 - (y1 - y0), x1 + step, step):
        segs.append([(c, y0), (c + (y1 - y0), y1)])
    return segs


def _dot_centres(x0, y0, x1, y1, pitch):
    """Centres of a staggered grid of dots covering the box [x0,x1] x [y0,y1]."""
    import numpy as np
    pts = []
    dy = pitch * 3 ** 0.5 / 2
    for i, yy in enumerate(np.arange(y0 + dy / 2, y1 + dy, dy)):
        off = pitch / 2 if i % 2 else 0.0
        for xx in np.arange(x0 + pitch / 4 + off - pitch, x1 + pitch, pitch):
            pts.append((xx, yy))
    return pts


def stripes(ax, x, y, w, h, color=WHITE, kind="stripes"):
    """Queue a texture (white 45-degree stripes, or white dots) over a data
    rectangle; drawn by save() once the axis limits are final. (Drawn as lines
    and dots, not a PDF hatch pattern, so every viewer shows the same strokes.)"""
    if not hasattr(ax, "_stripes"):
        ax._stripes = []
    ax._stripes.append((x, y, w, h, color, kind))


def _draw_stripes(fig):
    from matplotlib.collections import LineCollection
    from matplotlib.patches import Rectangle
    for ax in fig.axes:
        for x, y, w, h, color, kind in getattr(ax, "_stripes", []):
            t = ax.transData
            (X0, Y0), (X1, Y1) = t.transform([(x, y), (x + w, y + h)])
            X0, X1 = sorted((X0, X1))
            Y0, Y1 = sorted((Y0, Y1))
            clip = Rectangle((x, y), w, h, transform=t, facecolor="none", edgecolor="none")
            ax.add_patch(clip)
            if kind == "dots":
                pitch = DOT_PITCH * fig.dpi / 72
                pts = t.inverted().transform(_dot_centres(X0, Y0, X1, Y1, pitch))
                lims = ax.get_xlim(), ax.get_ylim()
                sc = ax.scatter(pts[:, 0], pts[:, 1], s=DOT_D ** 2, marker="o", color=color,
                                linewidths=0, zorder=2.5, clip_on=True)
                sc.set_clip_path(clip)
                ax.set_xlim(lims[0])
                ax.set_ylim(lims[1])
            else:
                pitch = STRIPE_PITCH * fig.dpi / 72
                segs = [t.inverted().transform(s) for s in _stripe_lines(X0, Y0, X1, Y1, pitch)]
                lc = LineCollection(segs, colors=color, linewidths=STRIPE_LW, transform=t,
                                    zorder=2.5)
                ax.add_collection(lc, autolim=False)
                lc.set_clip_path(clip)
        ax._stripes = []


def hbar_segment(ax, y, left, width, height, key, **kw):
    _, face, texture, tcol, _ = OUTCOMES[key]
    bars = ax.barh(y, width, left=left, height=height, color=face, linewidth=0, **kw)
    if texture and width > 0:
        stripes(ax, left, y - height / 2, width, height, tcol, kind=texture)
    return bars


class _StripeHandler:
    """Legend handler: a filled box with the same texture (stripes or dots) as the plot."""

    def __init__(self, face, color, kind="stripes"):
        self.face, self.color, self.kind = face, color, kind

    def legend_artist(self, legend, orig_handle, fontsize, handlebox):
        from matplotlib.collections import LineCollection
        from matplotlib.patches import Circle, Rectangle
        x0, y0 = handlebox.xdescent, handlebox.ydescent
        w, h = handlebox.width, handlebox.height
        tr = handlebox.get_transform()
        r = Rectangle((-x0, -y0), w, h, facecolor=self.face, edgecolor="none", transform=tr)
        handlebox.add_artist(r)
        if self.kind == "dots":
            for cx, cy in _dot_centres(-x0, -y0, -x0 + w, -y0 + h, DOT_PITCH):
                c = Circle((cx, cy), DOT_D / 2, facecolor=self.color, edgecolor="none",
                           transform=tr)
                c.set_clip_path(r)
                handlebox.add_artist(c)
        else:
            segs = _stripe_lines(-x0, -y0, -x0 + w, -y0 + h, STRIPE_PITCH)
            lc = LineCollection(segs, colors=self.color, linewidths=STRIPE_LW, transform=tr)
            lc.set_clip_path(r)
            handlebox.add_artist(lc)
        return r


def legend_handles(keys=ORDER):
    """Handles and a handler_map for ax.legend(handles=..., handler_map=...)."""
    from matplotlib.patches import Patch
    hs, hmap = [], {}
    for k in keys:
        label, face, texture, tcol, _ = OUTCOMES[k]
        p = Patch(facecolor=face, linewidth=0, label=label)
        if texture:
            hmap[p] = _StripeHandler(face, tcol, texture)
        hs.append(p)
    return hs, hmap


def report_kip_before():
    """'Before (nodes / time)' column of the 16-instance table in bobilib/REPORT.md.

    Returns name -> (nodes, seconds) or name -> text when there is no result.
    """
    out = {}
    pat = re.compile(r"^\s*\|\s*(interKP-\d+-100-\d-\d)\s*\|[^|]*\|[^|]*\|[^|]*\|\s*([^|]+?)\s*\|\s*$")
    for line in REPORT.read_text().splitlines():
        m = pat.match(line)
        if not m:
            continue
        name, before = m.groups()
        mm = re.match(r"^([\d,]+)\s*/\s*([\d.]+)\s*s$", before)
        out[name] = (int(mm.group(1).replace(",", "")), float(mm.group(2))) if mm else before
    return out
