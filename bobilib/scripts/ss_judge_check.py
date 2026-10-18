#!/usr/bin/env python3
"""Independent lower-bound prover for subset-sum knapsack interdiction (judge's own code).

Claim checked: for EVERY leader deletion within budget, the survivors have a subset sum in
[lo, hi] (hi = follower capacity).  Written from the problem definition only; shares no code
with bobilib.

Search: count formulation (delete the r cheapest copies of each weight class; dominance: any
deletion of r copies of a class costs at least the r cheapest and leaves the follower the same
multiset of weights).  Branch on r per class.  Prune a node only by
  (1) direct: decided survivors already reach [lo, hi];
  (2) the online game with base set A = S + [0, hi - lo] (S = decided sums): the leader
      deletes class by class in a fixed play order and may react to the follower's earlier
      choices; the follower must end with residual in A starting from residual hi.  Its value
      thr(t) = largest budget at which residual t is still won, computed by
      thr(t) = min(cap, min_r P(r) + max_{j <= n - r, j v <= t} thr'(t - j v)), base cap on A, -1 off A.
      If thr(hi) >= budget the follower wins this game, hence the real one (the real follower
      sees every deletion before answering).
A leaf (all classes decided) that does not reach [lo, hi] is a blocking deletion: reported.
"""
import gzip, sys, time, itertools, random
import numpy as np


def parse(mps_path):
    aux_path = mps_path[:-len('.mps.gz')] + '.aux' if mps_path.endswith('.mps.gz') else mps_path[:-4] + '.aux'
    op = gzip.open if mps_path.endswith('.gz') else open
    rows = {}
    cols = {}
    rhs = {}
    sec = None
    with op(mps_path, 'rt', encoding='latin-1') as f:
        for line in f:
            if line.startswith('*') or not line.strip():
                continue
            if not line[0].isspace():
                sec = line.split()[0]
                continue
            p = line.split()
            if sec == 'ROWS':
                rows[p[1]] = p[0]
            elif sec == 'COLUMNS':
                if "'MARKER'" in p:
                    continue
                c = cols.setdefault(p[0], {})
                for i in range(1, len(p) - 1, 2):
                    c[p[i]] = float(p[i + 1])
            elif sec == 'RHS':
                for i in range(1, len(p) - 1, 2):
                    rhs[p[i]] = float(p[i + 1])
    fobj = {}
    with open(aux_path, encoding='latin-1') as f:
        lines = [l.strip() for l in f]
    i = lines.index('@VARSBEGIN') + 1
    while lines[i] != '@VARSEND':
        a, b = lines[i].split()
        fobj[a] = float(b)
        i += 1
    i = lines.index('@CONSTRSBEGIN') + 1
    fcons = []
    while lines[i] != '@CONSTRSEND':
        fcons.append(lines[i])
        i += 1
    return rows, cols, rhs, fobj, fcons


def load(mps_path):
    rows, cols, rhs, fobj, fcons = parse(mps_path)
    B = rhs['leaderCons']
    C = rhs['followerCons']
    assert B == int(B) and C == int(C)
    B, C = int(B), int(C)
    ys = sorted([v for v in fobj], key=lambda s: int(s[1:]))
    items = []
    for y in ys:
        j = int(y[1:])
        x = 'x%d' % j
        yc, xc = cols[y], cols.get(x, {})
        w = yc.get('followerCons', 0.0)
        p = -fobj[y]
        assert w == p, 'not subset sum'
        assert w == int(w) and w >= 0
        # coupling row: x_j + y_j <= 1 with rhs 1
        r = 'inter%d' % j
        assert yc.get(r) == 1.0 and xc.get(r) == 1.0 and rhs.get(r) == 1.0 and rows[r] == 'L'
        assert set(yc) <= {'COST', 'followerCons', r}, yc
        assert set(xc) <= {'leaderCons', r}, xc
        cost = xc.get('leaderCons', 0.0)
        assert cost == int(cost) and cost >= 0
        items.append((int(w), int(cost)))
    assert rows['leaderCons'] == 'L' and rows['followerCons'] == 'L'
    # every follower row other than followerCons must be a coupling row
    for r in fcons:
        assert r == 'followerCons' or r.startswith('inter')
    return items, B, C


