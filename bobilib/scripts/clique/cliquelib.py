"""Shared helpers for the BOBILib edge clique interdiction set ('clique').

Everything here is written from the instance files alone: a generic MPS/AUX
reader, a structural check that the follower really is "maximum clique after
the leader deletes edges", exact clique and Turan arithmetic, and a writer
for BOBILib's solution format. Nothing is taken from earlier runs.
"""
import gzip
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
BOBILIB = os.path.dirname(os.path.dirname(HERE))
DATA = os.path.join(BOBILIB, "data")
SET_DIR = os.path.join(DATA, "collection/interdiction/clique-interdiction")
SOL_DIR = os.path.join(DATA, "collection-solutions/interdiction-solutions/clique-interdiction-solutions")
RESULTS = os.path.join(BOBILIB, "results/clique")
META = os.environ.get(
    "BOBILIB_META",
    "/tmp/claude-0/-home-user-flyover-country/954d7e75-24ef-5fff-8cf4-9c9d3794ab78/scratchpad/dl/collection.json",
)


def _open(path):
    return gzip.open(path, "rt", encoding="latin-1") if path.endswith(".gz") else open(path, encoding="latin-1")


def read_mps(path):
    """Free-format MPS. Returns (vars, rows, obj_name); vars[name] = dict(lb, ub, int, obj, coefs{row: a}),
    rows[name] = dict(sense, rhs, range). Only what BOBILib uses; anything else raises."""
    vars_, rows, order = {}, {}, []
    obj = None
    sec = None
    integer = False
    with _open(path) as fh:
        for line in fh:
            if line.startswith("*") or not line.strip():
                continue
            t = line.split()
            if not line[0].isspace():
                sec = t[0].upper()
                if sec not in ("NAME", "ROWS", "COLUMNS", "RHS", "RANGES", "BOUNDS", "ENDATA"):
                    raise ValueError(f"{path}: unsupported section {sec}")
                continue
            if sec == "ROWS":
                if t[0] == "N":
                    if obj is not None:
                        raise ValueError("two objective rows")
                    obj = t[1]
                else:
                    rows[t[1]] = dict(sense=t[0], rhs=0.0, range=None)
            elif sec == "COLUMNS":
                if len(t) >= 3 and "MARKER" in t[1]:
                    integer = "INTORG" in t[2]
                    continue
                v = vars_.get(t[0])
                if v is None:
                    v = vars_[t[0]] = dict(lb=0.0, ub=float("inf"), int=integer, obj=0.0, coefs={})
                    order.append(t[0])
                for r, a in zip(t[1::2], t[2::2]):
                    a = float(a)
                    if r == obj:
                        v["obj"] += a
                    elif r in rows:
                        v["coefs"][r] = v["coefs"].get(r, 0.0) + a
                    else:
                        raise ValueError(f"unknown row {r}")
            elif sec == "RHS":
                rest = t[1:] if len(t) % 2 == 1 else t
                for r, b in zip(rest[0::2], rest[1::2]):
                    if r == obj:
                        raise ValueError("objective constant present")
                    rows[r]["rhs"] = float(b)
            elif sec == "RANGES":
                raise ValueError("RANGES present")
            elif sec == "BOUNDS":
                ty, col = t[0], t[2]
                v = vars_[col]
                if ty == "BV":
                    v.update(lb=0.0, ub=1.0, int=True)
                elif ty == "UP":
                    v["ub"] = float(t[3])
                elif ty == "LO":
                    v["lb"] = float(t[3])
                elif ty == "FX":
                    v["lb"] = v["ub"] = float(t[3])
                else:
                    raise ValueError(f"bound type {ty}")
    return vars_, rows, obj, order


def read_aux(path):
    """Returns (follower vars {name: lower-level objective coefficient}, follower rows set, header dict)."""
    lv, lr, head = {}, set(), {}
    block = None
    with open(path, encoding="latin-1") as fh:
        for line in fh:
            t = line.split()
            if not t:
                continue
            if t[0].startswith("@"):
                block = t[0]
                continue
            if block == "@VARSBEGIN":
                lv[t[0]] = float(t[1])
            elif block == "@CONSTRSBEGIN":
                lr.add(t[0])
            elif block in ("@NUMVARS", "@NUMCONSTRS", "@NAME", "@MPS"):
                head[block] = t[0]
    return lv, lr, head


