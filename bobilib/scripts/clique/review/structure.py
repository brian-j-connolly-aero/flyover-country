"""Re-derive the bilevel model of each clique instance from MPS + AUX (reviewer's code).

Prints one JSON line per instance with n, m, k, the row-pattern census and any surprise.
Writes the graph (edge list) to OUTDIR/<name>.edges for later use.
"""
import collections
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mymps  # noqa: E402

D = sys.argv[1]
OUT = sys.argv[2]
os.makedirs(OUT, exist_ok=True)


def vkind(c):
    m = re.fullmatch(r'([a-z])(\d+)(?:,(\d+))?', c)
    if not m:
        return None
    return m.group(1), int(m.group(2)), (int(m.group(3)) if m.group(3) is not None else None)


for fn in sorted(os.listdir(D)):
    if not fn.endswith('.mps.gz'):
        continue
    name = fn[:-7]
    mps = mymps.read_mps(os.path.join(D, fn))
    aux = mymps.read_aux(os.path.join(D, name + '.aux'))
    surprises = []
    if mps['objsense'] != 'MIN':
        surprises.append('objsense ' + mps['objsense'])
    if mps['ranges']:
        surprises.append('ranges present')
    cols = mps['cols']
    kinds = {c: vkind(c) for c in cols}
    if any(v is None for v in kinds.values()):
        surprises.append('odd col names')
    lead = [c for c in mps['col_order'] if c not in aux['vars']]
    foll = aux['var_order']
    if aux['numvars'] != len(foll):
        surprises.append('numvars mismatch')
    if set(foll) - set(cols):
        surprises.append('aux var not in mps')
    if aux['numconstrs'] != len(aux['constrs']):
        surprises.append('numconstrs mismatch')
    # leader variables must be exactly the w's
    if set(kinds[c][0] for c in lead) != {'w'}:
        surprises.append('leader vars not all w: %s' % collections.Counter(kinds[c][0] for c in lead))
    if set(kinds[c][0] for c in foll) - {'x', 'y'}:
        surprises.append('follower vars not x/y')
    # integrality and bounds
    for c in cols:
        if c not in mps['integer']:
            surprises.append('continuous ' + c)
        if mps['bounds'][c] != [0.0, 1.0]:
            surprises.append('bound %s %s' % (c, mps['bounds'][c]))
    xs = sorted(kinds[c][1] for c in cols if kinds[c][0] == 'x')
    n = len(xs)
    if xs != list(range(n)):
        surprises.append('x indices not 0..n-1')
    E = sorted((kinds[c][1], kinds[c][2]) for c in cols if kinds[c][0] == 'w')
    Ey = sorted((kinds[c][1], kinds[c][2]) for c in cols if kinds[c][0] == 'y')
    if E != Ey:
        surprises.append('w and y edge sets differ')
    Eset = set(E)
    if any(not (0 <= u < v < n) for u, v in E):
        surprises.append('edge index order')
    m = len(E)
    # objective of the leader: coefficients from N row
    objrow = mps['obj']
    objc = collections.Counter()
    for c in cols:
        objc[(kinds[c][0], cols[c].get(objrow, 0.0))] += 1
    # follower objective from AUX
    fobj = collections.Counter((kinds[c][0], v) for c, v in aux['vars'].items())
    # build row -> {col: coef}
    R = collections.defaultdict(dict)
    for c, d in cols.items():
        for r, v in d.items():
            if r != objrow:
                R[r][c] = v
    foll_rows = set(aux['constrs'])
    if not foll_rows <= set(mps['rows']):
        surprises.append('aux constr not in mps')
    lead_rows = [r for r in mps['row_order'] if r != objrow and r not in foll_rows]
    pattern = collections.Counter()
    nonedge_rows = set()
    k = None
    for r in mps['row_order']:
        if r == objrow:
            continue
        sense = mps['rows'][r]
        b = mps['rhs'].get(r, 0.0)
        d = R[r]
        sig = tuple(sorted(collections.Counter((kinds[c][0], v) for c, v in d.items()).items()))
        side = 'L' if r in lead_rows else 'F'
        # semantic checks
        vs = sorted(d)
        kk = [kinds[c] for c in vs]
        if side == 'L':
            if sense == 'L' and all(t[0] == 'w' for t in kk) and all(v == 1.0 for v in d.values()) \
                    and len(d) == m:
                k = b
                pattern[('L budget', sense)] += 1
            else:
                pattern[('L other', sense, sig, b)] += 1
                surprises.append('extra leader row ' + r)
            continue
        if sense == 'L' and b == 1.0 and len(d) == 2 and all(t[0] == 'x' for t in kk) \
                and all(v == 1.0 for v in d.values()):
            u, v = sorted(t[1] for t in kk)
            if (u, v) in Eset:
                surprises.append('C-type row on an edge ' + r)
            nonedge_rows.add((u, v))
            pattern['F x_u+x_v<=1 (non-edge)'] += 1
        elif sense == 'L' and b == 1.0 and len(d) == 3:
            xx = [(t, d[c]) for c, t in zip(vs, kk) if t[0] == 'x']
            yy = [(t, d[c]) for c, t in zip(vs, kk) if t[0] == 'y']
            ok = (len(xx) == 2 and len(yy) == 1 and all(v == 1.0 for _, v in xx) and yy[0][1] == -1.0
                  and tuple(sorted(t[1] for t, _ in xx)) == (yy[0][0][1], yy[0][0][2]))
            if ok:
                pattern['F x_u+x_v-y_uv<=1'] += 1
            else:
                pattern[('F other3', sig, b)] += 1
                surprises.append('odd 3-row ' + r)
        elif sense == 'L' and b == 1.0 and len(d) == 2 and sorted(t[0] for t in kk) == ['w', 'y']:
            t1, t2 = kk
            if (t1[1], t1[2]) == (t2[1], t2[2]) and all(v == 1.0 for v in d.values()):
                pattern['F w_uv+y_uv<=1'] += 1
            else:
                pattern[('F otherwy', sig, b)] += 1
                surprises.append('odd wy row ' + r)
        else:
            pattern[('F other', sense, sig, b)] += 1
            surprises.append('unknown follower row ' + r)
    allpairs = {(u, v) for u in range(n) for v in range(u + 1, n)}
    if nonedge_rows != allpairs - Eset:
        surprises.append('non-edge rows != complement of E')
    # E-row and I-row count per edge
    if pattern['F x_u+x_v-y_uv<=1'] != m or pattern['F w_uv+y_uv<=1'] != m:
        surprises.append('E/I row counts != m')
    with open(os.path.join(OUT, name + '.edges'), 'w') as f:
        f.write('%d %d %d\n' % (n, m, int(k)))
        for u, v in E:
            f.write('%d %d\n' % (u, v))
    rec = dict(name=name, n=n, m=m, k=k, k_eq_floor_m_over_4=(k == m // 4),
               nleadrows=len(lead_rows), nfollrows=len(foll_rows),
               leader_obj={'%s:%g' % kv: c for kv, c in objc.items()},
               follower_obj={'%s:%g' % kv: c for kv, c in fobj.items()},
               patterns={str(p): c for p, c in pattern.items()}, objsense=mps['objsense'],
               surprises=surprises[:10], nsurprises=len(surprises))
    print(json.dumps(rec), flush=True)
