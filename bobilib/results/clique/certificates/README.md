# Checkable certificates for w3(G) > k (clique interdiction, density 0.9)

The value-4 proof for the 30 density-0.9 instances `normalClique{40,50,60}-90-{1..10}`
(`../README.md`, section (c), and the appendix, "Clique interdiction proofs") has three parts:

1. Brouwer's theorem (1981, Theorem 2, complemented): if n >= 7 and
   m - k >= t_3(n) - floor(n/3) + 2, every K4-free remainder G - D with |D| <= k is 3-partite.
2. w3(G) > k: every partition of V into 3 parts has more than k edges of G inside parts.
3. A solution file deleting the inside edges of a 4-partition (value <= 4).

Part 2 used to rest on the verdicts of five programs (CP-SAT, SCIP, `split_bnb.rs`, HiGHS and
`rbnb.c`). They print "infeasible" but nothing a reader can check. This directory replaces them
with an explicit branch-and-bound tree for each of the 170 cases and a short checker,
`../../../scripts/check_clique_certificates.py`, that shares no code with the prover or with the
rest of the project.

## Result

**All 30 instances and all 170 cases are certified** (`check.log`, `RESULT PASS`).

| group | instances | cases per instance (budgets b) | tree nodes | leaves | largest tree |
| --- | ---: | --- | ---: | ---: | --- |
| n=40, k=175, h=78 | 10 | 6 (0, 1, 2, 4, 5, 6) | 66,810 | 44,420 | 4,832 nodes (40-90-5, sizes (14,13,13)) |
| n=50, k=275, h=123 | 10 | 6 (0, 1, 2, 4, 5, 6) | 54,906 | 36,464 | 9,888 nodes (50-90-2, sizes (17,17,16)) |
| n=60, k=398, h=177 | 10 | 5 (1, 2, 2, 4, 5) | 12,652 | 8,248 | 1,075 nodes (60-90-1, sizes (21,20,19)) |
| total | 30 | 170 | 134,368 | 89,132 | |

h is the number of non-edges of G. The checker takes 3 to 4 s for all 30 instances on one core
of a shared machine. The certificates take 30 KB (gzip). The simple bound below was enough, so no
stronger bound was needed.

The checker also recomputes, from the rows of each file, n, m, k and h. It confirms Brouwer's
hypothesis in integers, with margins 5, 8 and 13 for n = 40, 50, 60. It enumerates the sorted
size vectors itself (154, 234 and 331 of them) and keeps those with b(s) >= 0. Its case list is
the appendix table exactly: 6, 6 and 5 vectors per instance, the same in all 10 graphs of a
group, with the same budgets (`selftest.log`, test (F)).

**Not certified here.** These parts are outside this directory's claim:

* Brouwer's theorem itself, which turns part 2 into "value >= 4". It is a published theorem,
  quoted and cross-checked against later restatements in `../README.md`. No program here proves it.
* The model lemma: if the rows have the shape the checker verifies, the follower's value is
  omega(G - D). The proof is a few lines of pen and paper (appendix, "Follower's problem").
* The upper bound 4 (part 3). Those are the solution files, checked separately (`../README.md`,
  section (b)).
* The three open density-0.7 instances. Nothing here applies to them, since Brouwer's
  hypothesis fails at density 0.7.

In the review's terms, w3(G) > k is now an **exact certificate**: an exhaustive integer
computation checked by a short independent program. Together with the cited theorem it gives
value >= 4.

## What the checker proves

For each instance file, a PASS means the following holds. It is computed in exact integer
arithmetic, from the MPS and AUX files alone.

1. **The model.** BOBILib's reading of the files is this: the leader minimises the MPS objective;
   the follower owns the AUX columns and minimises the AUX objective subject to the AUX rows.
   Under that reading the file is edge clique interdiction on a graph G = (V, E):
   * every column is binary (`BV`), and the file has no `RANGES`, `OBJSENSE` or other extra
     section;
   * V is the set of follower columns x with follower cost -1 and leader cost +1; every other
     cost is 0;
   * the rows are exactly: one leader row, the sum of all leader columns w <= k; and follower
     rows. For each vertex pair uv there is exactly one of x_u + x_v <= 1 (uv not in E) or
     x_u + x_v - y <= 1 (uv in E, with its own y). For each such y there is exactly one row
     w + y <= 1, with its own w.

   The rows are classified by their coefficients, not by their names. Names serve only as
   vertex labels.
