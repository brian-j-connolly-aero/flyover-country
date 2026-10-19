# Tight verification of every solution file

Every solution file under `results/` was re-checked with `bobi verify
--tight`, which keeps BOBILib's checks but holds the follower to its
optimum:

* rows, bounds and integrality within 1e-6 (absolute), as in the normal
  check;
* the file's stated objective equal to the point's, as in the normal check;
* the follower's value within max(1e-6, 1e-9 |phi|) of every follower
  optimum phi the solver's proof allows, instead of BOBILib's relative 1e-4.
  The follower's problem at the file's leader values is solved by the exact
  knapsack oracle (`kip`, `ss`: one follower row), the multidimensional
  knapsack MILP (`or`), or a HiGHS MILP built from the model (`clique`,
  `network`, `general`, `general-coupled`), with zero relative gap and
  absolute gap 1e-9 at HiGHS's default feasibility tolerance (1e-7); for an
  integer-valued follower objective the bound is rounded up.

The same solve also gives BOBILib's verdict. A file that fails the tight
test but passes BOBILib's relative 1e-4 is listed as **benchmark-accepted**
(class (iii) of the paper: it passes BOBILib's tolerance-based checker
only).

The run covered all 1,478 current files (1,469 loose ones and the nine in
`general/large_points.tar.gz`, extracted to a temporary directory) and, for
the record, the nine superseded coupled files in
`general-coupled/tolerance/`. The time limit was 1,200 s per follower MILP.
No solve reached it; the whole run took about 5 minutes on 2 threads.
`tight_verify.tsv` has one row per file.

## Counts

| folder | files | pass | benchmark-accepted | other failures |
| --- | ---: | ---: | ---: | ---: |
| `ss` | 16 | 16 | 0 | 0 |
| `kip` | 583 | 583 | 0 | 0 |
| `or` | 690 | 690 | 0 | 0 |
| `clique` | 60 | 60 | 0 | 0 |
| `network` | 36 | 36 | 0 | 0 |
| `general` (loose files) | 55 | 40 | 15 | 0 |
| `general/large_points.tar.gz` | 9 | 7 | 2 | 0 |
| `general-coupled` | 29 | 29 | 0 | 0 |
| **all current files** | **1,478** | **1,461** | **17** | **0** |
| `general-coupled/tolerance` (superseded) | 9 | 1 | 8 | 0 |

"Other failures" counts every other reason a file could fail: a row, bound
or integrality violation above 1e-6, a stated objective that is not the
point's, a follower value that BOBILib's own test rejects, an unproven
follower optimum, or a file that could not be read. There are none.

* All 1,385 interdiction files (`ss`, `kip`, `or`, `clique`, `network`)
  pass. Their followers have integer objectives, solved exactly or to a
  zero gap.
* All 29 current `general-coupled` files pass. Nine of them were re-derived
  under the strict tolerance for this check (`general-coupled/README.md`,
  "Strict tolerance"). Of the nine files they replace, eight are
  benchmark-accepted. The ninth, neos17_50_50's old file, passes: its slack
  of 1.6e-7 is below 1e-6.
* 17 of the 64 `general` files are benchmark-accepted, and the reason is the
  same in every case. These points come from `bobi general`, whose
  optimistic tie-break let the follower's value sit up to 1e-7 (1 + |phi|)
  above its optimum, the same slack the old coupled files used. Their
  followers sit 5.0e-8 to 1.02e-7 relative above the optimum (absolute
  6.3e-6 to 1,000). Two more `general` files carry such slack but stay
  under 1e-6 absolute, so they pass: neos17_0_100 (1.6e-7) and
  neos-5188808-nattai_0_100 (7.1e-7). That makes 19 files more than 1e-8
  relative above the optimum, 17 of them over the tight tolerance.

## The 17 benchmark-accepted files

All of them are in `results/general`, and none has coupling rows. The
"strict value" column is a dry run of `bobi coupled-tighten` at each
file's own leader values: the follower is solved with zero relative gap,
then the optimistic tie-break is solved with the follower's value capped
1e-9 (1e-7 where the solver found no point with 1e-9) above its optimum and
every leader variable fixed. All 17 such points pass the strict
certificate. The files in `results/general` were not replaced; the dry-run
points are not part of the package.

