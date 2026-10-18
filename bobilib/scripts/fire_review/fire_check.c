// Reviewer's independent check of BOBILib inter-fire (attack-protect) instances.
// Written before reading src/fire.rs; reads the graph files made by
// parse_fire_model.py, which asserts the model pattern row by row.
//
// Model (re-derived from the MPS/AUX files): leader y in {0,1}^n, sum y <= Phi;
// follower (a, x) in {0,1}^{2n}: max sum a s.t. sum x <= Lam, a_v + y_v <= 1,
// a_v <= a_u + x_v for both arcs of every edge. The leader minimises sum a.
// For fixed Y and X the largest feasible a is the complement of the burned set
// B = closure of Y under spreading to unprotected neighbours, so the follower's
// optimum is max over |X| <= Lam of n - |B(Y, X)|.
//
// Modes (G = graph file):
//   enum G     forests: every attack |Y| <= Phi; tree DP over (a_v, x_v) with
//              traceback; each traced point is checked against every ILP row and
//              its X by the burned-set closure (a lower-bound witness); then brute
//              force over ALL X (|X| <= Lam, any nodes) at every minimising attack.
//   brute G    any graph: every attack |Y| <= Phi, follower by brute force over
//              X subset of V \ Y with |X| = min(Lam, |V \ Y|) (saved is monotone in
//              X and protecting an attacked node does nothing); DP compared on forests.
//   hash G     forests: FNV-1a hash of the DP value at every attack, in DFS order
//              over nodes sorted by label (to compare with another implementation).
//   sample G N SEED   N random full-budget attacks, DP against brute force.
//   eval G l1,l2,...  follower optimum at one attack (labels) by brute force over all X.
// Build: cc -O2 -o fire_check fire_check.c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
typedef unsigned __int128 u128;
#define MAXN 128
#define MAXL 8
#define NEG (-1000000)
static int n, m, Phi, Lam, lab[MAXN];
static int deg[MAXN], adj[MAXN][MAXN];
static u128 adjm[MAXN];
static int isforest;
static int order[MAXN], parent[MAXN], norder; // DFS preorder over forest
static int roots[MAXN], nroots;

static int popc(u128 x) { return __builtin_popcountll((unsigned long long)x) + __builtin_popcountll((unsigned long long)(x >> 64)); }

// burned closure size
static int burned(u128 Y, u128 X) {
    u128 B = Y, F = Y;
    while (F) {
        u128 N = 0;
        u128 f = F;
        while (f) {
            int v;
            unsigned long long lo = (unsigned long long)f;
            if (lo) v = __builtin_ctzll(lo); else v = 64 + __builtin_ctzll((unsigned long long)(f >> 64));
            f &= ~((u128)1 << v);
            N |= adjm[v];
        }
        N &= ~X;
        N &= ~B;
        B |= N;
        F = N;
    }
    return popc(B);
}
static int saved(u128 Y, u128 X) { return n - burned(Y, X); }

// ---------- DP over (a_v,x_v) states on the forest ----------
// state s: bit0 = x, bit1 = a
static int f[MAXN][4][MAXL + 1];          // full subtree table
// partial tables: P[v][j] after merging first j children (j = 0..nch)
static int nch[MAXN], ch[MAXN][MAXN];
static int *P[MAXN]; // P[v] has (nch+1)*4*(Lam+1) ints
#define PIDX(v, j, s, k) P[v][((j) * 4 + (s)) * (Lam + 1) + (k)]

static int compat(int sp, int sc) {
    int ap = sp >> 1, xp = sp & 1, ac = sc >> 1, xc = sc & 1;
    // arc (p,c): a_c <= a_p + x_c ; arc (c,p): a_p <= a_c + x_p
    return ac <= ap + xc && ap <= ac + xp;
}

