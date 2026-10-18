#!/usr/bin/env python3
# Reviewer's check of a BOBILib-format solution file against a graph file from
# parse_fire_model.py: binary values, |Y| <= Phi, every Lambda/V/A row, stated
# objective == sum a. Prints the attack so that `fire_check eval` can re-solve the
# follower by brute force. Missing entries (sparse files) count as 0.
# Usage: python3 -I check_solution.py G.g X.res.json
import json, sys

def main(gpath, solpath):
    with open(gpath) as f:
        n, m, Phi, Lam = map(int, f.readline().split())
        lab = list(map(int, f.readline().split()))
        edges = [tuple(map(int, f.readline().split())) for _ in range(m)]
    s = json.load(open(solpath))
    U = s['upper_level_decisions']; L = s['lower_level_decisions']
    assert set(U) <= {f'y_{v}' for v in lab}, 'upper keys'
    assert set(L) <= {f'a_{v}' for v in lab} | {f'x_{v}' for v in lab}, 'lower keys'
    sparse = len(U) < n or len(L) < 2 * n
    U = {f'y_{v}': U.get(f'y_{v}', 0.0) for v in lab}
    L = {k: L.get(k, 0.0) for v in lab for k in (f'a_{v}', f'x_{v}')}
    for d in (U, L):
        for k, v in d.items():
            assert v in (0.0, 1.0), (k, v)
    y = [int(U[f'y_{v}']) for v in lab]
    a = [int(L[f'a_{v}']) for v in lab]
    x = [int(L[f'x_{v}']) for v in lab]
    ok = True
    if sum(y) > Phi: ok = False; print('Phi violated')
    if sum(x) > Lam: ok = False; print('Lambda violated')
    for i in range(n):
        if a[i] + y[i] > 1: ok = False; print('V row violated', lab[i])
    for (u, v) in edges:
        for (p, q) in ((u, v), (v, u)):
            if a[q] > a[p] + x[q]: ok = False; print('arc violated', lab[p], lab[q])
    val = sum(a)
    if s.get('objective_value') is not None and abs(s['objective_value'] - val) > 1e-9:
        ok = False; print('objective mismatch', s['objective_value'], val)
    att = ','.join(str(lab[i]) for i in range(n) if y[i])
    prot = ','.join(str(lab[i]) for i in range(n) if x[i])
    print(f"{'sparse ' if sparse else ''}{s['name']} status={s.get('status')} obj={s.get('objective_value')} sum_a={val} feasible={ok} attack={att} protect={prot}")

if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
