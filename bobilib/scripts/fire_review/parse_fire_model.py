#!/usr/bin/env python3
# Reviewer's parser for BOBILib inter-fire (written before reading src/fire.rs).
# Reads an MPS/AUX pair, asserts the attack-protect pattern exactly (every column, row,
# coefficient, right-hand side, bound and the AUX level split) and writes "<name>.g":
#   line 1: n m Phi Lambda; line 2: node labels (sorted); then m edges as node indices.
# Usage: python3 -I parse_fire_model.py X.mps.gz X.aux OUTDIR
import gzip, sys, os, re

def parse_mps(path):
    rows = {}  # name -> sense
    rowsorder = []
    cols = {}  # col -> {row: coef}
    colorder = []
    rhs = {}
    bounds = {}
    integer = set()
    sec = None
    intmode = False
    objconst = None
    with gzip.open(path, 'rt', encoding='latin-1') as f:
        for line in f:
            if line.startswith('*') or not line.strip():
                continue
            if not line.startswith(' '):
                tok = line.split()
                sec = tok[0]
                if sec in ('OBJSENSE', 'OBJSENSE MAX', 'OBJSENSE MAXIMIZE'):
                    raise SystemExit('OBJSENSE present in ' + path)
                continue
            tok = line.split()
            if sec == 'ROWS':
                rows[tok[1]] = tok[0]
                rowsorder.append(tok[1])
            elif sec == 'COLUMNS':
                if len(tok) >= 3 and tok[1] == "'MARKER'":
                    if tok[2] == "'INTORG'":
                        intmode = True
                    elif tok[2] == "'INTEND'":
                        intmode = False
                    else:
                        raise SystemExit('bad marker')
                    continue
                c = tok[0]
                if c not in cols:
                    cols[c] = {}
                    colorder.append(c)
                if intmode:
                    integer.add(c)
                pairs = tok[1:]
                assert len(pairs) in (2, 4), line
                for i in range(0, len(pairs), 2):
                    r, v = pairs[i], float(pairs[i + 1])
                    assert r in rows, r
                    assert r not in cols[c], (c, r)
                    cols[c][r] = v
            elif sec == 'RHS':
                pairs = tok[1:]
                for i in range(0, len(pairs), 2):
                    rhs[pairs[i]] = float(pairs[i + 1])
            elif sec == 'BOUNDS':
                bounds.setdefault(tok[2], []).append((tok[0], tok[3] if len(tok) > 3 else None))
            elif sec == 'RANGES':
                raise SystemExit('RANGES present')
            elif sec == 'ENDATA':
                pass
            else:
                raise SystemExit('unknown section ' + str(sec))
    return rows, rowsorder, cols, colorder, rhs, bounds, integer


def parse_aux(path):
    lines = [l.strip() for l in open(path, encoding='latin-1')]
    i = 0
    out = {}
    while i < len(lines):
        l = lines[i]
        if l == '@NUMVARS':
            out['nv'] = int(lines[i + 1]); i += 2
        elif l == '@NUMCONSTRS':
            out['nc'] = int(lines[i + 1]); i += 2
        elif l == '@VARSBEGIN':
            v = {}
            i += 1
            while lines[i] != '@VARSEND':
                t = lines[i].split()
                v[t[0]] = float(t[1])
                i += 1
            out['vars'] = v
            i += 1
        elif l == '@CONSTRSBEGIN':
            c = []
            i += 1
            while lines[i] != '@CONSTRSEND':
                c.append(lines[i].split()[0])
                i += 1
            out['cons'] = c
            i += 1
        else:
            i += 1
    assert out['nv'] == len(out['vars'])
    assert out['nc'] == len(out['cons'])
    return out