PAIR = re.compile(r"^([A-Za-z]+)(\d+),(\d+)$")
SINGLE = re.compile(r"^([A-Za-z]+)(\d+)$")
TRIPLE = re.compile(r"^([A-Za-z]+)(\d+),(\d+),(\d+)$")


class Instance:
    """An edge clique interdiction instance after its structure has been checked row by row."""

    def __init__(self, mps_path):
        self.path = mps_path
        self.name = os.path.basename(mps_path).replace(".mps.gz", "").replace(".mps", "")
        aux = mps_path.replace(".mps.gz", ".aux").replace(".mps", ".aux")
        self.vars, self.rows, self.obj_row, self.order = read_mps(mps_path)
        self.lvars, self.lrows, self.aux_head = read_aux(aux)
        self.notes = []
        self._check()

    def _check(self):
        V, R = self.vars, self.rows
        assert int(self.aux_head["@NUMVARS"]) == len(self.lvars)
        assert int(self.aux_head["@NUMCONSTRS"]) == len(self.lrows)
        assert set(self.lvars) <= set(V) and self.lrows <= set(R)
        xs, ys, ws = {}, {}, {}
        for name in V:
            m1, m2 = PAIR.match(name), SINGLE.match(name)
            if m2 and m2.group(1) == "x":
                xs[int(m2.group(2))] = name
            elif m1 and m1.group(1) in ("y", "w"):
                u, v = int(m1.group(2)), int(m1.group(3))
                assert u < v, name
                (ys if m1.group(1) == "y" else ws)[(u, v)] = name
            else:
                raise AssertionError(f"unexpected variable {name}")
        n = len(xs)
        assert sorted(xs) == list(range(n)), "vertex indices not 0..n-1"
        assert set(ys) == set(ws), "edge variables of the two levels differ"
        for name, v in V.items():
            assert v["int"] and v["lb"] == 0.0 and v["ub"] == 1.0, f"{name} not binary"
        # Levels: x and y are the follower's, w the leader's.
        for name in V:
            assert (name in self.lvars) == (name[0] in "xy"), f"level of {name}"
        # Objectives: leader minimises sum x, follower minimises -sum x (so maximises the clique).
        for name, v in V.items():
            want = 1.0 if name[0] == "x" else 0.0
            assert v["obj"] == want, f"upper objective coefficient of {name} is {v['obj']}"
            if name in self.lvars:
                assert self.lvars[name] == -want, f"lower objective coefficient of {name}"
        edges = sorted(ys)
        eset = set(edges)
        # Row patterns, checked coefficient by coefficient.
        by_row = {r: {} for r in R}
        for name, v in V.items():
            for r, a in v["coefs"].items():
                by_row[r][name] = a
        k = None
        nonedges = set()
        seenE, seenI = set(), set()
        triples = []
        for r, row in R.items():
            c = by_row[r]
            m3 = TRIPLE.match(r)
            if m3:
                # Lifted edge row x_u + x_v + x_w - y_uv <= 1 ('plusbcpins'); checked below.
                assert m3.group(1) == "S" and r in self.lrows and row["sense"] == "L" and row["rhs"] == 1.0, r
                u, v, w = (int(m3.group(i)) for i in (2, 3, 4))
                assert c == {xs[u]: 1.0, xs[v]: 1.0, xs[w]: 1.0, ys[(min(u, v), max(u, v))]: -1.0}, r
                triples.append((u, v, w))
                continue
            if r == "KL":
                assert r not in self.lrows and row["sense"] == "L"
                assert set(c) == set(ws.values()) and all(a == 1.0 for a in c.values()), "budget row"
                k = row["rhs"]
                assert k == int(k)
                k = int(k)
                continue
            m1 = PAIR.match(r)
            assert m1, f"unexpected row {r}"
            kind, u, v = m1.group(1), int(m1.group(2)), int(m1.group(3))
            assert u < v and r in self.lrows and row["sense"] == "L", r
            if kind == "C":  # non-edge: x_u + x_v <= 1
                assert (u, v) not in eset
                assert c == {xs[u]: 1.0, xs[v]: 1.0} and row["rhs"] == 1.0, r
                nonedges.add((u, v))
            elif kind == "E":  # edge: x_u + x_v - y_uv <= 1
                assert c == {xs[u]: 1.0, xs[v]: 1.0, ys[(u, v)]: -1.0} and row["rhs"] == 1.0, r
                seenE.add((u, v))
            elif kind == "I":  # interdiction: w_uv + y_uv <= 1
                assert c == {ws[(u, v)]: 1.0, ys[(u, v)]: 1.0} and row["rhs"] == 1.0, r
                seenI.add((u, v))
            else:
                raise AssertionError(f"unexpected row kind {r}")
        assert k is not None
        assert seenE == eset and seenI == eset, "every edge needs one E and one I row"
        allpairs = {(u, v) for u in range(n) for v in range(u + 1, n)}
        assert nonedges | eset == allpairs and not (nonedges & eset), "C rows are not exactly the non-edges"
        # A lifted row changes nothing for 0/1 points exactly when w is adjacent to neither u nor v:
        # then x_w = 1 forces x_u = x_v = 0 and otherwise the row is the E row of uv. If w were
        # adjacent to one of them, a follower clique {v, w} would need edge uv kept: a different game.
        for (u, v, w) in triples:
            pu, pv = (min(u, w), max(u, w)), (min(v, w), max(v, w))
            assert pu in nonedges and pv in nonedges, f"lifted row S{u},{v},{w} is not redundant"
        if triples:
            self.notes.append(f"{len(triples)} lifted rows S_uvw, all redundant for 0/1 points")
        self.n, self.edges, self.k = n, edges, k
        self.nonedges = sorted(nonedges)
        self.xs, self.ys, self.ws = xs, ys, ws
        self.m = len(edges)

    def adjacency(self, removed=()):
        """Bitset adjacency of G minus the given edges."""
        rem = set(removed)
        A = [0] * self.n
        for (u, v) in self.edges:
            if (u, v) not in rem:
                A[u] |= 1 << v
                A[v] |= 1 << u
        return A

    def max_violation(self, val):
        """Largest violation of bounds and rows for a full assignment {var: value}."""
        worst, what = 0.0, ""
        for name, v in self.vars.items():
            x = val.get(name, 0.0)
            for amt, why in ((v["lb"] - x, "lb"), (x - v["ub"], "ub"), (abs(x - round(x)), "int")):
                if amt > worst:
                    worst, what = amt, f"{why} {name}"
        act = {r: 0.0 for r in self.rows}
        for name, v in self.vars.items():
            x = val.get(name, 0.0)
            if x:
                for r, a in v["coefs"].items():
                    act[r] += a * x
        for r, row in self.rows.items():
            s, b = row["sense"], row["rhs"]
            amt = act[r] - b if s == "L" else (b - act[r] if s == "G" else abs(act[r] - b))
            if amt > worst:
                worst, what = amt, f"row {r}"
        return worst, what