2. **The arithmetic.** n >= 7 and m - k >= t_3(n) - floor(n/3) + 2.
3. **The claim.** w3(G) > k, by the reduction and the trees below.

## The reduction to part-size cases

Let H be the complement of G (the x_u + x_v <= 1 rows) and h = |E(H)|. A partition with part
sizes s = (s1, s2, s3) has

    inside_G = C(s1,2) + C(s2,2) + C(s3,2) - h + cut_H.

Every pair inside a part is an edge of G or of H, and h - cut_H edges of H lie inside parts.
Relabelling the parts changes neither count, so take s1 >= s2 >= s3 >= 0. With
b(s) = k + h - sum C(s_i,2), we have inside_G <= k iff cut_H <= b(s). If b(s) < 0 no partition
with sizes s qualifies, because cut_H >= 0. For the others, the certificate must show
cut_H > b(s) for every partition with exactly these sizes. This is the appendix's
"Part-size reduction". `cert_selftest.py` test (A) confirms it against brute force (below).

## Certificate format

There is one file per instance, `NAME.cert.jsonl.gz`: gzip-compressed text, one JSON object per
line.

* **Line 1 (header):** `format` ("bobilib-clique-w3-tree"), `version` (1), `instance`, `n`, `m`,
  `k`, `nonedges` (h) and `cases`, a list of `[s1, s2, s3, b]`. The checker compares n, m, k
  and h with its own values.
* **One line per case:** `sizes` [s1, s2, s3], `budget` b, `order` (a permutation of the vertex
  column names, such as `"x29"`), `nodes`, `leaves`, and `tree`.

`tree` is the search tree in preorder, one character per node: `1` for a branch, `0` for a leaf.
A node at depth d is a placement of the first d vertices of `order`. Its children are not
listed; the checker derives them. A branch node at depth d < n gets one child for each part
p = 0, 1, 2, in that order, that is *allowed* for vertex v_d:

* part p holds fewer than s_p vertices; and
* if part p is empty and s_{p-1} = s_p, then part p-1 is not empty (equal parts are opened in
  index order).

A certificate therefore cannot leave a branch out. If its string has too few characters, too
many, or a `1` at depth n, the checker rejects it. Example: the first case of `normalClique40-90-1`
has sizes (16, 13, 11), b = 0, 115 nodes and 77 leaves. Its tree begins with sixteen `1`s.

## The leaf bound

At a leaf the checker recomputes

    LB = cut_H(placed vertices)
         + sum over unplaced u of  min over parts p with room of
           |{placed H-neighbours of u that are not in p}|

and requires LB > b(s). A part "has room" if it holds fewer than s_p vertices. LB is at most
cut_H of every completion with sizes s, for two reasons:

* Each unplaced u ends up in some part that has room now. All of its edges to placed vertices in
  other parts are then cut.
* The edges counted for u join u to placed vertices. They are distinct for different u, and
  distinct from the edges among placed vertices.

At depth n, LB is the exact cut. No other bound is used. It is the bound that `rbnb.c` and
`split_bnb.rs` prune with (`split_bnb.rs` writes it as placed degree minus the largest count), and
it already gives trees of at most 9,888 nodes. So no strengthening was needed.

## The symmetry argument

