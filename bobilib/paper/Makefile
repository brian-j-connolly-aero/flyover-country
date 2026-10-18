# Build the BOBILib paper, its appendix, its figures and the data package.
#
#   make data      results/ data package (instances.csv, bobilib_update.solu, README.md,
#                  SHA256SUMS), generated/facts.tex and appendix/generated/*.tex
#   make figures   figures/*.pdf: figures/src/make_all.sh (matplotlib) and the TikZ sources
#   make paper     main.pdf (pdflatex + bibtex via latexmk)
#   make appendix  appendix/appendix.pdf
#   make all       all of the above, in this order
#   make clean     remove LaTeX auxiliary files
#
# Optional inputs of `make data` (see scripts/build_data_package.py):
#   META=collection.json         BOBILib metadata; default: the BOBILib columns of results/instances.csv
#   MKIP_LIT=, MIBLP_LIT=, LBC_OPTIMAL=   literature tables for the published optima;
#                                default: the values recorded in results/instances.csv

PY          ?= python3 -I
LATEXMK     ?= latexmk -pdf -interaction=nonstopmode -halt-on-error
META        ?=
MKIP_LIT    ?=
MIBLP_LIT   ?=
LBC_OPTIMAL ?=

DATA_ARGS = $(if $(META),--meta $(META)) $(if $(MKIP_LIT),--mkip-lit $(MKIP_LIT)) \
            $(if $(MIBLP_LIT),--miblp-lit $(MIBLP_LIT)) $(if $(LBC_OPTIMAL),--lbc-optimal $(LBC_OPTIMAL))

.PHONY: all data figures paper appendix clean
.NOTPARALLEL:

all: data figures paper appendix

data:
	$(PY) scripts/build_data_package.py $(DATA_ARGS)
	$(PY) scripts/gen_facts.py
	$(PY) scripts/gen_appendix_tables.py

figures:
	@if [ -f figures/src/make_all.sh ]; then \
	  echo "figures: figures/src/make_all.sh"; sh figures/src/make_all.sh; \
	else echo "figures: no figures/src/make_all.sh, skipping the data figures"; fi
	@set -e; tmp=$$(mktemp -d); n=0; \
	for f in figures/src/*.tex; do \
	  [ -f "$$f" ] || continue; b=$$(basename "$$f" .tex); n=$$((n+1)); \
	  echo "figures: $$f"; \
	  (cd figures/src && $(LATEXMK) -outdir="$$tmp" "$$b.tex" > "$$tmp/$$b.out" 2>&1) \
	    || { tail -20 "$$tmp/$$b.out"; exit 1; }; \
	  cp "$$tmp/$$b.pdf" "figures/$$b.pdf"; \
	done; rm -rf "$$tmp"; echo "figures: $$n TikZ sources"

paper:
	@[ -f main.tex ] || { echo "paper: main.tex not found"; exit 1; }
	$(LATEXMK) main.tex

appendix:
	@[ -f appendix/appendix.tex ] || { echo "appendix: appendix/appendix.tex not found"; exit 1; }
	$(LATEXMK) -cd appendix/appendix.tex

clean:
	-@[ -f main.tex ] && latexmk -c main.tex
	-@[ -f appendix/appendix.tex ] && latexmk -c -cd appendix/appendix.tex
