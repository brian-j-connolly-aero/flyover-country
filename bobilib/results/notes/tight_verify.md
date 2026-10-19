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
the record, the 26 superseded files kept under `tolerance/`: nine in
`general-coupled/tolerance/` and 17 in `general/tolerance/` (15 loose, two
in `large_points.tol.tar.gz`). The time limit was 1,200 s per follower
MILP. No solve reached it; the whole run took about 4 minutes on 2
threads. `tight_verify.tsv` has one row per file.

## Counts

| folder | files | pass | benchmark-accepted | other failures |
| --- | ---: | ---: | ---: | ---: |
| `ss` | 16 | 16 | 0 | 0 |
| `kip` | 583 | 583 | 0 | 0 |
| `or` | 690 | 690 | 0 | 0 |
| `clique` | 60 | 60 | 0 | 0 |
| `network` | 36 | 36 | 0 | 0 |
| `general` (loose files) | 55 | 55 | 0 | 0 |
| `general/large_points.tar.gz` | 9 | 9 | 0 | 0 |
| `general-coupled` | 29 | 29 | 0 | 0 |
| **all current files** | **1,478** | **1,478** | **0** | **0** |
| `general-coupled/tolerance` (superseded) | 9 | 1 | 8 | 0 |
| `general/tolerance` (superseded) | 17 | 0 | 17 | 0 |

"Other failures" counts every other reason a file could fail: a row, bound
or integrality violation above 1e-6, a stated objective that is not the
point's, a follower value that BOBILib's own test rejects, an unproven
follower optimum, or a file that could not be read. There are none.

* All 1,385 interdiction files (`ss`, `kip`, `or`, `clique`, `network`)
  pass. Their followers have integer objectives, solved exactly or to a
  zero gap.
* All 64 `general` files and all 29 `general-coupled` files pass. In both
  folders the files that had used the follower's tolerance were re-derived
  under the strict tolerance with `bobi coupled-tighten` and replaced:
  nine coupled files (`general-coupled/README.md`, "Strict tolerance") and
  17 `general` files (`general/README.md`, "Strict tolerance"). Of the 26
  files they replace, 25 are benchmark-accepted; the 26th, neos17_50_50's
  old file, passes (its slack of 1.6e-7 is below 1e-6).
* The 17 superseded `general` files came from `bobi general`, whose
  optimistic tie-break let the follower's value sit up to 1e-7 (1 + |phi|)
  above its optimum. Their followers sat 5.0e-8 to 1.02e-7 relative above
  the optimum (absolute 6.3e-6 to 1,000). The re-derived points keep each
  file's leader values; their followers sit at most 1.3e-9 above the
  proven bound, and each leader objective is worse than the old one by the
  old follower slack (6.3e-6 to 1,000; at most 1.0e-7 relative). The
  general README has the table of old and new values.
* Two current files still carry a slack above 1e-8 relative, under the
  tight test's 1e-6 absolute, so they pass: neos17_0_100 (1.6e-7) and
  neos-5188808-nattai_0_100 (7.1e-7). Every other current file's follower
  is at most 1.3e-9 (absolute) above the proven bound on its optimum.
* None of the HPR-bound optima (`general/hpr_optimal.csv`) was among the
  re-derived files; their files are unchanged and pass.

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
# the superseded files, renamed to .res.json in temporary directories
mkdir -p /tmp/tolc /tmp/tolg /tmp/tolx
for f in results/general-coupled/tolerance/*.tol; do cp $f /tmp/tolc/$(basename $f .tol); done
for f in results/general/tolerance/*.tol; do cp $f /tmp/tolg/$(basename $f .tol); done
tar xzf results/general/tolerance/large_points.tol.tar.gz -C /tmp/tolx
for f in /tmp/tolx/*.tol; do mv $f /tmp/tolg/$(basename $f .tol); done
./target/release/bobi verify --instances data/collection --tight --time-limit 1200 -j 2 \
    /tmp/tolc > tv_tol_coupled.tsv
./target/release/bobi verify --instances data/collection --tight --time-limit 1200 -j 2 \
    /tmp/tolg > tv_tol_general.tsv
python3 -I scripts/tight_verify_summary.py results/notes/tight_verify.tsv \
    ss=tv_ss.tsv kip=tv_kip.tsv or=tv_or.tsv clique=tv_clique.tsv network=tv_network.tsv \
    general=tv_general.tsv general/large_points.tar.gz=tv_general-large.tsv \
    general-coupled=tv_general-coupled.tsv general-coupled/tolerance=tv_tol_coupled.tsv \
    general/tolerance=tv_tol_general.tsv
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