static int dp(u128 Y) {
    for (int i = norder - 1; i >= 0; i--) {
        int v = order[i];
        int yv = (int)((Y >> v) & 1);
        for (int s = 0; s < 4; s++)
            for (int k = 0; k <= Lam; k++) PIDX(v, 0, s, k) = NEG;
        for (int s = 0; s < 4; s++) {
            int a = s >> 1, x = s & 1;
            if (a + yv > 1) continue;
            if (x > Lam) continue;
            PIDX(v, 0, s, x) = a;
        }
        for (int j = 0; j < nch[v]; j++) {
            int c = ch[v][j];
            for (int s = 0; s < 4; s++)
                for (int k = 0; k <= Lam; k++) {
                    int best = NEG;
                    for (int k1 = 0; k1 <= k; k1++) {
                        int pv = PIDX(v, j, s, k1);
                        if (pv <= NEG / 2) continue;
                        for (int t = 0; t < 4; t++) {
                            if (!compat(s, t)) continue;
                            int cv = f[c][t][k - k1];
                            if (cv <= NEG / 2) continue;
                            if (pv + cv > best) best = pv + cv;
                        }
                    }
                    PIDX(v, j + 1, s, k) = best;
                }
        }
        for (int s = 0; s < 4; s++)
            for (int k = 0; k <= Lam; k++) f[v][s][k] = PIDX(v, nch[v], s, k);
    }
    // combine components
    static int g[MAXN + 1][MAXL + 1];
    for (int k = 0; k <= Lam; k++) g[0][k] = (k == 0) ? 0 : NEG;
    for (int r = 0; r < nroots; r++) {
        int v = roots[r];
        for (int k = 0; k <= Lam; k++) {
            int best = NEG;
            for (int k1 = 0; k1 <= k; k1++) {
                if (g[r][k1] <= NEG / 2) continue;
                for (int s = 0; s < 4; s++) {
                    int cv = f[v][s][k - k1];
                    if (cv <= NEG / 2) continue;
                    if (g[r][k1] + cv > best) best = g[r][k1] + cv;
                }
            }
            g[r + 1][k] = best;
        }
    }
    int best = NEG;
    for (int k = 0; k <= Lam; k++)
        if (g[nroots][k] > best) best = g[nroots][k];
    return best;
}

// reconstruct X from the tables: returns X mask achieving the DP value (recomputes component combination)
static u128 recon_X;
static int recon_a[MAXN], recon_x[MAXN];
static void recon_v(int v, int s, int k) {
    recon_a[v] = s >> 1;
    recon_x[v] = s & 1;
    if (s & 1) recon_X |= (u128)1 << v;
    for (int j = nch[v]; j >= 1; j--) {
        int c = ch[v][j - 1];
        int target = PIDX(v, j, s, k);
        int found = 0;
        for (int k2 = 0; k2 <= k && !found; k2++) {
            int pv = PIDX(v, j - 1, s, k - k2);
            if (pv <= NEG / 2) continue;
            for (int t = 0; t < 4 && !found; t++) {
                if (!compat(s, t)) continue;
                int cv = f[c][t][k2];
                if (cv <= NEG / 2) continue;
                if (pv + cv == target) {
                    recon_v(c, t, k2);
                    k -= k2;
                    found = 1;
                }
            }
        }
        if (!found) { fprintf(stderr, "recon failure\n"); exit(3); }
    }
}
static u128 reconstruct(int value) {
    // recompute g and backtrack
    static int g[MAXN + 1][MAXL + 1];
    for (int k = 0; k <= Lam; k++) g[0][k] = (k == 0) ? 0 : NEG;
    for (int r = 0; r < nroots; r++) {
        int v = roots[r];
        for (int k = 0; k <= Lam; k++) {
            int best = NEG;
            for (int k1 = 0; k1 <= k; k1++) {
                if (g[r][k1] <= NEG / 2) continue;
                for (int s = 0; s < 4; s++) {
                    int cv = f[v][s][k - k1];
                    if (cv <= NEG / 2) continue;
                    if (g[r][k1] + cv > best) best = g[r][k1] + cv;
                }
            }
            g[r + 1][k] = best;
        }
    }
    int k = -1;
    for (int kk = 0; kk <= Lam; kk++) if (g[nroots][kk] == value) { k = kk; break; }
    if (k < 0) { fprintf(stderr, "recon: no k\n"); exit(3); }
    recon_X = 0;
    for (int r = nroots - 1; r >= 0; r--) {
        int v = roots[r];
        int found = 0;
        for (int k2 = 0; k2 <= k && !found; k2++) {
            if (g[r][k - k2] <= NEG / 2) continue;
            for (int s = 0; s < 4 && !found; s++) {
                int cv = f[v][s][k2];
                if (cv <= NEG / 2) continue;
                if (g[r][k - k2] + cv == g[r + 1][k]) {
                    recon_v(v, s, k2);
                    k -= k2;
                    found = 1;
                }
            }
        }
        if (!found) { fprintf(stderr, "recon root failure\n"); exit(3); }
    }
    return recon_X;
}

