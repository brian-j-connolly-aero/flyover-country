#!/bin/sh
# Rebuild the matplotlib data figures (figures/<name>.pdf) from bobilib/results/.
# Each script checks its numbers against the paper's and exits non-zero on a mismatch.
set -eu
cd "$(dirname "$0")"
for f in fig_overview_status fig_family_outcomes fig_kip_search fig_or_gaps \
         fig_or_improvement fig_clique_values fig_general_outcomes; do
    python3 -I -B "$f.py"
done