| instance | leader objective (file) | follower slack (absolute) | relative | strict value | worse by |
| --- | ---: | ---: | ---: | ---: | ---: |
| binkar10_1_0_100 | 5006810.579955 | 0.500 | 1.0e-7 | 5006811.080028 | 0.500 |
| cmflsp50-24-8-8_0_100 | 59480213.400598 | 1.253 | 1.0e-7 | 59480214.654200 | 1.254 |
| core2536-691_0_100 | 9999999000.000046 | 1000 | 1.0e-7 | 10000000000 | 1000 |
| cost266-UUE_0_100 | 35801080.956898 | 1.043 | 1.0e-7 | 35801082 | 1.043 |
| exp-1-500-5-5_0_100 | 912510.912065 | 0.0879 | 1.0e-7 | 912511.000000 | 0.0879 |
| ic97_potential_0_100 | 4002.999965 | 3.54e-5 | 1.0e-7 | 4003.000000 | 3.54e-5 |
| n4-3_0_100 | 34802892.519897 | 3.480 | 1.0e-7 | 34802896 | 3.480 |
| n5-3_0_100 | 30001631.999933 | 3.000 | 1.0e-7 | 30001635 | 3.000 |
| n9-3_0_100 | 50404627.959837 | 5.040 | 1.0e-7 | 50404633 | 5.040 |
| neos-2978193-inde_0_100 | 61.453995 | 6.33e-6 | 1.0e-7 | 61.454001 | 6.33e-6 |
| pg5_34_0_100 | -11516.287042 | 1.83e-4 | 1.0e-7 | -11516.286859 | 1.83e-4 |
| pg_0_100 | -5364.259407 | 2.02e-4 | 1.0e-7 | -5364.259205 | 2.02e-4 |
| seymour1_0_100 | 959.535976 | 6.87e-5 | 1.0e-7 | 959.536045 | 6.87e-5 |
| tr12-30_0_100 | 201018.990736 | 9.26e-3 | 1.0e-7 | 201019.000000 | 9.26e-3 |
| uct-subprob_0_100 | 827.999932 | 6.83e-5 | 1.0e-7 | 828.000000 | 6.83e-5 |
| neos-873061_0_100 (large) | 258644.394024 | 0.0259 | 1.0e-7 | 258644.419889 | 0.0259 |
| netdiversion_0_100 (large) | 19850249 | 1.000 | 5.0e-8 | 19850250 | 1.000 |

Follower slack is the file's follower value minus the proven bound on the
follower's optimum at the file's leader values (the follower minimises).
"Worse by" is the strict value minus the file's value. In all 17 it equals
the follower's slack to within 0.02%: in these instances the leader's
objective moves one for one with the follower's. None of the HPR-bound
optima (`general/hpr_optimal.csv`) is among the 17.

## Commands

```sh
cd bobilib
mkdir -p /tmp/large && tar xzf results/general/large_points.tar.gz -C /tmp/large
for d in ss kip or clique network general general-coupled; do
  ./target/release/bobi verify --instances data/collection --tight --time-limit 1200 -j 2 \
      results/$d > tv_$d.tsv
done
./target/release/bobi verify --instances data/collection --tight --time-limit 1200 -j 2 \
    /tmp/large > tv_general-large.tsv
# the superseded coupled files, renamed to .res.json in a temporary directory
mkdir -p /tmp/tol && for f in results/general-coupled/tolerance/*.tol; do
  cp $f /tmp/tol/$(basename $f .tol); done
./target/release/bobi verify --instances data/collection --tight --time-limit 1200 -j 2 \
    /tmp/tol > tv_tolerance.tsv
python3 -I scripts/tight_verify_summary.py results/notes/tight_verify.tsv \
    ss=tv_ss.tsv kip=tv_kip.tsv or=tv_or.tsv clique=tv_clique.tsv network=tv_network.tsv \
    general=tv_general.tsv general/large_points.tar.gz=tv_general-large.tsv \
    general-coupled=tv_general-coupled.tsv general-coupled/tolerance=tv_tolerance.tsv
# dry run of the strict re-derivation for one benchmark-accepted file
sh scripts/tighten_coupled.sh /tmp/strict 600 results/general/pg_0_100.res.json
```

Other tolerances: `--follower-abs-tol` and `--follower-rel-tol` (which imply
`--tight`).

## `tight_verify.tsv`

One row per file, tab separated: `folder`, `name`, `class` (`pass`,
`benchmark-accepted`, `follower-not-optimal`, `row-violation`,
`objective-mismatch`, `unproven`, `error`, `no-solution`), `reason`, then
the columns of `bobi verify --tight` (`result`, `max_violation`,
`violation_at`, `objective`, `file_objective`, `follower_optimal` (tight
verdict), `follower_bobilib` (BOBILib's verdict, same solve), `phi_sol`,
`phi_check`, `phi_bound`, `follower_slack` (phi_sol - phi_bound),
`follower_rel_slack` (slack / (1e-10 + |phi_bound|)), `oracle`, `seconds`,
`note`).
