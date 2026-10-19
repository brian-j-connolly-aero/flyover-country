# The BOBILib paper

Sources of the paper "Settling 648 unsolved BOBILib instances: published optima, new
optimality proofs and infeasibility certificates"
(Brian J. Connolly), its supplementary document, its figures, and the scripts that
build the data package in `../results/`.

## What is here

| Path | Contents |
| --- | --- |
| `main.tex`, `main.pdf` | the paper, in the format of SPIE proceedings (`spie.cls`) |
| `refs.bib`, `refs_notes.md` | the bibliography, and one line per key saying what it supports and how it was checked |
| `spiebib-doi.bst` | a copy of SPIE's `spiebib.bst` (v2.7) that ends every reference with `https://doi.org/<doi>` (or the url when there is no DOI), writes names as "Last, F.", puts the year in parentheses and ends every title with a comma; the changes are marked `bobi` in the file. `main.tex` prints the labels as "[1]", as in the author's earlier SPIE paper |
| `generated/facts.tex` | one `\newcommand` for every number the paper quotes, written by `scripts/gen_facts.py` from `../results/` (never edit by hand) |
| `figures/*.pdf` | the 15 figures, one file each; `figures/MANIFEST.md` lists them with their captions and data sources. The paper uses 13 of them: `fig_family_outcomes` (it repeats Table 1) and `fig_hpr_bound` were dropped in the review round but are still built |
| `figures/src/` | the figure sources: matplotlib scripts (`fig_*.py`, `make_all.sh`, `style.py`) that read only `../results/`, and standalone TikZ files (`fig_*.tex`); see `figures/src/README.md` |
| `appendix/appendix.tex`, `appendix/appendix.pdf` | the supplementary document (proofs, verification protocol, Lean map, data package, per-instance tables), cited by the paper as "Supplementary Section A, B, ..."; `appendix/sections/` holds its sections |
| `appendix/generated/*.tex` | its per-instance tables, written by `scripts/gen_appendix_tables.py` (never edit by hand) |
| `scripts/build_data_package.py` | writes `../results/instances.csv`, `bobilib_update.solu`, `README.md` and `SHA256SUMS` |
| `scripts/gen_facts.py` | writes `generated/facts.tex` and checks every value against the paper's spec |
| `scripts/gen_appendix_tables.py` | writes `appendix/generated/*.tex` |
| `Makefile` | the targets below |

## Build

From this folder (`bobilib/paper/`):

```sh
make all        # data, figures, paper, appendix, in this order
make data       # ../results/ index files, generated/facts.tex, appendix/generated/*.tex
make figures    # figures/*.pdf: figures/src/make_all.sh (matplotlib) and every TikZ source
make paper      # main.pdf  (latexmk -pdf main.tex: pdflatex + bibtex)
make appendix   # appendix/appendix.pdf
make clean      # remove LaTeX auxiliary files
```

`make data` takes optional inputs (see the comments in the `Makefile` and
`scripts/build_data_package.py`): `META=collection.json` (BOBILib's instance metadata
from bobilib.org; without it the BOBILib columns are read back from
`../results/instances.csv`) and the literature tables `MKIP_LIT=`, `MIBLP_LIT=` and
`LBC_OPTIMAL=`. Without them the build reproduces the committed data package
unchanged. The scripts stop with an error if a count differs from the one the
paper quotes.

The paper takes every number from `generated/facts.tex`, so run `make data` before
`make paper` after any change under `../results/`. The figures are deterministic:
rebuilding them gives byte-identical PDFs.

## Requirements

* A TeX Live installation with `pdflatex`, `bibtex` and `latexmk`. On Debian or
  Ubuntu the packages are `texlive-latex-base`, `texlive-latex-recommended`,
  `texlive-latex-extra` (authblk, capt-of, xurl, titlesec, standalone),
  `texlive-fonts-recommended` (Times), `lmodern`, `texlive-science` (siunitx, for the
  appendix tables), `texlive-pictures` (TikZ, for the figures), `latexmk`, and
  **`texlive-publishers`, which provides `spie.cls` and `spiebib.bst`**.
* Python 3 with matplotlib (3.11 was used) and numpy, for `make figures`. The data
  scripts use only the standard library.
* `pdftoppm` (poppler) is handy for looking at pages:
  `pdftoppm -r 80 -png main.pdf /tmp/page`.

## Placeholders

`main.tex` and `appendix/appendix.tex` define `\authoremail` as the literal text
`EMAIL`, and `main.tex` defines `\repourl` (`REPOSITORY-URL`) and `\datadoi`
(`DATA-DOI`) for the Data availability section. Fill them in before submission;
`../results/README.md` has the matching placeholders for the data package.
