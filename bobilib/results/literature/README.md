# Published results used for the "literature" outcomes

Compiled for this work from the authors' published per-instance data; each
row cites its source. `instances.csv` takes the literature optima from
these files and from Weninger and Fukasawa's answer files (`../kip/`).

* `mkip_literature_status.csv`: the 954 multidimensional knapsack
  interdiction instances (sets imkp and OR). Columns: BOBILib's status and
  value at the time of writing; Lozano, Bergman and Cire's lower bound
  (`dd_lcb2026_lb`) and point (`dd_lcb2026_ub`); Lefebvre and Schmidt's
  point where published (`lefebvre_schmidt_ub`, see below) and, for
  reference only, a lower value of theirs from a run stopped at the time
  limit (`lefebvre_schmidt_ub_timelimit`); the best published point
  (`best_known_ub`, the minimum of Lozano et al.'s point, Lefebvre and
  Schmidt's counted point, Fischetti et al.'s optimum and BOBILib's value);
  whether the instance is proven optimal in the literature; and Fischetti,
  Ljubić, Monaci and Sinnl's 2019 optimum for imkp (`flms2019_opt`).
* `miblp_literature_status.csv`: the 266 general (MIPLIB-derived and other)
  instances BOBILib lists as feasible or open. Columns: best published
  upper and lower bounds with their sources (Fischetti et al. 2017 MIX++
  tables; Vasquez, Lozano and van Hoeve 2025; Lozano, Bergman and Cire's
  solution files; an earlier BOBILib snapshot), whether the instance is
  closed in the literature, every source's values (`sources`), and values
  re-checked with HiGHS.

Lozano, Bergman and Cire's own `optimal.csv` (their repository) is not
copied here; `instances.csv` records which instances it closes.

Lefebvre and Schmidt: H. Lefebvre and M. Schmidt, A Dantzig-Wolfe
single-level reformulation for mixed-integer linear bilevel optimization:
exact and heuristic approaches, Optimization Online preprint (June 2025),
with per-instance run files on the first author's web page
(henrilefebvre.com, open-science page of the paper). Their MKIP values are
the column `lefebvre_schmidt_ub`, written by
`python3 -I scripts/ls_mkip_points.py RUN_DIR mkip_literature_status.csv`
from their four run files (`results-MILP.csv`, `results-MILP-first.csv`,
`results-PADM.csv`, `results-PADM-interdiction.csv`) with the rule of their
own analysis script: a run counts as having found a point only if it ended
before the time limit of 3,600 s, and a PADM run only if its status is
`Feasible`. Rows that hit the limit can carry relaxation values that no
bilevel feasible point has (`../notes/lefebvre_schmidt.md`); for 21 OR
instances such a value lies below the best counted published point, for 10
of them also below Lozano, Bergman and Cire's lower bound. These are kept in
`lefebvre_schmidt_ub_timelimit` and enter no comparison. The counted values
are published values, not verified here. Their run files also list points for
general instances; the four that bear on whether a point of this work is the
first one anywhere (rmine6_0_100, rmine6_50_50, ex10_0_100 and
neos-631710_0_100) are recorded in `paper/scripts/build_data_package.py`
(`EXTRA_PUBLISHED`) and in the notes of `instances.csv`. ex10_0_100 is now
optimal at the MIPLIB bound of its HPR (`general/hpr_optimal.csv`); their
MILP run files report the same optimum, which is not counted as a published
optimum because their paper does not state it. `notes/lefebvre_schmidt.md`
checks all their run files against this package.
