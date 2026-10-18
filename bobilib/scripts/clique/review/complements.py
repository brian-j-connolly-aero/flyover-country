"""Write the complement of every density-0.9 graph (GRAPHDIR/<name>.edges, from structure.py)
as OUTDIR/<name>.comp ("n m" then edges), plus planted controls OUTDIR/planted_c{1,2,3,5}.comp:
three random blobs of sizes 22, 20, 18 (p = 0.3) joined by c cross edges.

usage: complements.py GRAPHDIR OUTDIR
"""
import os
import random
import sys

GD, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
for fn in sorted(os.listdir(GD)):
    if '-90-' not in fn or not fn.endswith('.edges'):
        continue
    L = open(os.path.join(GD, fn)).read().split('\n')
    n, m, k = map(int, L[0].split())
    E = set(tuple(map(int, l.split())) for l in L[1:] if l.strip())
    assert len(E) == m
    Eb = [(u, v) for u in range(n) for v in range(u + 1, n) if (u, v) not in E]
    with open(os.path.join(OUT, fn.replace('.edges', '.comp')), 'w') as f:
        f.write('%d %d\n' % (n, len(Eb)))
        for u, v in Eb:
            f.write('%d %d\n' % (u, v))
rng = random.Random(5)
for c in (1, 2, 3, 5):
    n = 60
    lab = []
    for i, s in enumerate([22, 20, 18]):
        lab += [i] * s
    perm = list(range(n))
    rng.shuffle(perm)
    Lb = [0] * n
    for i, v in enumerate(perm):
        Lb[v] = lab[i]
    E = set()
    for u in range(n):
        for v in range(u + 1, n):
            if Lb[u] == Lb[v] and rng.random() < 0.3:
                E.add((u, v))
    cross = 0
    while cross < c:
        u, v = sorted(rng.sample(range(n), 2))
        if Lb[u] != Lb[v] and (u, v) not in E:
            E.add((u, v))
            cross += 1
    with open(os.path.join(OUT, 'planted_c%d.comp' % c), 'w') as f:
        f.write('%d %d\n' % (n, len(E)))
        for u, v in sorted(E):
            f.write('%d %d\n' % (u, v))
