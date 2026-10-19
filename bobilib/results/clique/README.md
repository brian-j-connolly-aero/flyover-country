# Edge clique interdiction, BOBILib set `clique`: an independent check

BOBILib's `clique` set has 60 instances, `normalClique{n}-{d}-{i}` with n in {40, 50, 60},
density d in {0.7, 0.9} and i = 1..10. The leader deletes at most k edges of a graph G, the
follower then takes a maximum clique of what is left, and the leader minimises that clique's
size. BOBILib lists all 60 as `feasible`, with values 5 to 13.

An earlier research agent claimed 57 of the 60 are now solved to optimality (value 3 at
density 0.7, value 4 at density 0.9), with normalClique60-70-2, -3 and -5 open between 3 and 4.
This directory re-derives everything from the instance files, with new code
(`../../scripts/clique/`), and tries to break the claim.

## Verdict

**The claim holds. 57 of 60 are optimal, and the other three are open.**

| group | k | instances | value | lower bound | upper bound |
| --- | --- | --- | --- | --- | --- |
| n=40, d=0.7 | 136 | 10 | **3** | Mantel | 3-partition, 123 to 128 inside edges |
| n=50, d=0.7 | 214 | 10 | **3** | Mantel | 3-partition, 203 to 210 inside edges |
| n=60, d=0.7 | 309 | 7 | **3** | Mantel | 3-partition, 306 to 309 inside edges |
| n=60, d=0.7 (-2, -3, -5) | 309 | 3 | 3 or 4 (open) | Mantel | 4-partition, 200 to 202 inside edges |
| n=40, d=0.9 | 175 | 10 | **4** | Brouwer + w3(G) > k | 4-partition, 127 to 131 inside edges |
| n=50, d=0.9 | 275 | 10 | **4** | Brouwer + w3(G) > k | 4-partition, 209 to 212 inside edges |
| n=60, d=0.9 | 398 | 10 | **4** | Brouwer + w3(G) > k | 4-partition, 310 to 317 inside edges |

Here w_r(G) is the fewest edges of G inside the parts of an r-partition of V(G) (parts may be
empty). Each instance's numbers are in `summary.csv`. Each upper bound is a solution file
`<name>.res.json` in BOBILib's format: the leader deletes exactly the inside edges of the
partition, and the follower's decision is a maximum clique of the remaining graph.

I found no error in the earlier claim. Its arithmetic, its reading of Brouwer's theorem
(checked against the scanned 1981 report), its partitions' inside-edge counts (I matched all of
them with my own search) and its "every 3-partition has more than k inside edges" result
(re-proved three ways) all hold up. The three open instances stay open (section (d)). Four
searches for a K4-transversal within budget all stop at 311, 310 and 310 edges against k = 309.
Three of them can reach remainders that are not 3-partite. LP bounds on the transversal
number stop near 296.

Points to watch (none of them changes a result):

* The earlier agent's logs show `minwithin3_lb = 399` for every n=60, d=0.9 instance. That is
  not a computed bound. Its script sets the bound to the target when SCIP reports
  `infeasible` under an objective limit, so the field means only "w3 > k = 398" (likewise 276
  at n=50). My search finds w3 ≤ 450 to 456 for the n=60 graphs. The same runs print a
  "best" objective next to `infeasible` (1095 to 1310, and a fractional 802.68 for 50-90-8),
  which is not a solution under the limit. The "infeasible under the objective limit" logic
  itself is sound, and my own runs (explicit budget rows, three methods) agree with it.
* normalClique60-70-4 is tight: the best 3-partition found has exactly k = 309 inside edges.
  The value-3 certificate uses the whole budget.
* k = floor(m/4) in every `clique` instance. In `bcpins` and `plusbcpins` it is ceil(m/4)
  instead, which differs from floor(m/4) in 70 of the 80 graphs (for example m = 19, k = 5).

## (a) The formulation, row by row

`cliquelib.Instance` reads the MPS and AUX files generically and then checks every row and
coefficient against a pattern. It raises an error on anything unexpected. All 60 instances
pass the check.

* Variables (all binary). Leader: `w{u},{v}`, one for each edge uv of G (u < v). Follower:
  `x{i}` for each vertex and `y{u},{v}` for each edge.
* Objectives. The leader minimises sum x_i (`cost` row coefficient +1 on every x and 0 on
  everything else). The follower minimises -sum x_i (AUX coefficient -1 on x and 0 on y). The
  two objectives are exact negatives, so the optimistic and pessimistic readings coincide.