The sizes are sorted, so parts of equal size are adjacent in the index order. Take any
partition with sizes s. Within each group of equal-size, non-empty parts, relabel the parts so
that their first vertices (in the certificate's order) come in increasing order. This keeps the
sizes and cut_H. The relabelled, *canonical* partition never puts a vertex into a forbidden empty
part: when the first vertex of part p is placed, part p-1 already holds its own first vertex,
which comes earlier. So every canonical partition follows a root-to-leaf path of the tree.
That leaf shows cut_H > b(s), and so the same holds for every partition with sizes s. Empty parts
never take a vertex (they have no room), so they need no rule. Test (B) checks on random graphs
that the rule loses no minimum.

## Trusted base

* A Python 3 interpreter: its integers, `fractions.Fraction` (used only to read numbers such as
  `1.` and `-1.` as integers), and `json`, `gzip`, `os`, `sys` and `time`.
* `scripts/check_clique_certificates.py`: 174 non-blank lines of code and a 37-line docstring
  that states what it proves. It imports no project code. Run it without `-O`, because its
  checks are `assert` statements; it refuses to run under `-O`.
* BOBILib's file conventions as stated in item 1, and the model lemma.
* For the conclusion "value >= 4" only: Brouwer's theorem.

**Not trusted:** the prover (`scripts/clique/cert_prove.rs` and `cert_prove.py`), `cliquelib.py`,
and every solver. A faulty prover could only produce a certificate that the checker rejects.

## Tests (`selftest.log`, `scripts/clique/cert_selftest.py`)

Every test compares against brute force over all 3^n labelled partitions of small random
graphs. That brute force shares no code with the prover or the checker.

* **(A) Reduction.** 450 random graphs (n = 3 to 9). The counting identity holds for every
  labelled partition, and relabelling parts keeps the minimum cut. For every k from -1 to C(n,2),
  "w3(G) > k" agrees with the size-vector test, in 8,243 decisions with 0 mismatches.
* **(B) Symmetry and bound.** 150 random graphs. The canonical minimum equals the true minimum
  for every size vector. The leaf bound never exceeds the best completion, in 3,824 random partial
  placements.
* **(C) End to end.** 150 random instance files written in BOBILib's clique format (n = 7 to 12,
  k close to w3, 441 cases). The prover wrote a certificate exactly when brute force says
  w3 > k (77 instances), and the checker passed exactly then. The other 73 instances got no
  certificate, and the checker failed them. 74 of the instances satisfy Brouwer's hypothesis;
  the checker without `--no-brouwer` gives the same verdicts on them. (`--no-brouwer` is a test
  option that skips item 2.)
* **(D) Forgeries.** For each of the 73 false claims, a certificate was built from trees proved
  for a smaller budget, or from a bare root leaf where the minimum cut is 0. All 73 were
  rejected.
* **(E) Mutations.** Seven edits of `normalClique40-90-1`'s files were all rejected: budget k+1,
  budget k-1, one non-edge row deleted, a y coefficient changed to -2, a RANGES section added,
  a follower cost changed to -2, and a BV bound replaced. Of 60 random edits of its
  certificate, 56 were rejected (flipped, dropped or appended tree characters, raised budgets,
  deleted cases). The other 4 swapped two early vertices in a vertex order. They are still valid
  proofs: in a small tree, for example at b = 0, two vertices can play the same role, and the
  checker recomputed every bound for the new order.
* **(F)** The case lists match the appendix table.

## Reproduce

From `bobilib/`, with `data` linked to the BOBILib collection:

```sh
W=/tmp/certwork; mkdir -p $W
rustc -O --edition 2021 scripts/clique/cert_prove.rs -o $W/cert_prove
python3 -I scripts/clique/cert_prove.py $W/cert_prove $W results/clique/certificates 50   # about 6 s
python3 -I scripts/check_clique_certificates.py data/collection/interdiction/clique-interdiction/clique \
  results/clique/certificates                                                              # check.log
python3 -I scripts/clique/cert_selftest.py $W/cert_prove $W/selftest 450                   # selftest.log, about 1 min
```

The prover tries 50 vertex orders per case and keeps the smallest tree. The first order is
greedy: the most already-ordered H-neighbours first, then the highest degree, then the lowest
index. The other 49 break ties at random, with a fixed seed. With the first order alone, the
170 trees have 179,554 nodes in all, the largest 9,914. The files are deterministic: gzip with
mtime 0, and a rerun reproduces them byte for byte.
