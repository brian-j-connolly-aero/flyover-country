/* Reviewer's exact search: does graph H (the complement of G) have a partition into 3
 * labelled parts of sizes exactly s0,s1,s2 with at most B edges of H between parts?
 *
 * DFS over vertices in a connectivity order. Lower bound: current cut + sum over unassigned
 * vertices v of min over parts p with room of (#assigned H-neighbours of v not in p).
 * Symmetry: among parts of equal size, part j may be opened only after part j-1.
 *
 * Modes:
 *   rbnb decide FILE s0 s1 s2 B     FILE: "n m" then m lines "u v" (edges of H)
 *   rbnb selftest SEED COUNT        compare with brute force on random small graphs
 *   rbnb ls GFILE RESTARTS ITERS SEED   local search for min inside edges of a 3-partition of G
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAXN 64
static int n;
static int adjn[MAXN][MAXN], deg[MAXN];
static int order[MAXN], pos[MAXN];
static int part[MAXN];
static int cnt[MAXN][3];   /* assigned H-neighbours of v in part p */
static int asg[MAXN];      /* assigned H-neighbours of v */
static int sz[3], cap[3], B;
static long long nodes;
static int found;

static void make_order(void) {
    int used[MAXN] = {0};
    int conn[MAXN] = {0};
    for (int i = 0; i < n; i++) {
        int best = -1;
        for (int v = 0; v < n; v++) {
            if (used[v]) continue;
            if (best < 0 || conn[v] > conn[best] || (conn[v] == conn[best] && deg[v] > deg[best]))
                best = v;
        }
        used[best] = 1;
        order[i] = best;
        pos[best] = i;
        for (int j = 0; j < deg[best]; j++) conn[adjn[best][j]]++;
    }
}

static int lower_bound(int d, int cut) {
    int lb = cut;
    for (int i = d; i < n; i++) {
        int v = order[i];
        int best = 1 << 30;
        for (int p = 0; p < 3; p++) {
            if (sz[p] >= cap[p]) continue;
            int c = asg[v] - cnt[v][p];
            if (c < best) best = c;
        }
        lb += best;
        if (lb > B) return lb;
    }
    return lb;
}

static void assign(int v, int p, int sign) {
    for (int j = 0; j < deg[v]; j++) {
        int u = adjn[v][j];
        cnt[u][p] += sign;
        asg[u] += sign;
    }
    sz[p] += sign;
}

static void dfs(int d, int cut) {
    if (found) return;
    nodes++;
    if (d == n) {
        found = 1;
        return;
    }
    if (lower_bound(d, cut) > B) return;
    int v = order[d];
    int ps[3], cs[3], np = 0;
    for (int p = 0; p < 3; p++) {
        if (sz[p] >= cap[p]) continue;
        /* symmetry: equal-size parts opened in index order */
        if (sz[p] == 0 && p > 0 && cap[p - 1] == cap[p] && sz[p - 1] == 0) continue;
        ps[np] = p;
        cs[np] = asg[v] - cnt[v][p];
        np++;
    }
    /* try cheaper parts first */
    for (int a = 0; a < np; a++)
        for (int b2 = a + 1; b2 < np; b2++)
            if (cs[b2] < cs[a]) {
                int t = cs[a]; cs[a] = cs[b2]; cs[b2] = t;
                t = ps[a]; ps[a] = ps[b2]; ps[b2] = t;
            }
    for (int a = 0; a < np; a++) {
        if (cut + cs[a] > B) continue;
        int p = ps[a];
        part[v] = p;
        assign(v, p, +1);
        dfs(d + 1, cut + cs[a]);
        if (found) return; /* keep the found partition in part[] */
        assign(v, p, -1);
        part[v] = -1;
    }
}

static int decide(int s0, int s1, int s2, int budget) {
    /* parts must be listed in non-increasing order for the symmetry rule to be valid */
    cap[0] = s0; cap[1] = s1; cap[2] = s2;
    B = budget;
    memset(cnt, 0, sizeof cnt);
    memset(asg, 0, sizeof asg);
    memset(sz, 0, sizeof sz);
    for (int i = 0; i < n; i++) part[i] = -1;
    nodes = 0;
    found = 0;
    make_order();
    dfs(0, 0);
    return found;
}

static void add_edge(int u, int v) {
    adjn[u][deg[u]++] = v;
    adjn[v][deg[v]++] = u;
}

static int read_graph(const char *fn) {
    FILE *f = fopen(fn, "r");
    if (!f) { perror(fn); exit(2); }
    int m;
    if (fscanf(f, "%d %d", &n, &m) != 2) exit(2);
    memset(deg, 0, sizeof deg);
    for (int i = 0; i < m; i++) {
        int u, v;
        if (fscanf(f, "%d %d", &u, &v) != 2) exit(2);
        add_edge(u, v);
    }
    fclose(f);
    return m;
}

static unsigned long long rs = 88172645463325252ULL;
static unsigned long long rnd(void) {
    rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17; return rs;
}

