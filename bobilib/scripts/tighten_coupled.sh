#!/bin/sh
# Re-derive coupled solution files under the strict follower tolerance with
# `bobi coupled-tighten`: the follower solved at each file's leader values
# with zero relative gap, then the optimistic tie-break with the follower's
# value at most 1e-9 (absolute; 1e-7 if the solver finds no point with
# 1e-9) above its optimum, every leader variable
# fixed or, if no such response meets the coupling rows, only the linking
# ones. One JSON line per file on stdout; certified points go to OUTDIR.
#
# usage: sh scripts/tighten_coupled.sh OUTDIR FOLLOWER_TIME SOL...   (run from bobilib/)
# SOL may end in .res.json or .res.json.tol (the files kept in
# results/general-coupled/tolerance/).
# BOBI overrides the binary (default ./target/release/bobi).
out=$1
ft=$2
shift 2
bobi=${BOBI:-./target/release/bobi}
mkdir -p "$out"
for sol in "$@"; do
  name=$(basename "$sol" .tol)
  name=${name%.res.json}
  mps=$(find data/collection -name "$name.mps.gz" | head -n 1)
  if [ -z "$mps" ]; then
    echo "{\"name\": \"$name\", \"status\": \"failed\", \"why\": \"no instance\"}"
    continue
  fi
  "$bobi" coupled-tighten "$mps" "$sol" --follower-time "$ft" --out "$out"
done