* Leader row `KL`: sum of all w <= k. It is the only leader row.
* Follower rows:
  * `C{u},{v}`: x_u + x_v <= 1, for exactly the non-edges of G. The C rows plus the edges
    cover every vertex pair exactly once.
  * `E{u},{v}`: x_u + x_v - y_uv <= 1, one per edge.
  * `I{u},{v}`: w_uv + y_uv <= 1, one per edge.

  If uv is deleted (w_uv = 1), then y_uv = 0 and so x_u + x_v <= 1. If uv is kept, nothing
  restricts the pair. So the follower's x is exactly a clique of G minus the deleted edges.
  The y variables carry no objective.
* `clique` has no extra rows. `plusbcpins` (the same 80 graphs and budgets as `bcpins`, in this
  formulation) adds lifted rows `S{u},{v},{w}`: x_u + x_v + x_w - y_uv <= 1 (0 to 102 per
  instance, 1899 in all). The code
  checks that in every one of them w is adjacent to neither u nor v. Then x_w = 1 forces
  x_u = x_v = 0, so the row cuts off no 0/1 point. If w were adjacent to one of them, the
  follower clique {v, w} would need uv kept, which would make it a different game. That never
  happens. `bcpins` uses a third encoding with generic row names (`c1` is the budget,
  w_uv + x_u + x_v <= 2 per edge, x_u + x_v <= 1 per non-edge), checked the same way by
  `cliquelib.BcpInstance`.
* BOBILib's own solution files evaluate to their listed values. Each is within budget, its
  clique is a clique of the remaining graph, and that graph's exact maximum clique equals the
  listed value (`parse_check.py`). So BOBILib's numbers are honest feasible points whose
  leader choices are just poor. The graphs' own clique numbers are 10 to 13 (d=0.7) and 18 to
  27 (d=0.9).

## (b) Upper bounds

`rcut_tabu.rs` is a standalone tabu search for max r-cut. Its moves relocate a single vertex,
it has aspiration and perturbed restarts, and it is deterministic from a seed. `ub.py` runs it
with r = 3, 4 (and 5 if needed), recounts the inside edges in Python, deletes exactly those
edges and computes the remaining graph's maximum clique exactly (`max_clique`, a colouring
branch and bound). A separate exhaustive search confirms there is no clique one vertex larger.
It then writes the solution file.

Every one of the 60 files was then checked three ways:

