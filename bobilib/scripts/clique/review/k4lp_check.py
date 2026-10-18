"""Count K4s of a graph and solve the LP min sum d_e s.t. sum_{e in Q} d_e >= 1 for each K4 Q."""
import itertools
import json
import os
import sys
import time

import highspy
import numpy as np

GD, name = sys.argv[1], sys.argv[2]
with open(os.path.join(GD, name + '.edges')) as f:
    n, m, k = map(int, f.readline().split())
    E = [tuple(map(int, l.split())) for l in f]
eid = {e: i for i, e in enumerate(E)}
adj = [set() for _ in range(n)]
for u, v in E:
    adj[u].add(v)
    adj[v].add(u)
K4 = []
for a in range(n):
    for b in sorted(x for x in adj[a] if x > a):
        cab = adj[a] & adj[b]
        for c in sorted(x for x in cab if x > b):
            for d in sorted(x for x in cab & adj[c] if x > c):
                K4.append((a, b, c, d))
h = highspy.Highs()
h.setOptionValue('output_flag', False)
h.setOptionValue('threads', 1)
h.addVars(m, np.zeros(m), np.ones(m))
h.changeColsCost(m, np.arange(m, dtype=np.int32), np.ones(m))
starts, idx = [], []
for q in K4:
    starts.append(len(idx))
    idx.extend(eid[p] for p in itertools.combinations(q, 2))
R = len(K4)
h.addRows(R, np.ones(R), np.full(R, highspy.kHighsInf), len(idx), np.array(starts, dtype=np.int32),
          np.array(idx, dtype=np.int32), np.ones(len(idx)))
t0 = time.time()
h.run()
print(json.dumps(dict(name=name, k4=len(K4), status=h.modelStatusToString(h.getModelStatus()),
                      lp=h.getInfo().objective_function_value, sec=round(time.time() - t0, 1))))