class BcpInstance(Instance):
    """The 'bcpins' formulation: no follower edge variables. Rows are the budget (sum w <= k),
    w_uv + x_u + x_v <= 2 for each edge and x_u + x_v <= 1 for each non-edge, with generic names."""

    def _check(self):
        V, R = self.vars, self.rows
        assert int(self.aux_head["@NUMVARS"]) == len(self.lvars)
        assert int(self.aux_head["@NUMCONSTRS"]) == len(self.lrows)
        xs, ws = {}, {}
        for name, v in V.items():
            assert v["int"] and v["lb"] == 0.0 and v["ub"] == 1.0, f"{name} not binary"
            m2 = SINGLE.match(name)
            m1 = re.match(r"^w(\d+)_(\d+)$", name)
            if m2 and m2.group(1) == "x":
                xs[int(m2.group(2))] = name
                assert name in self.lvars and self.lvars[name] == -1.0 and v["obj"] == 1.0
            elif m1:
                u, w = int(m1.group(1)), int(m1.group(2))
                assert u < w and name not in self.lvars and v["obj"] == 0.0
                ws[(u, w)] = name
            else:
                raise AssertionError(f"unexpected variable {name}")
        n = len(xs)
        assert sorted(xs) == list(range(n))
        xi = {name: i for i, name in xs.items()}
        wi = {name: e for e, name in ws.items()}
        by_row = {r: {} for r in R}
        for name, v in V.items():
            for r, a in v["coefs"].items():
                by_row[r][name] = a
        k, edges, nonedges = None, set(), set()
        for r, row in R.items():
            c = by_row[r]
            assert row["sense"] == "L" and all(a == 1.0 for a in c.values()), r
            if r not in self.lrows:
                assert k is None and set(c) == set(ws.values()), "budget row"
                k = int(row["rhs"])
                assert k == row["rhs"]
                continue
            vx = sorted(xi[t] for t in c if t in xi)
            vw = [wi[t] for t in c if t in wi]
            assert len(vx) == 2, r
            if not vw:
                assert row["rhs"] == 1.0
                nonedges.add(tuple(vx))
            else:
                assert len(vw) == 1 and vw[0] == tuple(vx) and row["rhs"] == 2.0, r
                edges.add(vw[0])
        assert edges == set(ws), "every leader edge needs its follower row"
        allpairs = {(u, v) for u in range(n) for v in range(u + 1, n)}
        assert edges | nonedges == allpairs and not (edges & nonedges)
        self.n, self.k = n, k
        self.edges, self.nonedges = sorted(edges), sorted(nonedges)
        self.xs, self.ws, self.ys = xs, ws, {}
        self.m = len(self.edges)


