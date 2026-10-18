#!/usr/bin/env python3
# Reviewer's check of the follower semantics used by fire_check: on random small graphs,
# max sum a over every feasible (a, x) of the inter-fire follower ILP equals
# max over |X| <= Lambda of n - |closure of Y under spreading to unprotected neighbours|.
# Usage: python3 -I closure_vs_ilp.py
import random, itertools
random.seed(5)
def closure(n, adj, Y, X):
    B = set(Y); fr = list(Y)
    while fr:
        u = fr.pop()
        for w in adj[u]:
            if w not in B and w not in X:
                B.add(w); fr.append(w)
    return n - len(B)
bad = 0
for t in range(400):
    n = random.randint(1, 6)
    E = [(u, v) for u in range(n) for v in range(u+1, n) if random.random() < 0.4]
    adj = [[] for _ in range(n)]
    for u, v in E: adj[u].append(v); adj[v].append(u)
    Lam = random.randint(0, 3)
    Y = [v for v in range(n) if random.random() < 0.3]
    y = [1 if v in Y else 0 for v in range(n)]
    best = -1
    for a in itertools.product((0,1), repeat=n):
        if any(a[v] + y[v] > 1 for v in range(n)): continue
        for x in itertools.product((0,1), repeat=n):
            if sum(x) > Lam: continue
            if any(a[q] > a[p] + x[q] for (u, v) in E for (p, q) in ((u, v), (v, u))): continue
            best = max(best, sum(a))
    sem = max(closure(n, adj, Y, set(X)) for k in range(Lam+1) for X in itertools.combinations(range(n), k))
    if sem != best: bad += 1; print('BAD', n, E, Y, Lam, best, sem)
print('trials 400 bad', bad)