/* brute force: min cut for exact sizes, over all 3^n labelled assignments */
static int brute_min(int s0, int s1, int s2, int (*E)[2], int m) {
    int best = 1 << 30;
    int lab[16];
    long long tot = 1;
    for (int i = 0; i < n; i++) tot *= 3;
    for (long long code = 0; code < tot; code++) {
        long long c = code;
        int z[3] = {0, 0, 0};
        for (int i = 0; i < n; i++) { lab[i] = c % 3; c /= 3; z[lab[i]]++; }
        if (z[0] != s0 || z[1] != s1 || z[2] != s2) continue;
        int cut = 0;
        for (int e = 0; e < m; e++) cut += lab[E[e][0]] != lab[E[e][1]];
        if (cut < best) best = cut;
    }
    return best;
}

int main(int argc, char **argv) {
    if (argc >= 7 && !strcmp(argv[1], "decide")) {
        read_graph(argv[2]);
        int s0 = atoi(argv[3]), s1 = atoi(argv[4]), s2 = atoi(argv[5]), b = atoi(argv[6]);
        if (!(s0 >= s1 && s1 >= s2 && s0 + s1 + s2 == n)) { fprintf(stderr, "bad sizes\n"); return 2; }
        int r = decide(s0, s1, s2, b);
        printf("%s nodes=%lld\n", r ? "FOUND" : "NONE", nodes);
        if (r) {
            /* recount the cut of the found partition from scratch */
            int cut = 0, z[3] = {0, 0, 0};
            for (int v = 0; v < n; v++) z[part[v]]++;
            for (int v = 0; v < n; v++)
                for (int j = 0; j < deg[v]; j++)
                    if (v < adjn[v][j] && part[v] != part[adjn[v][j]]) cut++;
            printf("recount cut=%d sizes=%d,%d,%d\n", cut, z[0], z[1], z[2]);
        }
        return 0;
    }
    if (argc >= 4 && !strcmp(argv[1], "selftest")) {
        rs ^= (unsigned long long)atoll(argv[2]) * 2654435761ULL;
        int count = atoi(argv[3]);
        int bad = 0, checked = 0;
        for (int t = 0; t < count; t++) {
            n = 6 + rnd() % 6; /* 6..11 */
            int E[64][2], m = 0;
            memset(deg, 0, sizeof deg);
            int dens = 20 + rnd() % 60;
            for (int u = 0; u < n; u++)
                for (int v = u + 1; v < n; v++)
                    if ((int)(rnd() % 100) < dens && m < 60) { E[m][0] = u; E[m][1] = v; m++; add_edge(u, v); }
            for (int s0 = 0; s0 <= n; s0++)
                for (int s1 = 0; s1 <= s0; s1++) {
                    int s2 = n - s0 - s1;
                    if (s2 < 0 || s2 > s1) continue;
                    int bm = brute_min(s0, s1, s2, E, m);
                    for (int b = 0; b <= m; b++) {
                        int r = decide(s0, s1, s2, b);
                        checked++;
                        if (r != (bm <= b)) { bad++; fprintf(stderr, "MISMATCH n=%d s=%d,%d,%d b=%d brute=%d r=%d\n", n, s0, s1, s2, b, bm, r); }
                    }
                }
        }
        printf("selftest checked=%d bad=%d\n", checked, bad);
        return bad != 0;
    }
    if (argc >= 6 && !strcmp(argv[1], "ls")) {
        /* local search on G: minimise edges inside parts (parts may be any size) */
        int m = read_graph(argv[2]);
        int R = atoi(argv[3]);
        long its = atol(argv[4]);
        rs ^= (unsigned long long)atoll(argv[5]) * 2654435761ULL;
        int best = 1 << 30;
        static int g[MAXN][3];
        static int tabu[MAXN][3];
        for (int r = 0; r < R; r++) {
            for (int v = 0; v < n; v++) part[v] = rnd() % 3;
            memset(g, 0, sizeof g);
            memset(tabu, 0, sizeof tabu);
            int cur = 0;
            for (int v = 0; v < n; v++)
                for (int j = 0; j < deg[v]; j++) {
                    g[v][part[adjn[v][j]]]++;
                    if (v < adjn[v][j] && part[v] == part[adjn[v][j]]) cur++;
                }
            int rb = cur;
            for (long it = 1; it <= its; it++) {
                int bv = -1, bp = -1, bd = 1 << 30;
                for (int v = 0; v < n; v++)
                    for (int p = 0; p < 3; p++) {
                        if (p == part[v]) continue;
                        int dlt = g[v][p] - g[v][part[v]];
                        if (tabu[v][p] > it && cur + dlt >= rb) continue;
                        if (dlt < bd || (dlt == bd && rnd() % 2)) { bd = dlt; bv = v; bp = p; }
                    }
                if (bv < 0) continue;
                int op = part[bv];
                tabu[bv][op] = it + 7 + rnd() % 10;
                for (int j = 0; j < deg[bv]; j++) {
                    int u = adjn[bv][j];
                    g[u][op]--;
                    g[u][bp]++;
                }
                part[bv] = bp;
                cur += bd;
                if (cur < rb) rb = cur;
            }
            if (rb < best) best = rb;
        }
        printf("best_inside=%d m=%d\n", best, m);
        return 0;
    }
    fprintf(stderr, "usage\n");
    return 2;
}