def classes_of(items, C):
    by = {}
    for w, c in items:
        if w == 0 or w > C:
            continue  # never matters for the follower's sums
        if c == 0:
            continue  # the leader deletes it for free (deleting only hurts the follower)
        by.setdefault(w, []).append(c)
    return [(w, sorted(cs)) for w, cs in sorted(by.items())]


class Prover:
    def __init__(self, classes, lo, hi, budget, search_order='asc', play_order='ratioasc',
                 early_exit=True):
        self.lo, self.hi, self.budget = lo, hi, budget
        cl = list(classes)
        if search_order == 'asc':
            cl.sort(key=lambda c: c[0])
        elif search_order == 'desc':
            cl.sort(key=lambda c: -c[0])
        elif search_order == 'random':
            random.shuffle(cl)
        self.cl = cl
        self.mask = (1 << (hi + 1)) - 1
        self.band = ((1 << (hi + 1)) - 1) ^ ((1 << lo) - 1)
        self.play = []
        m = len(cl)
        for k in range(m):
            idx = list(range(k, m))
            if play_order == 'ratioasc':
                # total deletion cost per unit of mass, cheapest first
                idx.sort(key=lambda i: sum(cl[i][1]) / (cl[i][0] * len(cl[i][1])))
            elif play_order == 'search':
                pass
            elif play_order == 'desc':
                idx.sort(key=lambda i: -cl[i][0])
            elif play_order == 'random':
                random.shuffle(idx)
            self.play.append(idx)
        self.prefix = []
        for v, cs in cl:
            p = [0]
            for c in cs:
                p.append(p[-1] + c)
            self.prefix.append(p)
        self.early_exit = early_exit
        self.nodes = 0
        self.table_prunes = 0
        self.direct = 0
        self.block = None

    def game_thr(self, k, S):
        """thr(hi) of the online game on classes k.. (play order) with base S + [0, hi-lo]."""
        hi, lo = self.hi, self.lo
        cap = self.budget
        bits = np.frombuffer(S.to_bytes((hi + 8) // 8 + 1, 'little'), dtype=np.uint8)
        sb = np.unpackbits(bits, bitorder='little')[:hi + 1].astype(bool)
        # dilate: A[t] iff some s in S with t - (hi-lo) <= s <= t
        slack = hi - lo
        cs = np.concatenate(([0], np.cumsum(sb)))
        idx = np.arange(hi + 1)
        lowi = np.maximum(idx - slack, 0)
        A = (cs[idx + 1] - cs[lowi]) > 0
        thr = np.where(A, cap, -1).astype(np.int64)
        for i in reversed(self.play[k]):
            v, costs = self.cl[i]
            n = len(costs)
            P = self.prefix[i]
            out = np.full(hi + 1, cap, dtype=np.int64)
            mx = thr.copy()
            for j in range(n + 1):
                if j > 0:
                    d = j * v
                    if d <= hi:
                        np.maximum(mx[d:], thr[:hi + 1 - d], out=mx[d:])
                r = n - j
                if P[r] <= cap:
                    np.minimum(out, P[r] + mx, out=out)
            thr = out
            if self.early_exit and thr.max() < self.budget_now:
                return int(thr.max())
        return int(thr[hi])

    def node(self, k, budget, S):
        self.nodes += 1
        if S & self.band:
            self.direct += 1
            return True
        if k == len(self.cl):
            return False
        self.budget_now = budget
        if self.game_thr(k, S) >= budget:
            self.table_prunes += 1
            return True
        v, costs = self.cl[k]
        n = len(costs)
        for r in range(n, -1, -1):
            if self.prefix[k][r] > budget:
                continue
            S2 = S
            for _ in range(n - r):
                S2 = (S2 | (S2 << v)) & self.mask
            self.choice[k] = r
            if not self.node(k + 1, budget - self.prefix[k][r], S2):
                return False
        return True

    def run(self):
        self.choice = [0] * len(self.cl)
        sys.setrecursionlimit(10000)
        ok = self.node(0, self.budget, 1)
        if not ok:
            self.block = list(zip([c[0] for c in self.cl], self.choice))
        return ok


def brute(classes, lo, hi, budget):
    """Exhaustive: every count vector within budget."""
    ranges = [range(len(cs) + 1) for _, cs in classes]
    for rs in itertools.product(*ranges):
        cost = sum(sum(cs[:r]) for (_, cs), r in zip(classes, rs))
        if cost > budget:
            continue
        S = 1
        for (v, cs), r in zip(classes, rs):
            for _ in range(len(cs) - r):
                S |= S << v
        S &= (1 << (hi + 1)) - 1
        if not any((S >> t) & 1 for t in range(lo, hi + 1)):
            return False
    return True


def brute_items(items, lo, hi, budget):
    """Exhaustive over item subsets (not classes) for tiny instances."""
    n = len(items)
    for mask in range(1 << n):
        cost = sum(items[j][1] for j in range(n) if mask >> j & 1)
        if cost > budget:
            continue
        S = 1
        for j in range(n):
            if not mask >> j & 1 and items[j][0] > 0:
                S |= S << items[j][0]
        if not any((S >> t) & 1 for t in range(lo, hi + 1)):
            return False
    return True


def selftest(rounds):
    rng = random.Random(7)
    stats = [0, 0, 0]
    for rd in range(rounds):
        n = rng.randint(1, 12)
        items = [(rng.randint(1, 12), rng.randint(0, 9)) for _ in range(n)]
        C = rng.randint(0, sum(w for w, _ in items) + 3)
        lo = max(0, C - rng.choice([0, 0, 1, 2, 5]))
        B = rng.randint(0, 30)
        expect = brute_items(items, lo, C, B)
        cl = classes_of(items, C)
        assert brute(cl, lo, C, B) == expect, (items, lo, C, B)
        so = rng.choice(['asc', 'desc', 'random'])
        po = rng.choice(['ratioasc', 'search', 'desc', 'random'])
        p = Prover(cl, lo, C, B, so, po, early_exit=rng.random() < 0.5)
        got = p.run()
        assert got == expect, (rd, items, lo, C, B, so, po, got, expect)
        stats[0 if got else 1] += 1
        stats[2] += p.table_prunes
    print('selftest ok: %d safe, %d blocked, %d table prunes' % tuple(stats))


if __name__ == '__main__':
    if sys.argv[1] == 'selftest':
        selftest(int(sys.argv[2]))
        sys.exit(0)
    path = sys.argv[1]
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else None
    so = sys.argv[3] if len(sys.argv) > 3 else 'asc'
    po = sys.argv[4] if len(sys.argv) > 4 else 'ratioasc'
    items, B, C = load(path)
    if len(sys.argv) > 5:
        B = int(sys.argv[5])
    cl = classes_of(items, C)
    if lo is None:
        lo = C
    t = time.time()
    p = Prover(cl, lo, C, B, so, po)
    ok = p.run()
    print('%s n=%d B=%d C=%d classes=%d lo=%d -> %s nodes=%d table_prunes=%d direct=%d %.1fs%s' % (
        path.split('/')[-1], len(items), B, C, len(cl), lo, 'SAFE' if ok else 'BLOCKED', p.nodes,
        p.table_prunes, p.direct, time.time() - t, '' if ok else ' block=%s' % p.block))
    if not ok:
        # check the block independently: cost and best follower response
        cost = 0
        S = 1
        for (v, cs), r in zip(p.cl, p.choice):
            cost += sum(cs[:r])
            for _ in range(len(cs) - r):
                S |= S << v
        S &= (1 << (C + 1)) - 1
        best = max(t for t in range(C + 1) if (S >> t) & 1)
        print('  block check: cost %d <= %d, follower best %d < %d' % (cost, B, best, lo))