def load(path):
    """Pick the formulation by its variable names."""
    with _open(path) as fh:
        head = fh.read(200000)
    return BcpInstance(path) if re.search(r"\bw\d+_\d+\b", head) else Instance(path)


def instance_paths(sub="clique"):
    d = os.path.join(SET_DIR, sub)
    return sorted(os.path.join(d, f) for f in os.listdir(d) if f.endswith(".mps.gz"))


def sort_key(name):
    """normalClique40-70-10 sorts after -9."""
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", name)]


def turan(n, r):
    """Edges of the balanced complete r-partite graph T_r(n)."""
    q, s = divmod(n, r)
    sizes = [q + 1] * s + [q] * (r - s)
    return n * (n - 1) // 2 - sum(a * (a - 1) // 2 for a in sizes)


def popcount(x):
    return bin(x).count("1")


def max_clique(A):
    """Exact maximum clique by branch and bound with greedy colouring bounds (Tomita-style).
    A is a list of neighbour bitsets. Returns a list of vertices."""
    n = len(A)
    best = []

    def colour_order(P):
        out = []
        c = 0
        while P:
            c += 1
            Q = P
            while Q:
                v = (Q & -Q).bit_length() - 1
                Q &= ~(1 << v)
                Q &= ~A[v]
                P &= ~(1 << v)
                out.append((v, c))
        return out

    def expand(C, P):
        nonlocal best
        order = colour_order(P)
        for v, c in reversed(order):
            if len(C) + c <= len(best):
                return
            Pv = P & A[v]
            if Pv:
                expand(C + [v], Pv)
            elif len(C) + 1 > len(best):
                best = C + [v]
            P &= ~(1 << v)

    expand([], (1 << n) - 1)
    return best


def has_clique(A, q, P=None, size=0):
    """Plain exhaustive test for a clique on q vertices (independent of max_clique)."""
    if P is None:
        P = (1 << len(A)) - 1
    if size == q:
        return True
    while P:
        if popcount(P) + size < q:
            return False
        v = (P & -P).bit_length() - 1
        P &= ~(1 << v)
        if has_clique(A, q, P & A[v], size + 1):
            return True
    return False


def is_clique(A, C):
    return all(A[u] >> v & 1 for i, u in enumerate(C) for v in C[i + 1:])


def inside_edges(inst, part):
    return [(u, v) for (u, v) in inst.edges if part[u] == part[v]]


def solution_values(inst, removed, clique):
    """Full assignment for a leader choice (edges removed) and a follower clique."""
    rem = set(removed)
    C = set(clique)
    val = {}
    for e, name in inst.ws.items():
        val[name] = 1.0 if e in rem else 0.0
    for i, name in inst.xs.items():
        val[name] = 1.0 if i in C else 0.0
    for (u, v), name in inst.ys.items():
        # y_uv = 1 exactly for kept edges inside the clique; any kept edge could be 1.
        val[name] = 1.0 if (u in C and v in C and (u, v) not in rem) else 0.0
    return val


def write_solution(inst, removed, clique, path, status="feasible"):
    val = solution_values(inst, removed, clique)
    up = {inst.ws[e]: val[inst.ws[e]] for e in inst.edges}
    lo = {inst.xs[i]: val[inst.xs[i]] for i in range(inst.n)}
    lo.update({inst.ys[e]: val[inst.ys[e]] for e in inst.edges})
    obj = sum(val[inst.xs[i]] for i in range(inst.n))
    doc = dict(name=inst.name, bilevel_type="optimistic", status=status, objective_value=obj,
               upper_level_decisions=up, lower_level_decisions=lo)
    with open(path, "w") as fh:
        json.dump(doc, fh, indent=1)
    return val, obj


def load_meta():
    with open(META) as fh:
        d = json.load(fh)
    return {v["Instance"]: v for v in d.values()}