// check the reconstructed (a,x) point directly against the ILP rows
static int check_point(u128 Y, int value) {
    int sx = 0, sa = 0;
    for (int v = 0; v < n; v++) {
        sx += recon_x[v];
        sa += recon_a[v];
        int yv = (int)((Y >> v) & 1);
        if (recon_a[v] + yv > 1) return 0;
        for (int i = 0; i < deg[v]; i++) {
            int u = adj[v][i];
            if (recon_a[v] > recon_a[u] + recon_x[v]) return 0;
        }
    }
    if (sx > Lam) return 0;
    if (sa != value) return 0;
    return 1;
}

// brute force follower over all X (|X| <= Lam) among `cand` nodes; returns max saved
static int cand[MAXN], ncand;
static int bestsaved;
static long long nevals;
static void rec(int start, int left, u128 Y, u128 X) {
    int s = saved(Y, X);
    nevals++;
    if (s > bestsaved) bestsaved = s;
    if (left == 0) return;
    for (int i = start; i < ncand; i++) rec(i + 1, left - 1, Y, X | ((u128)1 << cand[i]));
}
static int brute_all_X(u128 Y) { // any node, |X|<=Lam
    ncand = n;
    for (int i = 0; i < n; i++) cand[i] = i;
    bestsaved = -1;
    rec(0, Lam, Y, 0);
    return bestsaved;
}
// exactly min(Lam, |V\Y|) nodes of V\Y
static int bests2;
static void rec2(int start, int left, u128 Y, u128 X) {
    if (left == 0) {
        int s = saved(Y, X);
        nevals++;
        if (s > bests2) bests2 = s;
        return;
    }
    for (int i = start; i <= ncand - left; i++) rec2(i + 1, left - 1, Y, X | ((u128)1 << cand[i]));
}
static int brute_fast(u128 Y) {
    ncand = 0;
    for (int i = 0; i < n; i++) if (!((Y >> i) & 1)) cand[ncand++] = i;
    int L = Lam < ncand ? Lam : ncand;
    bests2 = -1;
    rec2(0, L, Y, 0);
    return bests2;
}

