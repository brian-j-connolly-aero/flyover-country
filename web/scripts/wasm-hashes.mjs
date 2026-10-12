// Plays a bot game in the WebAssembly engine and prints each night's state
// hash. CI compares this with `flyover hashes` from the native build.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { initSync, Engine } from '../src/wasm/flyover_wasm.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const args = Object.fromEntries(process.argv.slice(2).map((a, i, all) => (a.startsWith('--') ? [a.slice(2), all[i + 1]] : null)).filter(Boolean));
const seed = Number(args.seed ?? 7);
const factions = (args.factions ?? 'Pacific,FrontRange,Texas,Beehive').split(',');

initSync({ module: readFileSync(join(here, '..', 'src', 'wasm', 'flyover_wasm_bg.wasm')) });
const read = (f) => readFileSync(join(root, 'content', `${f}.ron`), 'utf8');
const engine = new Engine(read('map'), read('factions'), read('rules'), read('flavor'));
let state = engine.new_game(JSON.stringify({ seed, factions }));
for (let night = 1; night <= 8; night++) {
  const orders = factions.map((_, seat) => JSON.parse(engine.suggest(state, seat, 'normal')));
  const r = JSON.parse(engine.resolve(state, JSON.stringify(orders)));
  state = JSON.stringify(r.state);
  console.log(engine.state_hash(state));
}
