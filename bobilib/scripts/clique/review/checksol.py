"""Reviewer's independent check of BOBILib clique-interdiction solution files.

For each file: every MPS column present with a 0/1 value, every row (leader and follower)
satisfied exactly, budget, x is a clique of G - D, |x| equals the stated objective, and
omega(G - D) computed exactly by two different searches (Bron-Kerbosch with Tomita pivoting,
and a Carraghan-Pardalos branch and bound) equals |x|.
Also reports whether G - D is r-colourable for r = |x| (exact DSATUR-free backtracking).
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mymps  # noqa: E402

sys.setrecursionlimit(10000)
INST = sys.argv[1]
files = sys.argv[2:]


def popcount(x):
    return bin(x).count('1')


def bk_max(adj, n):
    best = [0]

    def rec(rsize, P, X):
        if P == 0 and X == 0:
            if rsize > best[0]:
                best[0] = rsize
            return
        if rsize + popcount(P) <= best[0]:
            return
        # Tomita pivot: u in P|X maximising |P & N(u)|
        PX = P | X
        bu, bc = -1, -1
        t = PX
        while t:
            lb = t & -t
            u = lb.bit_length() - 1
            t ^= lb
            c = popcount(P & adj[u])
            if c > bc:
                bc, bu = c, u
        cand = P & ~adj[bu]
        while cand:
            lb = cand & -cand
            v = lb.bit_length() - 1
            cand ^= lb
            rec(rsize + 1, P & adj[v], X & adj[v])
            P &= ~lb
            X |= lb

    rec(0, (1 << n) - 1, 0)
    return best[0]


def cp_max(adj, n):
    best = [0]

    def rec(size, P):
        if P == 0:
            if size > best[0]:
                best[0] = size
            return
        while P:
            if size + popcount(P) <= best[0]:
                return
            lb = P & -P
            v = lb.bit_length() - 1
            P ^= lb
            rec(size + 1, P & adj[v])

    rec(0, (1 << n) - 1)
    return best[0]


def colourable(adj, n, r):
    order = sorted(range(n), key=lambda v: -popcount(adj[v]))
    col = [-1] * n

    def rec(i):
        if i == n:
            return True
        v = order[i]
        used = set(col[u] for u in range(n) if (adj[v] >> u) & 1 and col[u] >= 0)
        maxc = max(col) + 1
        for c in range(min(r, maxc + 1)):
            if c not in used:
                col[v] = c
                if rec(i + 1):
                    return True
                col[v] = -1
        return False

    return rec(0)


cache = {}
for path in files:
    sol = json.load(open(path))
    name = sol['name']
    base = os.path.basename(path)
    assert base == name + '.res.json', (base, name)
    if name not in cache:
        cache[name] = (mymps.read_mps(os.path.join(INST, name + '.mps.gz')),
                       mymps.read_aux(os.path.join(INST, name + '.aux')))
    mps, aux = cache[name]
    errs = []
    vals = {}
    vals.update(sol['upper_level_decisions'])
    vals.update(sol['lower_level_decisions'])
    if len(vals) != len(sol['upper_level_decisions']) + len(sol['lower_level_decisions']):
        errs.append('name in both levels')
    cols = mps['cols']
    lead_names = set(c for c in cols if c not in aux['vars'])
    if set(sol['upper_level_decisions']) != lead_names:
        errs.append('upper-level var set differs from MPS leader columns')
    if set(sol['lower_level_decisions']) != set(aux['vars']):
        errs.append('lower-level var set differs from AUX')
    for c in cols:
        v = vals.get(c)
        if v is None:
            errs.append('missing ' + c)
            continue
        if v not in (0.0, 1.0):
            errs.append('nonbinary %s=%r' % (c, v))
    # rows
    act = {r: 0.0 for r in mps['rows']}
    for c, d in cols.items():
        v = vals.get(c, 0.0)
        if v:
            for r, a in d.items():
                act[r] += a * v
    objrow = mps['obj']
    maxviol = 0.0
    for r, s in mps['rows'].items():
        if r == objrow:
            continue
        b = mps['rhs'].get(r, 0.0)
        a = act[r]
        if s == 'L':
            viol = a - b
        elif s == 'G':
            viol = b - a
        else:
            viol = abs(a - b)
        maxviol = max(maxviol, viol)
    if maxviol > 0:
        errs.append('row violation %g' % maxviol)
    leader_obj = act[objrow]
    foll_obj = sum(aux['vars'][c] * vals[c] for c in aux['vars'])
    # graph
    n = sum(1 for c in cols if re.fullmatch(r'x\d+', c))
    E = []
    for c in cols:
        mm = re.fullmatch(r'w(\d+),(\d+)', c)
        if mm:
            E.append((int(mm.group(1)), int(mm.group(2))))
    k = mps['rhs']['KL']
    D = [(u, v) for (u, v) in E if vals['w%d,%d' % (u, v)] == 1.0]
    if len(D) > k:
        errs.append('budget exceeded')
    Dset = set(D)
    adj = [0] * n
    for u, v in E:
        if (u, v) not in Dset:
            adj[u] |= 1 << v
            adj[v] |= 1 << u
    X = [i for i in range(n) if vals['x%d' % i] == 1.0]
    for i in range(len(X)):
        for j in range(i + 1, len(X)):
            if not (adj[X[i]] >> X[j]) & 1:
                errs.append('x not a clique of G-D (%d,%d)' % (X[i], X[j]))
    w1 = bk_max(adj, n)
    w2 = cp_max(adj, n)
    if w1 != w2:
        errs.append('my two clique searches disagree %d %d' % (w1, w2))
    if w1 != len(X):
        errs.append('x not maximum: |x|=%d omega=%d' % (len(X), w1))
    if abs(leader_obj - len(X)) > 0 or abs(sol['objective_value'] - len(X)) > 0:
        errs.append('objective mismatch leader %g file %g |x| %d' % (leader_obj, sol['objective_value'], len(X)))
    if abs(foll_obj + len(X)) > 0:
        errs.append('follower objective %g' % foll_obj)
    rcol = colourable(adj, n, w1) if not os.environ.get("NOCOL") else None
    rec = dict(name=name, n=n, m=len(E), k=k, deleted=len(D), clique=len(X), omega_bk=w1, omega_cp=w2,
               remainder_colourable_with_omega_colours=rcol, max_violation=maxviol,
               status=sol['status'], file_obj=sol['objective_value'], ok=not errs, errs=errs[:5])
    print(json.dumps(rec), flush=True)
