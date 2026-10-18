"""Reviewer's lower-bound checks.

usage: mylb.py GRAPHDIR MODE [names...]
MODE = arith : exact integer Mantel / Brouwer / size-vector enumeration for all instances
MODE = highs : HiGHS MIP feasibility per (instance, size vector), 1 thread
"""
import itertools
import json
import os
import random
import sys
import time

GD = sys.argv[1]
MODE = sys.argv[2]
names = sys.argv[3:]


def C2(a):
    return a * (a - 1) // 2


def load(name):
    with open(os.path.join(GD, name + '.edges')) as f:
        n, m, k = map(int, f.readline().split())
        E = [tuple(map(int, l.split())) for l in f]
    assert len(E) == m
    return n, m, k, E


def turan(n, r):
    # max edges of a complete r-partite graph on n vertices, by brute force over size vectors
    best = 0
    for s in itertools.product(range(n + 1), repeat=r - 1):
        last = n - sum(s)
        if last < 0:
            continue
        sizes = list(s) + [last]
        best = max(best, C2(n) - sum(C2(a) for a in sizes))
    return best


def size_vectors(n, k, nbar):
    out = []
    for s1 in range(n + 1):
        for s2 in range(s1 + 1):
            s3 = n - s1 - s2
            if s3 < 0 or s3 > s2:
                continue
            b = k + nbar - (C2(s1) + C2(s2) + C2(s3))
            if b >= 0:
                out.append(((s1, s2, s3), b))
    return out


if MODE == 'arith':
    if not names:
        names = sorted(f[:-6] for f in os.listdir(GD) if f.endswith('.edges'))
    for name in names:
        n, m, k, E = load(name)
        Es = set(E)
        nbar = C2(n) - m
        Ebar = [(u, v) for u in range(n) for v in range(u + 1, n) if (u, v) not in Es]
        assert len(Ebar) == nbar
        rem = m - k
        t2 = turan(n, 2)
        t3 = turan(n, 3)
        assert t2 == n * n // 4
        brouwer_thr = t3 - n // 3 + 2
        rec = dict(name=name, n=n, m=m, k=k, rem=rem, floor_n2_4=n * n // 4, mantel=rem > n * n // 4,
                   t3=t3, brouwer_thr=brouwer_thr, n_gt_6=n > 6, brouwer=(n > 6 and rem >= brouwer_thr),
                   brouwer_margin=rem - brouwer_thr)
        if rec['brouwer']:
            sv = size_vectors(n, k, nbar)
            rec['size_vectors'] = [[list(s), b] for s, b in sv]
            # identity check on random partitions: inside_G = sum C2(s_i) - |Ebar| + cut_Gbar
            rng = random.Random(12345)
            bad = 0
            for _ in range(300):
                part = [rng.randrange(3) for _ in range(n)]
                if rng.random() < 0.3:
                    part = [min(p, rng.randrange(2) + 1) for p in part]
                sizes = [part.count(i) for i in range(3)]
                inside = sum(1 for u, v in E if part[u] == part[v])
                cutb = sum(1 for u, v in Ebar if part[u] != part[v])
                if inside != sum(C2(a) for a in sizes) - nbar + cutb:
                    bad += 1
                # and: if inside <= k then the sorted sizes must be one of the listed vectors
                if inside <= k and tuple(sorted(sizes, reverse=True)) not in [s for s, _ in sv]:
                    bad += 1
            rec['identity_failures'] = bad
        print(json.dumps(rec), flush=True)

elif MODE in ('highs', 'highs0'):
    import highspy
    import numpy as np
    for name in names:
        n, m, k, E = load(name)
        Es = set(E)
        nbar = C2(n) - m
        Ebar = [(u, v) for u in range(n) for v in range(u + 1, n) if (u, v) not in Es]
        for s, b in sorted(size_vectors(n, k, nbar), key=lambda t: -t[1])[:int(os.environ.get('MAXSV', '99'))]:
            h = highspy.Highs()
            h.setOptionValue('output_flag', False)
            h.setOptionValue('threads', 1)
            h.setOptionValue('time_limit', 900.0)
            h.setOptionValue('mip_rel_gap', 0.0)
            h.setOptionValue('random_seed', 7)
            nx = 3 * n
            nz = len(Ebar)
            ncol = nx + nz
            inf = highspy.kHighsInf
            cost = np.array([0.0] * nx + [0.0 if MODE == 'highs0' else 1.0] * nz)
            lb = np.zeros(ncol)
            ub = np.ones(ncol)
            h.addVars(ncol, lb, ub)
            h.changeColsCost(ncol, np.arange(ncol, dtype=np.int32), cost)
            h.changeColsIntegrality(ncol, np.arange(ncol, dtype=np.int32),
                                    np.array([highspy.HighsVarType.kInteger] * ncol))
            rows_lo, rows_hi, starts, idx, val = [], [], [], [], []

            def addrow(lo, hi, cols, coefs):
                rows_lo.append(lo)
                rows_hi.append(hi)
                starts.append(len(idx))
                idx.extend(cols)
                val.extend(coefs)
            for v in range(n):
                addrow(1, 1, [3 * v + i for i in range(3)], [1.0] * 3)
            for i in range(3):
                addrow(s[i], s[i], [3 * v + i for v in range(n)], [1.0] * n)
            for e, (u, v) in enumerate(Ebar):
                z = nx + e
                for i in range(3):
                    addrow(0, inf, [z, 3 * u + i, 3 * v + i], [1.0, -1.0, 1.0])
                    addrow(0, inf, [z, 3 * u + i, 3 * v + i], [1.0, 1.0, -1.0])
            addrow(-inf, b, list(range(nx, ncol)), [1.0] * nz)
            h.addRows(len(rows_lo), np.array(rows_lo, dtype=float), np.array(rows_hi, dtype=float),
                      len(idx), np.array(starts, dtype=np.int32), np.array(idx, dtype=np.int32),
                      np.array(val, dtype=float))
            t0 = time.time()
            h.run()
            st = h.modelStatusToString(h.getModelStatus())
            rec = dict(name=name, sizes=list(s), budget=b, nrows=len(rows_lo), ncols=ncol,
                       status=st, sec=round(time.time() - t0, 2))
            if st == 'Optimal':
                sol = h.getSolution().col_value
                part = [max(range(3), key=lambda i: sol[3 * v + i]) for v in range(n)]
                rec['inside'] = sum(1 for u, v in E if part[u] == part[v])
                rec['sizes_found'] = [part.count(i) for i in range(3)]
                rec['inside_le_k'] = rec['inside'] <= k
            print(json.dumps(rec), flush=True)