static void read_graph(const char *path) {
    FILE *fp = fopen(path, "r");
    if (!fp) { perror(path); exit(1); }
    if (fscanf(fp, "%d %d %d %d", &n, &m, &Phi, &Lam) != 4) exit(1);
    if (n > MAXN || Lam > MAXL) { fprintf(stderr, "too big\n"); exit(1); }
    for (int i = 0; i < n; i++) if (fscanf(fp, "%d", &lab[i]) != 1) exit(1);
    for (int e = 0; e < m; e++) {
        int u, v;
        if (fscanf(fp, "%d %d", &u, &v) != 2) exit(1);
        adj[u][deg[u]++] = v;
        adj[v][deg[v]++] = u;
        adjm[u] |= (u128)1 << v;
        adjm[v] |= (u128)1 << u;
    }
    fclose(fp);
    // forest check + rooting by iterative DFS
    int seen[MAXN] = {0};
    isforest = 1;
    norder = 0; nroots = 0;
    for (int r = 0; r < n; r++) {
        if (seen[r]) continue;
        roots[nroots++] = r;
        int stack[MAXN], sp = 0;
        stack[sp++] = r; seen[r] = 1; parent[r] = -1;
        while (sp) {
            int v = stack[--sp];
            order[norder++] = v;
            nch[v] = 0;
            for (int i = 0; i < deg[v]; i++) {
                int u = adj[v][i];
                if (u == parent[v]) continue;
                if (seen[u]) { isforest = 0; continue; }
                seen[u] = 1; parent[u] = v;
                stack[sp++] = u;
            }
        }
    }
    if (isforest) {
        for (int i = 0; i < norder; i++) {
            int v = order[i];
            if (parent[v] >= 0) ch[parent[v]][nch[parent[v]]++] = v;
        }
        // DFS preorder lists every child after its parent; dp() walks it backwards.
        for (int v = 0; v < n; v++) P[v] = malloc(sizeof(int) * (nch[v] + 1) * 4 * (Lam + 1));
    }
}

// enumerate attacks |Y| <= Phi
typedef void (*cb_t)(u128 Y, int sz);
static void enum_rec(int start, int depth, u128 Y, cb_t cb) {
    cb(Y, depth);
    if (depth == Phi) return;
    for (int i = start; i < n; i++) enum_rec(i + 1, depth + 1, Y | ((u128)1 << i), cb);
}

static long long cnt_attacks, cnt_by_size[MAXL + 2];
static int minval = 1 << 30;
#define MAXSTORE 100000
static u128 argmin[MAXSTORE];
static int nargmin;
static int histo[MAXN + 1];
static long long witness_fail, point_fail, mismatch;
static int mode_brute;

static void cb_enum(u128 Y, int sz) {
    cnt_attacks++;
    cnt_by_size[sz]++;
    int val = dp(Y);
    u128 X = reconstruct(val);
    if (!check_point(Y, val)) point_fail++;
    int s = saved(Y, X);
    if (s != val) witness_fail++; // s >= val is the certificate; equality is expected
    if (popc(X) > Lam) witness_fail++;
    histo[val]++;
    if (val < minval) { minval = val; nargmin = 0; }
    if (val == minval && nargmin < MAXSTORE) argmin[nargmin++] = Y;
}

static void cb_brute(u128 Y, int sz) {
    cnt_attacks++;
    cnt_by_size[sz]++;
    int b = brute_fast(Y);
    if (isforest) {
        int val = dp(Y);
        if (val != b) {
            mismatch++;
            if (mismatch < 5) fprintf(stderr, "MISMATCH dp %d brute %d\n", val, b);
        }
    }
    histo[b]++;
    if (b < minval) { minval = b; nargmin = 0; }
    if (b == minval && nargmin < MAXSTORE) argmin[nargmin++] = Y;
}

static unsigned long long hsh = 0xcbf29ce484222325ULL;
static void cb_hash(u128 Y, int sz) {
    cnt_attacks++;
    cnt_by_size[sz]++;
    int val = dp(Y);
    unsigned int u = (unsigned int)val;
    for (int b = 0; b < 4; b++) { hsh ^= (u >> (8 * b)) & 0xff; hsh *= 0x100000001b3ULL; }
    if (val < minval) minval = val;
}
static unsigned long long rng = 0x9e3779b97f4a7c15ULL;
static unsigned long long nextr(void) { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; return rng; }
static void print_set(u128 S) {
    int first = 1;
    for (int v = 0; v < n; v++) if ((S >> v) & 1) { printf(first ? "%d" : ",%d", lab[v]); first = 0; }
    if (first) printf("-");
}

