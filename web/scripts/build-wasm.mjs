// Build the Rust engine for the browser: cargo -> wasm32, then wasm-bindgen.
import { execFileSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: 'inherit' });
run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'flyover-wasm']);
run('wasm-bindgen', [
  'target/wasm32-unknown-unknown/release/flyover_wasm.wasm',
  '--out-dir', 'web/src/wasm',
  '--target', 'web',
  '--omit-default-module-path',
]);
console.log('wasm: built into web/src/wasm');
