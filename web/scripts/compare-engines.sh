#!/usr/bin/env sh
# Same seed and orders, same state: the native engine and the WebAssembly
# engine in web/src/wasm must print identical nightly hashes.
set -e
cd "$(dirname "$0")/../.."
for args in "--seed 7 --factions Pacific,FrontRange,Texas,Beehive" \
            "--seed 123 --factions Cascadia,Dreamland,GrainBelt,Texas" \
            "--seed 9001 --factions Beehive,Pacific,GrainBelt,Cascadia"; do
  # shellcheck disable=SC2086
  ./target/release/flyover hashes $args > target/native-hashes.txt
  # shellcheck disable=SC2086
  node web/scripts/wasm-hashes.mjs $args > target/wasm-hashes.txt
  diff target/native-hashes.txt target/wasm-hashes.txt
  echo "match: $args"
done