int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: fire_check enum|brute|hash|sample|eval G [args]\n"); return 1; }
    read_graph(argv[2]);
    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    if (!strcmp(argv[1], "eval")) {
        u128 Y = 0;
        char *s = strdup(argv[3]);
        for (char *tok = strtok(s, ","); tok; tok = strtok(NULL, ",")) {
            int L = atoi(tok), found = -1;
            for (int i = 0; i < n; i++) if (lab[i] == L) found = i;
            if (found < 0) { fprintf(stderr, "no label %d\n", L); return 1; }
            Y |= (u128)1 << found;
        }
        int b = brute_all_X(Y);
        printf("attack ");
        print_set(Y);
        printf(" brute_all_X=%d evals=%lld", b, nevals);
        if (isforest) printf(" dp=%d", dp(Y));
        printf("\n");
        return 0;
    }
    if (!strcmp(argv[1], "hash")) {
        enum_rec(0, 0, 0, cb_hash);
        printf("attacks=%lld min=%d hash=%016llx\n", cnt_attacks, minval, hsh);
        return 0;
    }
    if (!strcmp(argv[1], "sample")) {
        int N = atoi(argv[3]);
        rng ^= (unsigned long long)atoll(argv[4]) * 0x2545F4914F6CDD1DULL;
        int mism = 0;
        for (int t = 0; t < N; t++) {
            u128 Y = 0;
            int k = 0;
            while (k < Phi && k < n) { int v = (int)(nextr() % n); if (!((Y >> v) & 1)) { Y |= (u128)1 << v; k++; } }
            int b = brute_fast(Y);
            int d = isforest ? dp(Y) : b;
            if (b != d) mism++;
        }
        printf("sample=%d mismatches=%d\n", N, mism);
        return 0;
    }
    if (!strcmp(argv[1], "enum")) {
        if (!isforest) { fprintf(stderr, "not a forest\n"); return 1; }
        enum_rec(0, 0, 0, cb_enum);
    } else if (!strcmp(argv[1], "brute")) {
        mode_brute = 1;
        enum_rec(0, 0, 0, cb_brute);
    } else return 1;
    clock_gettime(CLOCK_MONOTONIC, &t1);
    double el = (t1.tv_sec - t0.tv_sec) + 1e-9 * (t1.tv_nsec - t0.tv_nsec);
    printf("n=%d m=%d Phi=%d Lam=%d forest=%d attacks=%lld", n, m, Phi, Lam, isforest, cnt_attacks);
    printf(" (by size:");
    for (int k = 0; k <= Phi; k++) printf(" %lld", cnt_by_size[k]);
    printf(") min=%d argmin_attacks=%d", minval, nargmin);
    int nfull = 0;
    for (int i = 0; i < nargmin; i++) if (popc(argmin[i]) == Phi) nfull++;
    printf(" (of size Phi: %d)", nfull);
    if (!mode_brute) printf(" witness_fail=%lld point_fail=%lld", witness_fail, point_fail);
    else printf(" dp_mismatch=%lld", mismatch);
    printf(" time=%.2fs\n", el);
    // second-smallest value
    int second = -1;
    for (int v = minval + 1; v <= n; v++) if (histo[v]) { second = v; break; }
    printf("histogram_min_values: %d:%d next %d:%d\n", minval, histo[minval], second, second >= 0 ? histo[second] : 0);
    // brute force at every argmin attack (all X, any nodes)
    int bad = 0;
    for (int i = 0; i < nargmin; i++) {
        nevals = 0;
        int b = brute_all_X(argmin[i]);
        if (b != minval) bad++;
        if (i < 40) {
            printf("argmin ");
            print_set(argmin[i]);
            printf(" brute_all_X=%d\n", b);
        }
    }
    printf("argmin brute-force disagreements=%d (of %d)\n", bad, nargmin);
    return 0;
}