1. `cliquelib.Instance.max_violation`: all bounds, integrality and rows hold (violation 0).
2. `bobi check` (the crate's MPS/AUX reader and `model::Bilevel::max_violation`): violation
   0.00e0 for all 60, with upper objective 3 or 4 and lower objective -3 or -4
   (`runs/bobi_check.txt`).
3. `verify_sol.py`, which follows BOBILib's feasibility checker without using the graph. It
   fixes the leader's w at the file's values, builds the follower's MIP from the raw AUX/MPS
   rows and solves it with SCIP. The optimum equals the file's lower-level objective for all 60
   (`runs/verify_ours.jsonl`). The budget used is at most k in every file.

As a final pass, after `summarize.py` set each file's status, `recheck.py` read every file
back and checked it from scratch (`runs/recheck.jsonl`): no variable missing or extra,
violation 0, at most k deletions, a clique of the remainder whose size equals the objective,
no larger clique (exact search plus an exhaustive test one size up), and an "optimal" status
only where the value meets the proved lower bound. `bobi check` was also re-run, with the
same output (`runs/bobi_check_after_restart.txt`). All 60 pass.

At density 0.7 the best 3-partitions found for the n=60 graphs have 306, 311, 310, 309, 310, 306,
307, 308, 307 and 308 inside edges (i = 1..10). These are the best that tabu search found, not
proven minima. Against k = 309, the best found for instances 2, 3 and 5 is too large. For those
three the certificate is a 4-partition (200 to 202 inside edges), giving value at most 4.

## (c) Lower bounds

**Mantel** (Turán for triangles): a triangle-free graph on n vertices has at most floor(n²/4)
edges. Any remainder G - D with |D| <= k has at least m - k edges, and
m - k = 410, 643, 930 > 400, 625, 900 for n = 40, 50, 60 at d = 0.7. So every remainder has a
triangle and the value is at least 3. This holds at d = 0.9 too.

**Brouwer.** The primary source is A. E. Brouwer, *Some lotto numbers from an extension of
Turán's theorem*, Afdeling Zuivere Wiskunde ZW 152/81, Mathematisch Centrum, Amsterdam, March
1981 (https://ir.cwi.nl/pub/6791). I read the scan. Its Theorem 2 says:

> Let G be a graph on n points such that α(G) ≤ t, and n > 2t. If G has no more than
> T(n,t) + m − 2 edges (where m := ⌊n/t⌋) then V(G) can be written as union of t cliques.
> Moreover, this bound is best possible.

Here T(n,t) = ((m+1)t − n)·C(m,2) + (n − mt)·C(m+1,2) is Turán's minimum for α ≤ t. Taking
complements, with H = Ḡ: H is K_{t+1}-free, e(H) ≥ C(n,2) − T(n,t) − m + 2 = t_t(n) − ⌊n/t⌋ + 2,
and the conclusion "V covered by t cliques of G" becomes "H is t-colourable", meaning t-partite
with parts that may be empty. The hypothesis is **n > 2t, that is n ≥ 2t + 1**.
`lb.py` checks the translation in code: T(n,3) + t_3(n) = C(n,2) and
C(n,2) − (T(n,3) + ⌊n/3⌋ − 2) = t_3(n) − ⌊n/3⌋ + 2.

Later restatements:

* M. Tyomkyn and A. J. Uzzell, *Strong Turán stability*, Electron. J. Combin. 22(3) (2015)
  #P3.9, arXiv:1409.8665. Theorem 1.1 says every K_{r+1}-free graph of order n ≥ r + 3 with at
  least h(n,r) + 1 edges is r-colourable, where h(n,r) = t_{n,r} − ⌊n/r⌋ + 1 for n ≥ 2r + 1.
  They credit Brouwer [ZW 152] as the first explicit proof and cite rediscoveries by
  Hanson–Toft (Ars Combin. 31, 1991) and Kang–Pikhurko (Mat. Stud. 24, 2005).
* K. Amin, J. Faudree, R. J. Gould and E. Sidorowicz, *On the non-(p−1)-partite K_p-free
  graphs*, Discuss. Math. Graph Theory 33 (2013) 9–23, doi:10.7151/dmgt.1654. Theorem 14 gives
  the maximum size of a non-(p−1)-partite *maximal* K_p-free graph for n ≥ p + 2. At p = 4 and
  n = 3q + r this is n²/3 − n/3 + r(r−1)/6 + 1 = t_3(n) − ⌊n/3⌋ + 1. Every K4-free graph lies
  inside a maximal one on the same vertices, and adding edges keeps a graph non-3-partite, so
  this is the same statement. (I read the PDF; the formula's last terms are garbled in the text
  layer, and I fixed them by checking them against t_3.) I did not read Kang–Pikhurko or
  Hanson–Toft myself.

The hypotheses hold for every density-0.9 instance. n ≥ 7 always. m − k = 527, 827 and 1195
reach the thresholds t_3(n) − ⌊n/3⌋ + 2 = 522, 819 and 1182 for n = 40, 50 and 60, with margins
5, 8 and 13. The plain Turán bound alone does *not* suffice: m − k is below t_3(n) = 533, 833 and
1200. At density 0.7 Brouwer does not apply (930 < 1182 for n = 60).

**The argument.** Take a remainder H = G − D with |D| ≤ k and suppose it is K4-free. Then
e(H) ≥ m − k reaches the threshold, so H is 3-partite. Every edge of G inside that partition's
parts is then in D, so |D| ≥ w3(G). If w3(G) > k, this is a contradiction, so the value is at
least 4.

**Proving w3(G) > k (my own runs).** For a partition with part sizes s,
inside_G = sum C(s_i,2) − |Ē| + cut_Ḡ, because every pair inside a part is either an edge or a
non-edge. So inside_G ≤ k needs cut_Ḡ ≤ b(s) = k + |Ē| − sum C(s_i,2). `lb.py` lists every size
vector s1 ≥ s2 ≥ s3 ≥ 0 with b(s) ≥ 0. That is 6 vectors per instance at n = 40 and 50 and 5 at
n = 60, with budgets b ≤ 6. All other size vectors are excluded outright. `lb.py` also checks
the identity on 200 random partitions per instance. Each (instance, size vector) pair, 170 in
all, was then decided by three independent methods:

1. **CP-SAT** (OR-Tools 9.15, 2 workers). This is a feasibility model with clause-defined cut
   indicators, sum of cuts ≤ b(s), and parts of equal size ordered by their smallest vertex.
   Every case was INFEASIBLE, the slowest in 11 s.
2. **SCIP** (pyscipopt 6.3 with SCIP, 1 thread). This is a linear model
   c_uv ≥ ±(x_ui − x_vi) with sum c ≤ b(s) and no symmetry breaking. Every case was infeasible,
   the slowest in 4 s.
3. **`split_bnb.rs`**, a solver-free exact depth-first search with a disjoint-edge lower bound
   and equal-size symmetry breaking, tested against brute force on 59 random small cases.
   Every case was NONE, with at most 3309 nodes.

The logs are in `logs/lb_solver_logs.tar.gz` (340 CP-SAT and SCIP logs), with records in
`runs/lb.jsonl` and `runs/lb_bnb.jsonl`. So w3(G) > k is proved for all 30 density-0.9
instances, and their value is exactly 4.

## (d) The three open instances (60-70-2, -3, -5)

At these instances the value is 3 if and only if τ4(G) ≤ k = 309, where τ4 is the fewest
edges meeting every K4 of G (Mantel already gives ≥ 3). A K4-free graph with
m − k = 930 edges need not be 3-partite, since Brouwer's threshold is 1182. So w3 only bounds
τ4 from above.

Both directions were tried. None of it closes an instance, so all three stay **open with
3 ≤ value ≤ 4**. The upper bound 4 is the 4-partition in the solution file (202, 200 and 202
inside edges, remainder's maximum clique 4).

**Looking for value 3 (a K4-transversal with at most 309 edges).**

| method | 60-70-2 | 60-70-3 | 60-70-5 |
| --- | --- | --- | --- |
| `rcut_tabu`, 3-partitions, 40 runs × 2,000,000 moves (seeds 101 to 103) | 311 in all 40 runs | 310 in all 40 | 310 in all 40 |
| CP-SAT, exact set cover (`k4cover.py`), 2 workers, 1200 s | best 311, bound 266 | not run | not run |
| `k4ls`, weighted hitting-set local search at \|D\| = 309, 1200 s | not run | best 310; never under 34 K4s unhit | best 310; never under 54 unhit |
| `ftabu`, maps onto W5, W7, C̄7 and the Moser spindle | 311 in all 24 runs | 310 in all 32 runs | 310 in all 32 runs |

* The CP-SAT model has one clause per K4 (56,232 of them in 60-70-2) and one row per maximal
  clique Q of size at least 5 (14,995 of them): |D ∩ E(Q)| ≥ C(|Q|,2) − t_3(|Q|). It is
  hinted with the 311-edge 3-partition and minimises |D|. It found nothing better than the
  hint, and its bound reached 266 (log in `logs/k4cover_cpsat_log.tar.gz`).
* `k4ls.rs` is not tied to partitions. It is a NuMVC-style search (Cai, Su, Luo and Sattar,
  JAIR 46, 2013) that treats K4s as sets to hit, fixes |D|, swaps edges in and out, and
  raises the weights of unhit K4s. Calibration: from a random 3-partition it reaches the best
  3-partition value in under a second on 40-70-1 (125 edges; the remainder is 3-colourable,
  so again a 3-partition's inside edges). At
  n = 60 it is weaker than `rcut_tabu`, though: in 160 s it reached only 323 on 60-70-1
  (w3 = 306) and 311 on 60-70-4 (w3 = 309). So its failure on the open instances is weak
  evidence.
* `ftabu.rs` generalises `rcut_tabu` from K3 to any small K4-free pattern graph F. A vertex
  map V(G) → V(F) is scored by the edges of G that land on one pattern vertex or on two
  non-adjacent ones. Deleting those edges leaves a graph that maps homomorphically into F, so
  it is K4-free. Since F is 4-chromatic, the remainder can fail to be 3-partite, and a
  3-partition search can never reach such a remainder. The first run starts from the best 3-partition
  placed on a triangle of F; the others start at random. Each of the 4 patterns got 8 runs of
  2,000,000 moves per instance (4 runs each for W5 and C̄7 on 60-70-2). Every run ended at the best
  3-partition's value. Its best map used at most 4 pattern vertices, and
  `check_transversal.py` recounted the deleted edges in Python and found every remainder
  K4-free *and 3-colourable*. So the search always collapsed back to a 3-partition
  (`runs/ftabu_check.jsonl`). An earlier 20-run attempt of those two 60-70-2 jobs also gave
  311 in all 33 runs it finished. Its output files were lost.

**Looking for value 4 (a proof that τ4(G) > 309).**

| bound on τ4 | 60-70-2 | 60-70-3 | 60-70-5 |
| --- | --- | --- | --- |
| LP, K4 rows only | 206.5 | | |
| LP, K4 rows and maximal-clique rows | 292.36 | 291.53 | |
| cutting-plane LP `k4lp.py`, Turán rows, 4 rounds | 296.58 | 295.99 | run, but its output was lost in the restart |
| the same with Brouwer-strengthened rows, 6 rounds | 296.61 | | |
| CP-SAT dual bound after 1200 s | 266 | | |

`k4lp.py` adds the row |D ∩ E(S)| ≥ e(S) − t_3(|S|) for vertex sets S found by local
search (D ∩ E(S) must make G[S] K4-free, and by Turán such a graph keeps at most t_3(|S|)
edges). The Brouwer version raises the right-hand side to
min(w3(G[S]), e(S) − t_3(|S|) + ⌊|S|/3⌋ − 1) for |S| ≥ 7, with w3(G[S]) computed exactly by
`w3_batch.rs`. Each bound is recomputed in exact rationals from the LP duals. The LPs level
off near 296 to 297, far below the 310 needed. A likely reason: a row from a set S only uses
what G[S] alone forces. For random-like graphs, the best partition beats a random one
(399 inside edges on average for balanced parts at n = 60, against 310 here) by an amount
that grows like |S|^{3/2}. Rows on small or medium sets then cannot add up to the
whole-graph value. Closing these instances would take an exact search on the whole problem,
and none of these methods comes near one. The 60-70-5 LP and the final JSON of the
Brouwer run were lost when the container restarted. The Brouwer run's per-round log survived
(`runs/k4lp_brouwer_60-70-2.rounds.jsonl`).

What the evidence does say: every search stops at the best 3-partition found (311, 310, 310). That
includes the three that are not limited to 3-partite remainders (CP-SAT, `k4ls`, `ftabu`). That fits an asymptotic theorem:
in G(n, p) the largest K_{r+1}-free subgraph is r-partite with probability tending to 1
(Babai, Simonovits and Spencer, J. Graph Theory 14 (1990) for p = 1/2; Brightwell, Panagiotou
and Steger, Random Structures Algorithms 41 (2012) for much smaller p). But n = 60 is small,
and nothing here proves τ4 = w3 for these graphs.

## What is proven by theorem, and what by computation

| claim | instances | rests on |
| --- | --- | --- |
| value ≥ 3 | all 60 | Mantel's theorem plus the exact integer comparison m − k > ⌊n²/4⌋ (`lb.py`; also formalised in `../../lean/Bobi/Clique.lean`, `triangle_survives`) |
| a K4-free remainder would be 3-partite | 30 at d = 0.9 | Brouwer 1981, Theorem 2 (complemented), with n ≥ 7 and m − k ≥ t_3(n) − ⌊n/3⌋ + 2 checked in exact integers (`lb.py`) |
| w3(G) > k, so value ≥ 4 | 30 at d = 0.9 | exact certificate: the size-vector reduction (exact identity) and an explicit branch-and-bound tree for each of the 170 remaining cases, verified by the short, dependency-free checker `../../scripts/check_clique_certificates.py`, which reads the instance files itself (`certificates/`). The same cases were also decided infeasible by CP-SAT, SCIP and `split_bnb.rs` (logs in `logs/lb_solver_logs.tar.gz`), and by HiGHS and `rbnb.c` in the review |
| value ≤ 3 or ≤ 4 | all 60 | computation, checked by anyone in seconds: the solution file's leader deletes the inside edges of an r-partition (at most k of them), so the remainder is r-partite and its clique number is at most r (`cliqueFree_of_partition` in the Lean file). The follower's clique and the exact clique number of the remainder are checked by `cliquelib`, `bobi check` and a SCIP re-solve of the follower |
| 3 ≤ value ≤ 4 | 60-70-2, -3, -5 | the same; no proof of either value |

The infeasibility verdicts behind w3(G) > k no longer need any solver to be trusted.
`certificates/` holds a branch-and-bound tree for each of the 170 cases (134,368 nodes in all,
the largest 9,888). It also holds the checker's log: all 170 pass, in 3 to 4 s. Its README
gives the format, the leaf bound, the symmetry argument and the trusted base: a Python
interpreter and one script with 174 lines of code, plus Brouwer's theorem for the step to value 4.
Before the certificates, each verdict was reproduced by two solvers and by `split_bnb.rs`, about
160 lines of Rust tested against brute force, whose largest case takes 3309 search nodes.

## (e) `bcpins` and `plusbcpins`

These sets hold the same 80 graphs and budgets in two formulations (n = 8, 10, 12, 15), and
BOBILib lists all of them as optimal. `small_exact.py` applies the same arguments to all 160
instances:

* Lower bound from Turán (m − k > t_r(n)) or from Brouwer at r = 2 (bipartite version,
  n ≥ 5) with "no bipartition has ≤ k inside edges" decided by CP-SAT.
* Upper bound from an r-partition (tabu) and the exact clique number of the remainder.

The arguments settle **all 160**. The values are 3 for 71 graphs and 2 for 9. The lower bound
comes from Turán t_2 in 40 cases, Brouwer r = 2 in 31 and Turán t_1 in 9. Every value equals
BOBILib's optimal value (`runs/small_bcpins.jsonl`, `runs/small_plusbcpins.jsonl`). The
lifted rows of `plusbcpins` were all checked to be redundant (section (a)).

## Reproduce

```sh
cd bobilib
ln -s /path/to/bobilib-data data          # collection/ and collection-solutions/
cargo build --release                     # for `bobi check`
S=scripts/clique; R=runs/clique; mkdir -p $R
export BOBILIB_META=/path/to/collection.json   # BOBILib metadata (status, best known value)
rustc -O --edition 2021 $S/rcut_tabu.rs -o $R/rcut_tabu
rustc -O --edition 2021 $S/split_bnb.rs -o $R/split_bnb
python3 -I $S/parse_check.py clique > $R/parse_clique.jsonl           # (a)
python3 -I $S/export_graphs.py clique $R/graphs
python3 -I $S/ub.py clique $R/graphs $R/rcut_tabu results/clique 200000 10 > $R/ub.jsonl   # (b)
python3 -I $S/verify_sol.py clique results/clique > $R/verify_ours.jsonl
for f in results/clique/*.res.json; do b=$(basename $f .res.json); \
  target/release/bobi check data/collection/interdiction/clique-interdiction/clique/$b.mps.gz $f; done
python3 -I $S/lb.py clique $R/lb_logs > $R/lb.jsonl                    # (c) CP-SAT + SCIP
python3 -I $S/lb_bnb.py clique $R/split_bnb $R/complements > $R/lb_bnb.jsonl   # (c) solver-free
for i in 2 3 5; do $R/rcut_tabu $R/graphs/normalClique60-70-$i.txt 3 2000000 40 10$i 309 \
  > $R/deep_tabu_60-70-$i.txt; done                                       # (d)
python3 -I $S/k4cover.py normalClique60-70-2 $R/deep_tabu_60-70-2.txt 1200 \
  $R/k4cover_60-70-2.cpsat.log > $R/k4cover_60-70-2.json
rustc -O --edition 2021 $S/w3_batch.rs -o $R/w3_batch
for i in 2 3; do python3 -I $S/k4lp.py normalClique60-70-$i 3 turan $R/w3_batch $R/k4lp_work \
  > $R/k4lp_turan_60-70-$i.json 2> $R/k4lp_turan_60-70-$i.log; done
python3 -I $S/k4lp.py normalClique60-70-2 6 brouwer $R/w3_batch $R/k4lp_work \
  > $R/k4lp_brouwer_60-70-2.json 2> $R/k4lp_brouwer_60-70-2.log
rustc -O --edition 2021 $S/k4ls.rs -o $R/k4ls; mkdir -p $R/k4ls_runs; C=results/clique/runs
$R/k4ls $R/graphs/normalClique60-70-3.txt $R/deep_tabu_60-70-3.txt 309 1200 1 20 > $R/k4ls_runs/ls_60-70-3_s1.txt
$R/k4ls $R/graphs/normalClique60-70-5.txt $R/deep_tabu_60-70-5.txt 309 1200 2 20 > $R/k4ls_runs/ls_60-70-5_s2.txt
# calibration from random 3-partitions (Python random.Random(seed), one part 0..2 per vertex)
$R/k4ls $R/graphs/normalClique40-70-1.txt $C/k4ls_start_rand3_40_s5.txt 125 20 1 20 > $R/k4ls_runs/calib_40-70-1.txt
python3 -I $S/check_transversal.py normalClique40-70-1 k4ls $R/k4ls_runs/calib_40-70-1.txt
for it in 1:306 4:309; do i=${it%:*}; $R/k4ls $R/graphs/normalClique60-70-$i.txt \
  $C/k4ls_start_rand3_60_s1.txt ${it#*:} 150 1 20 > $R/k4ls_runs/calib_60-70-$i.txt; done
rustc -O --edition 2021 $S/ftabu.rs -o $R/ftabu; mkdir -p $R/ftabu_runs
# one line "i pattern seed runs" per job in results/clique/runs/ftabu_jobs.txt
while read i p s runs; do $R/ftabu $R/graphs/normalClique60-70-$i.txt $p 2000000 $runs $s 309 \
  $R/deep_tabu_60-70-$i.txt > $R/ftabu_runs/ft_60-70-${i}_$p.txt; done < results/clique/runs/ftabu_jobs.txt
for f in $R/ftabu_runs/ft_*.txt; do b=$(basename $f .txt); python3 -I $S/check_transversal.py \
  normalClique$(echo $b | cut -d_ -f2) ftabu $f $(echo $b | cut -d_ -f3); done > $R/ftabu_check.jsonl
python3 -I $S/parse_check.py bcpins > $R/parse_bcpins.jsonl             # (e)
python3 -I $S/parse_check.py plusbcpins > $R/parse_plusbcpins.jsonl
python3 -I $S/small_exact.py bcpins $R/rcut_tabu $R/small_graphs > $R/small_bcpins.jsonl
python3 -I $S/small_exact.py plusbcpins $R/rcut_tabu $R/small_graphs > $R/small_plusbcpins.jsonl
python3 -I $S/summarize.py $R                                            # summary.csv, statuses
python3 -I $S/recheck.py > $R/recheck.jsonl                              # final re-check of all 60
```

`runs/` here holds copies of the run records that the summary and this text are built from
(`bobilib/runs/` itself is not in git). `logs/` holds the solver logs as tarballs, because the
repository ignores `*.log`. Every run used at most 2 threads. The 60-70-2 Turán LP record
comes from an earlier revision of `k4lp.py` with the same Turán rows and separation.

## Independent review (second agent, 2026-10-18)

A second agent re-checked everything above with its own code (`../../scripts/clique/review/`,
records in `review/`). It shares no code with the scripts above. It found no error in any
result. It fixed four wording errors in this README (k in `bcpins`/`plusbcpins` is ceil(m/4),
not floor(m/4); "best 3-partition" became "best 3-partition found" in three places, since those
values are heuristic; the 60-70-5 cutting-plane LP was run and lost, not "not run").

* **Model.** `mymps.py` is a generic MPS/AUX reader. `structure.py` classifies every row by
  its coefficients, not by its name. All 60 instances have the model of section (a), with no
  extra rows. The objective sense is MIN, there are no RANGES, and every column is binary. The
  one leader row is the sum of all w ≤ k, with k = floor(m/4). The leader's cost is +1 on each
  x and 0 elsewhere, and the follower's AUX cost is −1 on each x and 0 on each y
  (`review/structure.jsonl`).
* **Solution files.** `checksol.py` checks every MPS row and the budget. It checks that x is a
  clique of G − D and that ω(G − D) = |x|. ω is computed by two exact searches
  (Bron–Kerbosch with Tomita pivoting, and Carraghan–Pardalos). The remainder is also checked
  to be ω-colourable. All 60 files pass (`review/checksol.jsonl`). `partition_check.py`
  confirms each D is exactly the set of inside edges of an r-partition (60 of 60). BOBILib's own
  60 reference solutions also pass, with values 5 to 13 (`review/checksol_bobilib_ref.jsonl`).
  `bobi verify --generic --time-limit 300` gave 60 PASS. Every follower MILP was solved to
  optimality, with none left unproven (`review/bobi_verify.tsv`).
* **Arithmetic.** `mylb.py arith` checks in exact integers: Mantel for all 60 instances; and,
  for d = 0.9, n > 6 and m − k ≥ t_3(n) − floor(n/3) + 2, with margins 5, 8 and 13. It
  enumerates the same 170 size vectors and budgets as `lb.py` and checks the counting identity
  on 300 random partitions per instance. Brouwer's Theorem 2 was read in the scan
  (https://ir.cwi.nl/pub/6791/6791D.pdf, report p. 1: "Let G be a graph on n points such that
  α(G) ≤ t, and n > 2t. If G has no more than T(n,t) + m−2 edges (where m := ⌊n/t⌋) then V(G)
  can be written as union of t cliques."). It matches Tyomkyn–Uzzell, arXiv:1409.8665v1,
  eq. (1) and Theorem 1.1.
* **w3(G) > k, re-proved two more ways for all 30 d = 0.9 instances.** Both methods differ
  from CP-SAT, SCIP and `split_bnb.rs`:
  1. HiGHS 1.15.1 MIP, 1 thread, on the linear model with an explicit budget row. All 170 cases
     were infeasible, the slowest in 29 s (`review/highs_lb.jsonl`). As a positive control, the
     same model on 40-70-1 finds a 3-partition with 136 ≤ k inside edges
     (`review/controls_highs.jsonl`).
  2. `rbnb.c`, an exact depth-first search. Its lower bound charges each unplaced vertex its
     cheapest part against the vertices already placed, and it uses equal-size symmetry
     breaking. It agrees with brute force on 97,166 decisions over 450 random graphs. On planted
     n = 60 graphs with c = 1, 2, 3, 5 cut edges it finds the planted cut at b = c and nothing at
     b = c − 1 (`review/controls_bnb.txt`). It finds no partition in any of the 170 cases, with
     at most 6191 nodes (`review/bnb_lb.jsonl`).

  All 340 logs in `logs/lb_solver_logs.tar.gz` end infeasible, and each model has 3n + |Ē|
  variables for its graph. Those logs do not print the budget, so the thresholds rest on the
  re-runs above. A local search (`rbnb ls`) bounds w3(G) − k from above by 13 to 17 (n = 40),
  30 to 34 (n = 50) and 52 to 58 (n = 60) (`review/ls_w3.txt`).
* **Open instances.** 3 ≤ value ≤ 4 holds for 60-70-2, -3 and -5. The lower bound is Mantel,
  930 > 900. The upper bound is the solution files, with 202, 200 and 202 deletions and
  ω(G − D) = 4. Of the search and LP evidence in (d), the reviewer reproduced only the K4
  count (56,232) and the K4-only LP value (206.5) for 60-70-2 (`review/k4lp_check.jsonl`).
  The rest rests on the records in `runs/`. The graphs' clique numbers (10 to 13, and 18 to 27)
  were reproduced (`review/omegaG.jsonl`).
* **Not re-checked:** the earlier agent's `minwithin3_lb` logs (not found), the `bcpins` and
  `plusbcpins` results of (e) (their records are internally consistent), and the
  Lean file.

Reproduce the review from `bobilib` (R is any scratch directory, I the `clique` instance
directory):

```sh
V=scripts/clique/review; R=/tmp/review; I=data/collection/interdiction/clique-interdiction/clique
python3 -I $V/structure.py $I $R/graphs > $R/structure.jsonl
python3 -I $V/checksol.py $I results/clique/*.res.json > $R/checksol.jsonl
python3 -I $V/partition_check.py $R/graphs results/clique/*.res.json > $R/partition_check.jsonl
target/release/bobi verify --instances data/collection --generic --time-limit 300 -j 2 results/clique/*.res.json
python3 -I $V/mylb.py $R/graphs arith > $R/mylb_arith.jsonl
python3 -I $V/mylb.py $R/graphs highs $(cd $R/graphs && ls *-90-*.edges | sed 's/.edges//') > $R/highs_lb.jsonl
python3 -I $V/complements.py $R/graphs $R/comp && gcc -O2 -o $R/rbnb $V/rbnb.c
$R/rbnb selftest 1 150; $R/rbnb decide $R/comp/planted_c3.comp 22 20 18 3
python3 -I $V/bnb_driver.py $R/mylb_arith.jsonl $R/rbnb $R/comp > $R/bnb_lb.jsonl
python3 -I $V/k4lp_check.py $R/graphs normalClique60-70-2; python3 -I $V/omegaG.py $R/graphs
for f in $R/graphs/*-90-*.edges; do awk 'NR==1{print $1, $2; next} 1' $f > $R/g.txt; \
  echo "$(basename $f .edges) k=$(head -1 $f | cut -d' ' -f3) $($R/rbnb ls $R/g.txt 8 20000 7)"; done
```
