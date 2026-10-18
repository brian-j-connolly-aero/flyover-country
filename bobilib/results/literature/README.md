# Published results used for the "literature" outcomes

Compiled for this work from the authors' published per-instance data; each
row cites its source. `instances.csv` takes the literature optima from
these files and from Weninger and Fukasawa's answer files (`../kip/`).

* `mkip_literature_status.csv`: the 954 multidimensional knapsack
  interdiction instances (sets imkp and OR). Columns: BOBILib's status and
  value at the time of writing; Lozano, Bergman and Cire's lower bound
  (`dd_lcb2026_lb`) and point (`dd_lcb2026_ub`); Lefebvre and Schmidt's
  point where published; the best published point (`best_known_ub`);
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