def main(mps, aux, outdir):
    rows, rowsorder, cols, colorder, rhs, bounds, integer = parse_mps(mps)
    A = parse_aux(aux)
    name = os.path.basename(mps)[:-len('.mps.gz')]
    objrows = [r for r, s in rows.items() if s == 'N']
    assert objrows == ['obj'], objrows
    # every row is L
    assert all(s == 'L' for r, s in rows.items() if r != 'obj')
    # nodes
    nodes = set()
    for c in colorder:
        m = re.fullmatch(r'([axy])_(\d+)', c)
        assert m, c
        nodes.add(int(m.group(2)))
    nodes = sorted(nodes)
    for v in nodes:
        for p in 'axy':
            assert f'{p}_{v}' in cols, (p, v)
    assert len(cols) == 3 * len(nodes)
    # all binary
    for c in colorder:
        assert c in integer, c
        b = bounds.get(c)
        assert b == [('BV', None)] or (b and b[0][0] == 'BV'), (c, b)
    for c, b in bounds.items():
        assert c in cols
        assert b == [('BV', None)], (c, b)
    # AUX: follower vars are exactly a_v (obj -1) and x_v (obj 0)
    fv = A['vars']
    assert set(fv) == {f'a_{v}' for v in nodes} | {f'x_{v}' for v in nodes}, 'follower var set'
    for v in nodes:
        assert fv[f'a_{v}'] == -1.0 and fv[f'x_{v}'] == 0.0
    fc = set(A['cons'])
    assert len(fc) == len(A['cons'])
    assert 'Phi' not in fc and 'Lambda' in fc
    upper = [r for r in rows if r != 'obj' and r not in fc]
    assert upper == ['Phi'], upper
    # obj: +1 on a_v only
    for c in colorder:
        o = cols[c].get('obj', 0.0)
        if c.startswith('a_'):
            assert o == 1.0, c
        else:
            assert o == 0.0, c
    assert 'obj' not in rhs, 'objective constant'
    # Build row -> {col: coef}
    R = {}
    for c in colorder:
        for r, v in cols[c].items():
            if r == 'obj':
                continue
            R.setdefault(r, {})[c] = v
    Phi = rhs['Phi']; Lam = rhs['Lambda']
    assert R['Phi'] == {f'y_{v}': 1.0 for v in nodes}, 'Phi row'
    assert R['Lambda'] == {f'x_{v}': 1.0 for v in nodes}, 'Lambda row'
    assert Phi == int(Phi) and Lam == int(Lam)
    arcs = []
    vrows = 0
    for r in rows:
        if r in ('obj', 'Phi', 'Lambda'):
            continue
        assert r in fc, r
        m = re.fullmatch(r'V(\d+)', r)
        if m:
            v = int(m.group(1))
            assert R[r] == {f'a_{v}': 1.0, f'y_{v}': 1.0}, (r, R[r])
            assert rhs.get(r) == 1.0
            vrows += 1
            continue
        m = re.fullmatch(r'A(\d+)-(\d+)', r)
        assert m, r
        u, v = int(m.group(1)), int(m.group(2))
        assert u != v
        # a_v - a_u - x_v <= 0
        assert R[r] == {f'a_{v}': 1.0, f'a_{u}': -1.0, f'x_{v}': -1.0}, (r, R[r])
        assert rhs.get(r, 0.0) == 0.0
        arcs.append((u, v))
    assert vrows == len(nodes)
    # any rhs on unknown row?
    for r in rhs:
        assert r in rows
    aset = set(arcs)
    assert len(aset) == len(arcs)
    for (u, v) in arcs:
        assert (v, u) in aset, ('asymmetric', u, v)
    edges = sorted({(min(u, v), max(u, v)) for (u, v) in arcs})
    idx = {v: i for i, v in enumerate(nodes)}
    # forest check via union-find
    par = list(range(len(nodes)))
    def find(a):
        while par[a] != a:
            par[a] = par[par[a]]
            a = par[a]
        return a
    forest = True
    for (u, v) in edges:
        a, b = find(idx[u]), find(idx[v])
        if a == b:
            forest = False
        else:
            par[a] = b
    ncomp = len({find(i) for i in range(len(nodes))})
    with open(os.path.join(outdir, name + '.g'), 'w') as f:
        f.write(f'{len(nodes)} {len(edges)} {int(Phi)} {int(Lam)}\n')
        f.write(' '.join(map(str, nodes)) + '\n')
        for (u, v) in edges:
            f.write(f'{idx[u]} {idx[v]}\n')
    print(name, 'n', len(nodes), 'm', len(edges), 'Phi', int(Phi), 'Lambda', int(Lam), 'forest', forest, 'comps', ncomp)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2], sys.argv[3])
