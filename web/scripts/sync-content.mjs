// Copy the RON content files next to the app so the browser can fetch them.
import { cpSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const src = join(here, '..', '..', 'content');
const dst = join(here, '..', 'public', 'content');
mkdirSync(dst, { recursive: true });
const files = readdirSync(src).filter((f) => f.endsWith('.ron'));
for (const f of files) cpSync(join(src, f), join(dst, f));
// Drop files the game no longer has, so nothing stale gets served.
for (const f of readdirSync(dst)) if (f.endsWith('.ron') && !files.includes(f)) rmSync(join(dst, f));
console.log(`content: copied to ${dst}`);
