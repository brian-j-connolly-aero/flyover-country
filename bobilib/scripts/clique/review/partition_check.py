"""Is each solution's deletion set D exactly the set of inside edges of some r-partition
(r = the solution's objective)? Contract the components of (V, D), require no kept edge
inside a component, r-colour the quotient of G - D, and compare inside_G(colouring) with D.

usage: partition_check.py GRAPHDIR SOLUTION_FILES...
"""
import json
import os
import sys

sys.setrecursionlimit(10000)
GD = sys.argv[1]
for p in sys.argv[2:]:
    s = json.load(open(p))
    nm = s['name']
    with open(os.path.join(GD, nm + '.edges')) as f:
        n, m, k = map(int, f.readline().split())
        E = [tuple(map(int, l.split())) for l in f if l.strip()]
    D = set(e for e in E if s['upper_level_decisions']['w%d,%d' % e] == 1.0)
    r = int(s['objective_value'])
    par = list(range(n))

    def find(x):
        while par[x] != x:
            par[x] = par[par[x]]
            x = par[x]
        return x
    for u, v in D:
        par[find(u)] = find(v)
    comp = [find(v) for v in range(n)]
    loops = sum(1 for e in E if e not in D and comp[e[0]] == comp[e[1]])
    reps = sorted(set(comp))
    idx = {c: i for i, c in enumerate(reps)}
    q = len(reps)
    adj = [set() for _ in range(q)]
    for e in E:
        if e in D:
            continue
        a, b = idx[comp[e[0]]], idx[comp[e[1]]]
        if a != b:
            adj[a].add(b)
            adj[b].add(a)
    col = [-1] * q
    order = sorted(range(q), key=lambda x: -len(adj[x]))

    def rec(i):
        if i == q:
            return True
        v = order[i]
        used = {col[u] for u in adj[v] if col[u] >= 0}
        for c in range(min(r, max(col) + 2)):
            if c not in used:
                col[v] = c
                if rec(i + 1):
                    return True
                col[v] = -1
        return False
    ok = loops == 0 and rec(0)
    sizes = None
    if ok:
        part = [col[idx[comp[v]]] for v in range(n)]
        ok = set(e for e in E if part[e[0]] == part[e[1]]) == D
        sizes = sorted([part.count(i) for i in range(r)], reverse=True)
    print(json.dumps(dict(name=nm, r=r, deleted=len(D), exact_inside_edges_of_partition=ok,
                          part_sizes=sizes)), flush=True)
