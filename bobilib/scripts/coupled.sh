#!/bin/sh
# Feasible points for general instances with coupling rows, one instance at
# a time (single-threaded HiGHS), appending one JSON line per instance.
#
# usage: sh scripts/coupled.sh LIST OUT.jsonl LOG [TIME] [EXTRA bobi coupled ARGS...]
# LIST holds one .mps.gz path per line; run from bobilib/.
# Optional environment:
#   BOBI    another binary (e.g. a copy, so rebuilding does not change a long run)
#   BUDGET  wall-clock seconds for the whole batch: once fewer than TIME
#           seconds remain, the rest of LIST is recorded as not attempted
#   MEM_KB  address-space limit per run in KB (default 3000000, about 3 GB)
list=$1
out=$2
log=$3
time=${4:-600}
if [ $# -ge 4 ]; then shift 4; else shift $#; fi
bin=${BOBI:-./target/release/bobi}
mem=${MEM_KB:-3000000}
start=$(date +%s)
mkdir -p results/general-coupled
while read -r f; do
  [ -z "$f" ] && continue
  name=$(basename "$f" .mps.gz)
  if [ -n "$BUDGET" ] && [ $(($(date +%s) - start + time)) -gt "$BUDGET" ]; then
    echo "{\"name\":\"$name\",\"file\":\"$f\",\"status\":\"not_attempted\"}"
    continue
  fi
  # Hard stop well past the search's own budget (certification included).
  (
    ulimit -v "$mem"
    exec timeout $((time * 2 + 300)) nice -n 5 "$bin" coupled "$f" \
      --time "$time" --out results/general-coupled --log "$@"
  ) </dev/null 2>>"$log" ||
    echo "{\"name\":\"$name\",\"file\":\"$f\",\"status\":\"error\",\"error\":\"timeout, memory limit or crash\"}"
done <"$list" >>"$out"
echo '{"finished":true}' >>"$out"
