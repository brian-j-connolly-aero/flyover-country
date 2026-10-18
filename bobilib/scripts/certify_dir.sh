#!/bin/sh
# Re-check every solution file in a directory against its instance with
# `bobi certify` (rows, follower optimality, optimistic tie-break).
#
# usage: sh scripts/certify_dir.sh DIR [FOLLOWER_TIME]   (run from bobilib/)
dir=$1
ft=${2:-300}
for sol in "$dir"/*.res.json; do
  name=$(basename "$sol" .res.json)
  mps=$(find data/collection -name "$name.mps.gz" | head -n 1)
  if [ -z "$mps" ]; then
    echo "$name	NO INSTANCE"
    continue
  fi
  ./target/release/bobi certify "$mps" "$sol" --follower-time "$ft"
done
