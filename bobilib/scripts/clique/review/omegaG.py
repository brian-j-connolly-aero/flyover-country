import sys, os, json
sys.setrecursionlimit(10000)
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



GD=sys.argv[1]
for fn in sorted(os.listdir(GD)):
    if not fn.endswith('.edges'): continue
    L=open(os.path.join(GD,fn)).read().split('\n'); n,m,k=map(int,L[0].split())
    adj=[0]*n
    for l in L[1:]:
        if l.strip():
            u,v=map(int,l.split()); adj[u]|=1<<v; adj[v]|=1<<u
    print(json.dumps(dict(name=fn[:-6], omega=bk_max(adj,n))), flush=True)
