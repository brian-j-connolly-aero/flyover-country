# refs.bib: what each entry supports

One line per BibTeX key in `refs.bib` (74 entries), grouped as in the file.
"Checked" says how the bibliographic data were confirmed. "Crossref" means the
DOI's Crossref record matched authors, title, volume, issue, pages and year.

## The library and the solvers behind its statuses

- `thurauf2026bobilib`: BOBILib itself (2,654 instances, file format, statuses from MibS and MIX++ with a 1 h limit, families and their sources, 1e-4 tolerances). MPC 18(3) 819-876 (2026), online 21 Mar 2026. Checked: Crossref and the Springer landing page.
- `bobilib_website`: bobilib.org, the source of the status data, solution files and feasibility checker used here (accessed 18 Oct 2026). Checked: site title fetched.
- `tahernejad2020mibs`: MibS, one of BOBILib's two solvers; the general branch-and-cut framework. Checked: Crossref.
- `mibs_software`: the MibS code (BOBILib ran version 1.2.2). Authors and year as in BOBILib's own reference list. Checked: URL as cited by BOBILib and Tahernejad-Ralphs; GitHub is not reachable from this machine.
- `denegre2009branch`: first branch-and-cut for integer bilevel programs; history in the introduction. Checked: Crossref and the Springer page (editors, ORCS vol. 47).
- `denegre2011thesis`: DeNegre's thesis; source of the inter-assig, inter-kp and denegre sets. Checked: PDF title page (Lehigh University, May 2011).
- `fischetti2017new`: MIX++ (BOBILib's second solver); source of xularge; air03-0.100000 solved there. Checked: Crossref.
- `mixpp_software`: page distributing the MIX++ binary and the instance sets. Checked: page fetched.
- `fischetti2016intersection`: intersection cuts (IPCO 2016); source of the miplib3 set. Checked: Crossref and Springer page (LNCS 9682, editors Louveaux and Skutella).
- `fischetti2018intersection`: journal version; also a source of the miplib3 set. Checked: Crossref.

## Knapsack, multidimensional knapsack and generalized interdiction

- `fischetti2019interdiction`: interdiction games and monotonicity; source of the imkp and OR sets; solved all 144 imkp (52 counted as literature optima); interdiction cuts behind the MKIP master. Checked: Crossref.
- `fischetti2018dynamic`: generalized interdiction set, the kp set (450, item classes 6 = subset sum and 7 = even-odd subset sum), plusbcpins and the 60 clique instances. Checked: Crossref.
- `caprara2014complexity`: Sigma_2^p-completeness of bilevel knapsack (why KIP is hard). Checked: Crossref.
- `caprara2016bilevel`: the KIP model and the cclw set. Checked: Crossref.
- `dellacroce2020exact`: the DCS exact algorithm for KIP (prior state of the art). Checked: Crossref.
- `weninger2023fast`: Weninger-Fukasawa, IPCO 2023 (LNCS 13904, editors Del Pia and Kaibel). Checked: Crossref and Springer page.
- `weninger2025fast`: journal version, Math. Program. 210 (2025) 847-879; 376 KIP literature optima; the online lower bound and greedy heuristic used here. Checked: Crossref.
- `weninger_bkpsolver`: their code and answer files (source of the 376 per-instance optima). Checked: URL printed in their paper; repository cloned earlier in this project.
- `lozano2026network`: Lozano, Bergman, Cire, decision-diagram network relaxations ("lbc"/"LBC" in the scratch notes): 87 OR + 4 generalized + 5 MIPLIB 3 literature optima and the published OR bounds. Operations Research, Articles in Advance, online 26 Aug 2026, no volume yet. The arXiv version (2407.18193) had the title "... Discrete Bilevel Optimization ...". Checked: Crossref and the repository README.
- `lozano_dbo_repo`: their solution files (optimal and improved BOBILib points) read by the reconciliation. Checked: repository cloned earlier; README names the paper and DOI.
- `vasquez2025single`: Vasquez, Lozano, van Hoeve, decision-diagram single-level reformulation; p0282-0.500000 optimum. Math. Program., online first 16 Dec 2025, no volume yet. Checked: Crossref and Springer page.
- `martello1999dynamic`: the knapsack instance generator behind cclw and kp (item classes), and the combo algorithm. Checked: Crossref.
- `dantzig1957discrete`: the greedy LP (Dantzig) bound used in the exact knapsack oracle (`src/kp.rs`). Checked: Crossref.
- `martello1990knapsack`: standard knapsack reference (LP bound, dynamic programming). Book, no DOI; ISBN 0-471-92420-2. Checked: library catalogue records.
- `chu1998genetic`: the Chu-Beasley multidimensional knapsacks underlying the 810 OR instances. Checked: Crossref.
- `beasley1990orlibrary`: OR-Library, where the Chu-Beasley instances are distributed. Checked: Crossref.
- `khuri1994zero`: SAC-94 multidimensional knapsack suite underlying imkp. Checked: Crossref.

## Clique and network interdiction

- `tang2016class`: Tang-Richard-Smith; the bcpins edge clique interdiction instances. Checked: Crossref (online 2015, issue 2016).
- `furini2019maximum`: maximum (vertex) clique interdiction, related work. Checked: Crossref.
- `furini2021branch`: exact branch and cut for the edge interdiction clique problem (BOBILib's clique family is this problem); related work and its note on the n = 40-60 instances. Checked: Crossref.
- `baggio2021multilevel`: multilevel critical node (attack-protect) problem; source of inter-fire. Checked: Crossref.
- `smith2020survey`: survey of network interdiction; applications in the introduction. Checked: Crossref.
- `brown2006defending`: defending critical infrastructure; defence applications in the introduction. Checked: Crossref.

## Bilevel optimization background and other methods

- `stackelberg1934marktform`: the leader-follower game. The entry is the 2011 English translation (*Market Structure and Equilibrium*, Springer, doi 10.1007/978-3-642-12586-7), with the 1934 original in the note, so that the DOI prints like every other DOI. Checked: Crossref (title, author, 2011, Springer Berlin Heidelberg) and the Springer page of the translation; publisher and year of the original from catalogue/secondary records.
- `moore1990mixed`: first MIBLP branch and bound; the high point relaxation. Checked: Crossref.
- `jeroslow1985polynomial`: complexity of multilevel programs (Sigma_2^p). Checked: Crossref.
- `dempe2002foundations`: textbook on bilevel programming (background). Checked: Crossref and Springer page.
- `kleinert2021survey`: survey of MIP techniques in bilevel optimization (background, optimistic convention). Checked: Crossref.
- `kleinert2021computing`: PADM; source of the miplib2010 and miplib2017 bilevel sets; the `padm` method of the coupled portfolio. Checked: Crossref.
- `mitsos2010global`: value-function (bounding) cuts, the `vcut` method of the coupled portfolio. Checked: Crossref.
- `lozano2017value`: value-function-based exact approach, also behind `vcut`. Checked: Crossref.
- `xu2014exact`: Xu-Wang algorithm; source of the xuwang set. Checked: Crossref.
- `ozaltin2017branch`: Ozaltin-Zhang; source of the zhang set. Checked: Optimization Online page (published 2017/05/16).
- `meyer1974existence`: rational MILPs that are feasible and bounded attain their optimum; basis of the recession-ray certificates (no bilevel feasible point when the follower is unbounded). Checked: Crossref.

## Extremal graph theory (clique lower bounds)

- `mantel1907problem`: Mantel's theorem (triangle-free graphs have at most floor(n^2/4) edges); value >= 3 for all 60 clique instances. No DOI or online copy; details agree in two published reference lists (Borg-Fenech, Australas. J. Combin. 78 (2020); Korandi-Roberts-Scott 2021).
- `turan1941extremal`: Turan's theorem (t_r(n)); thresholds in the clique argument. No DOI; volume 48, pages 436-452 as in most published reference lists (one Springer list gives 436-453).
- `brouwer1981lotto`: Brouwer's Theorem 2 (K4-free with at least t_3(n) - floor(n/3) + 2 edges implies 3-partite, n >= 7); value >= 4 at density 0.9. Checked: CWI repository record and the scanned report (ZW 152/81, March 1981).
- `tyomkyn2015strong`: restatement of Brouwer's theorem (their eq. (1) and Theorem 1.1), crediting Brouwer. Checked: Crossref and the arXiv full text.
- `korandi2021exact`: restatement in complemented form (Theorem 1.1), cited in the appendix. Checked: Crossref; Advances in Combinatorics 2021:9, 17 pp.
- `amin2013non`: maximum size of non-(p-1)-partite maximal K_p-free graphs; an independent cross-check of Brouwer's threshold. Checked: Crossref (first page) and the journal/DOAJ record (pages 9-23).
- `babai1990extremal`: largest K_{r+1}-free subgraphs of G(n,1/2) are r-partite (context for the 3 open clique instances). Checked: Crossref.
- `brightwell2012extremal`: the same for sparser random graphs. Checked: Crossref.

## Maximum clique algorithms (solution checks)

- `bron1973algorithm`: Bron-Kerbosch, one of the two exact clique searches in `checksol.py`. Checked: Crossref.
- `tomita2006worst`: Tomita pivoting used with Bron-Kerbosch. Checked: Crossref.
- `carraghan1990exact`: Carraghan-Pardalos, the second exact clique search. Checked: Crossref.

## MIPLIB

- `gleixner2021miplib`: MIPLIB 2017; source models of the miplib2017 set, published optima used as HPR bounds, and the solution checker BOBILib's verification builds on. Checked: Crossref.
- `koch2011miplib`: MIPLIB 2010; source models of the miplib2010 set. Title as in the Crossref and Springer metadata ("MIPLIB 2010"); BOBILib's list adds the subtitle "Mixed integer programming library version 5". Checked: Crossref and the Springer page.
- `bixby1998miplib`: MIPLIB 3.0 (Optima 58, 12-15); source models of the miplib3 set. Checked: MIPLIB 3 page and the MIPLIB site's BibTeX record.

## Software used here

- `huangfu2018parallelizing`: HiGHS (version 1.15.1 here), the MILP/LP solver throughout; the citation HiGHS asks for. Checked: Crossref and highs.dev.
- `hojny2025scip`: SCIP Optimization Suite 10.0 (SCIP 10.0 via PySCIPOpt 6.3.0 here), used in the clique proofs and the OR10x100-0.75_4 check. Entry copied from scipopt.org's BibTeX (Optimization Online, Nov 2025, 28 authors). The arXiv version 2511.18580 lists 35 authors.
- `maher2016pyscipopt`: PySCIPOpt, the Python interface used to call SCIP. Checked: Crossref and Springer page (LNCS 9725).
- `perron2026cpsat`: OR-Tools CP-SAT (version 9.15, released Jan 2026), used in the clique proofs and searches. Software citation as requested on the OR-Tools "How to cite" page, with the version updated.
- `moura2021lean`: Lean 4 (toolchain v4.35.0 here). Checked: Crossref and Springer page (LNCS 12699).
- `mathlib2020`: Mathlib (v4.35.0 here). Checked: Crossref.

## Added in the review round

- `lefebvre2025dantzig`: Lefebvre and Schmidt, Dantzig-Wolfe single-level reformulation; source of published OR points (`lefebvre_schmidt_ub` in `results/literature/mkip_literature_status.csv`) and of four published points for general instances (`EXTRA_PUBLISHED` in `scripts/build_data_package.py`). Optimization Online preprint, June 2025 (latest version read: 20 July 2026); per-instance run files on henrilefebvre.com. Checked: Optimization Online page (title, authors) and the preprint's title page.
- `wood1993deterministic`: classical duality-based single-level reformulation in network interdiction (prior art for the LP bracket). Checked: Crossref.
- `israeli2002shortest`: the same, shortest-path interdiction. Checked: Crossref.
- `kellerer2004knapsack`: knapsack textbook; a basic LP solution of a multidimensional knapsack has at most as many fractional variables as rows (prior art for the m*p_max gap). Checked: Crossref (book, Springer Berlin Heidelberg, 2004).
- `cheung2017verifying`: VIPR certificates for MILP results (remedy for trusted solvers). Checked: Crossref (pages 148-160, LNCS 10328, editors Eisenbrand and Koenemann).
- `eifler2023computational`: exact rational MIP in SCIP. Checked: Crossref (Math. Program. 197(2) 793-812; online January 2022, issue 2023).
- `bogaerts2023certified`: VeriPB proof logging for optimisation (also maximum clique). Checked: Crossref (JAIR 77) and the JAIR/arXiv record for pages 1539-1589.
- `mahdavipajouh2020minimum`: the minimum cost edge blocker clique problem, the decision version behind the open clique instances. Checked: Crossref (Ann. Oper. Res. 294(1-2) 345-376; online 2019, issue 2020).
- `hanson1991ksaturated`: rediscovery of Brouwer's theorem. No DOI; details agree in the reference lists of Tyomkyn-Uzzell (2015) and Korandi-Roberts-Scott (2021).
- `kang2005maximum`: rediscovery of Brouwer's theorem. No DOI; details agree in the same two reference lists.
- `tyomkyn2015strong` (existing key) now also supports the refereed proof of Brouwer's theorem (their Theorem 1.1, proved in their Section 2), and `amin2013non` the same threshold for maximal K_p-free graphs (their Theorem 14).

## Not verified

None. No entry carries `note = {unverified}`. Two entries rest on published
secondary citations only, because the originals are not online:
`mantel1907problem` and `turan1941extremal`. Their details agree across the
reference lists consulted.

## Notes for the writer

- Plain `unsrt` prints neither `doi` nor `url`. For the SPIE-style list with DOIs
  use a style that prints them (for example `unsrtnat` with natbib's
  `numbers,super` options, or a custom SPIE-format style), and load `url` (or
  `hyperref`) for the `~` and `_` in URLs and DOIs.
- Article titles are stored in sentence case with proper nouns braced, so styles
  that lowercase titles keep "BOBILib", "MIPLIB", "Lean", "SCIP", "Turan".
- Web pages carry "Accessed 18 October 2026" in `note`.

## Re-checking and compile test

The `.bib` is curated by hand; nothing generates it. To compile-test it:

```
mkdir /tmp/bibtest && cp refs.bib /tmp/bibtest && cd /tmp/bibtest
printf '%s\n' '\documentclass{article}\usepackage{mathptmx}\usepackage[super]{cite}\usepackage{url}' \
  '\begin{document}x\nocite{*}\bibliographystyle{unsrt}\bibliography{refs}\end{document}' > t.tex
pdflatex t && bibtex t && pdflatex t && pdflatex t     # 64 items, 0 BibTeX warnings
```

The same document with `unsrtnat` also builds with 0 warnings and prints every
DOI and URL. To re-check the data, look up each `doi` at
`https://api.crossref.org/works/<doi>` and compare authors, title, volume,
number, pages and year.
