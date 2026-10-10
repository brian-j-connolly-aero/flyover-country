// Validates the soundtrack in src/music/songs.ts.
// Run from the repo root:  node web/scripts/check-music.ts
// (Node 22+ strips the TypeScript types; no build step needed.)
//
// Checks: every song parses; all channels share one length that is a whole
// number of 4-beat bars; `$` loop points agree; every |-delimited bar is
// exactly 4 beats; note ranges per instrument/role; volumes 0..15; BPM and
// form length per song; pad notes are held (a beat or longer). It also warns
// (without failing) about minor-second clashes between lead and harmony on
// strong beats and about volume changes on the triangle bass.

import { parseChannel, parseSong, type ParsedChannel, type ParsedSong, type SongDef } from '../src/music/mml.ts';
import { SONGS } from '../src/music/songs.ts';

const EPS = 1e-9;
const BAR = 4;
const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const noteName = (midi: number) => `${NOTE_NAMES[midi % 12]}${Math.floor(midi / 12) - 1}`;
const near = (a: number, b: number) => Math.abs(a - b) < EPS;
const isWhole = (x: number) => near(x, Math.round(x));
const fmtBeat = (b: number) => (isWhole(b) ? String(Math.round(b)) : b.toFixed(3));
/** "bar 12 beat 3" (1-based) for a beat position. */
const where = (t: number) => `bar ${Math.floor(t / BAR + EPS) + 1} beat ${fmtBeat((t % BAR) + 1)}`;

/** Expected tempo and form per song id: loop-body bars and optional intro bars before `$`. */
const SPEC: Record<string, { bpm: [number, number]; loopBars: [number, number]; introBars: [number, number] }> = {
  theme: { bpm: [112, 128], loopBars: [32, 48], introBars: [0, 4] },
  briefing: { bpm: [90, 100], loopBars: [24, 32], introBars: [0, 0] },
  sortie: { bpm: [140, 152], loopBars: [32, 48], introBars: [0, 4] },
  glovesoff: { bpm: [156, 168], loopBars: [32, 48], introBars: [0, 4] },
};

/** Allowed MIDI range for a channel, from its instrument and (for pulse/saw) its role. */
function rangeFor(ch: ParsedChannel): { lo: number; hi: number; label: string } | null {
  switch (ch.inst) {
    case 'noise':
      return null;
    case 'tri':
      return { lo: 28, hi: 48, label: 'bass (tri) E1..C3' };
    case 'pad':
      return { lo: 48, hi: 83, label: 'pad C3..B5' };
    default:
      if (ch.name.startsWith('lead')) return { lo: 60, hi: 84, label: 'lead C4..C6' };
      return { lo: 48, hi: 83, label: 'harmony C3..B5' };
  }
}

const errors: string[] = [];
const warnings: string[] = [];

function checkSong(song: SongDef): ParsedSong | null {
  const tag = `[${song.id}]`;
  const err = (m: string) => errors.push(`${tag} ${m}`);
  const warn = (m: string) => warnings.push(`${tag} ${m}`);

  // Parse each channel on its own first so one bad channel doesn't hide the others.
  let bad = false;
  for (const [name, def] of Object.entries(song.channels)) {
    try {
      parseChannel(name, def);
    } catch (e) {
      err(`syntax: ${(e as Error).message}`);
      bad = true;
    }
  }
  if (bad) return null;

  // Loop points: all or none, at identical beats (reported here in detail;
  // parseSong enforces the same rule).
  const chans = Object.entries(song.channels).map(([n, d]) => parseChannel(n, d));
  const withLoop = chans.filter((c) => c.loopAt !== null);
  let loopBad = false;
  if (withLoop.length > 0 && withLoop.length < chans.length) {
    err(`"$" present in ${withLoop.map((c) => c.name).join(', ')} but missing in ${chans.filter((c) => c.loopAt === null).map((c) => c.name).join(', ')}`);
    loopBad = true;
  }
  if (withLoop.length > 0) {
    const at = withLoop[0].loopAt!;
    for (const c of withLoop) {
      if (!near(c.loopAt!, at)) {
        err(`"$" in ${c.name} is at beat ${fmtBeat(c.loopAt!)} (${where(c.loopAt!)}), but ${withLoop[0].name} has it at beat ${fmtBeat(at)}`);
        loopBad = true;
      }
    }
  }
  if (loopBad) return null;

  let parsed: ParsedSong;
  try {
    parsed = parseSong(song);
  } catch (e) {
    err((e as Error).message);
    return null;
  }

  // Tempo.
  const spec = SPEC[song.id];
  if (!spec) err(`unexpected song id (expected one of ${Object.keys(SPEC).join(', ')})`);
  else if (song.bpm < spec.bpm[0] || song.bpm > spec.bpm[1]) err(`bpm ${song.bpm} outside ${spec.bpm[0]}..${spec.bpm[1]}`);

  if (chans.length < 4 || chans.length > 5) err(`${chans.length} channels (expected 4..5)`);
  if (!song.title) err('missing title');

  // Lengths.
  const len0 = parsed.channels[0]?.length ?? 0;
  for (const c of parsed.channels) {
    if (!near(c.length, len0)) err(`channel ${c.name} is ${fmtBeat(c.length)} beats, ${parsed.channels[0].name} is ${fmtBeat(len0)} beats`);
  }
  if (!isWhole(parsed.length / BAR)) err(`length ${fmtBeat(parsed.length)} beats is not a whole number of 4-beat bars`);
  if (!isWhole(parsed.loopAt / BAR)) err(`loop point at beat ${fmtBeat(parsed.loopAt)} is not on a bar line`);
  if (spec) {
    const loopBars = (parsed.length - parsed.loopAt) / BAR;
    const intro = parsed.loopAt / BAR;
    if (loopBars < spec.loopBars[0] - EPS || loopBars > spec.loopBars[1] + EPS) err(`loop body is ${fmtBeat(loopBars)} bars (expected ${spec.loopBars[0]}..${spec.loopBars[1]})`);
    if (intro < spec.introBars[0] - EPS || intro > spec.introBars[1] + EPS) err(`intro before "$" is ${fmtBeat(intro)} bars (expected ${spec.introBars[0]}..${spec.introBars[1]})`);
  }

  for (const c of parsed.channels) {
    // Bar lines.
    c.bars.forEach((b, i) => {
      if (!near(b, BAR)) err(`${c.name}: bar ${i + 1} is ${fmtBeat(b)} beats, not 4`);
    });

    // Ranges and volumes.
    const r = rangeFor(c);
    for (const n of c.notes) {
      if (r && (n.midi < r.lo || n.midi > r.hi)) err(`${c.name}: ${noteName(n.midi)} at ${where(n.t)} is outside ${r.label}`);
      if (n.vol < 0 || n.vol > 15) err(`${c.name}: volume ${n.vol} at ${where(n.t)}`);
      if (c.inst === 'pad' && n.dur < 1 - EPS) err(`${c.name}: pad note ${noteName(n.midi)} at ${where(n.t)} is shorter than a beat`);
    }
    for (const d of c.drums) {
      if (d.vol < 0 || d.vol > 15) err(`${c.name}: volume ${d.vol} at ${where(d.t)}`);
    }
    if (c.inst !== 'noise' && c.drums.length) err(`${c.name}: pitched channel produced drum events`);
    if (c.inst === 'noise' && c.notes.length) err(`${c.name}: drum channel produced notes`);
    if (c.inst === 'tri') {
      const vols = new Set(c.notes.map((n) => n.vol));
      if (vols.size > 1) warn(`${c.name}: triangle bass changes volume (${[...vols].join(', ')}); the triangle ignores it`);
    }
  }

  // Minor-second (or minor-ninth / major-seventh) clashes between lead and
  // harmony on strong beats (beats 1 and 3).
  const lead = parsed.channels.find((c) => c.name === 'lead');
  const harm = parsed.channels.find((c) => c.name === 'harmony');
  if (lead && harm) {
    const sounding = (c: ParsedChannel, t: number) => c.notes.find((n) => n.t <= t + EPS && t < n.t + n.gate - EPS);
    for (let t = 0; t < parsed.length - EPS; t += 2) {
      const a = sounding(lead, t);
      const b = sounding(harm, t);
      if (!a || !b) continue;
      const iv = Math.abs(a.midi - b.midi) % 12;
      if (iv === 1 || iv === 11) warn(`lead ${noteName(a.midi)} vs harmony ${noteName(b.midi)} at ${where(t)}`);
    }
  }
  return parsed;
}

// Song ids must be exactly these, once each.
const ids = SONGS.map((s) => s.id);
for (const id of Object.keys(SPEC)) if (!ids.includes(id)) errors.push(`missing song "${id}"`);
if (new Set(ids).size !== ids.length) errors.push(`duplicate song ids: ${ids.join(', ')}`);

const rows: string[][] = [];
for (const song of SONGS) {
  const p = checkSong(song);
  if (!p) {
    rows.push([song.id, String(song.bpm), '-', '-', '-', 'FAILED (see errors below)']);
    continue;
  }
  const intro = p.loopAt / BAR;
  const loopBars = (p.length - p.loopAt) / BAR;
  const secs = ((p.length - p.loopAt) * 60) / p.bpm;
  rows.push([
    p.id,
    String(p.bpm),
    intro > 0 ? `${fmtBeat(intro)}+${fmtBeat(loopBars)}` : fmtBeat(loopBars),
    secs.toFixed(1),
    String(p.channels.length),
    p.channels.map((c) => `${c.name}(${c.inst}):${c.inst === 'noise' ? c.drums.length : c.notes.length}`).join(' '),
  ]);
}

const head = ['song', 'bpm', 'bars', 'sec/loop', 'ch', 'events per channel (notes or drum hits)'];
const widths = head.map((h, i) => Math.max(h.length, ...rows.map((r) => r[i].length)));
const line = (r: string[]) => r.map((c, i) => c.padEnd(widths[i])).join('  ').trimEnd();
console.log(line(head));
console.log(widths.map((w) => '-'.repeat(w)).join('  '));
for (const r of rows) console.log(line(r));
console.log('bars = intro+loop body (intro plays once, before "$")');

if (warnings.length) {
  console.log(`\n${warnings.length} warning(s):`);
  for (const w of warnings) console.log(`  warn: ${w}`);
}
if (errors.length) {
  console.log(`\n${errors.length} error(s):`);
  for (const e of errors) console.log(`  FAIL: ${e}`);
  process.exit(1);
}
console.log('\nAll music checks passed.');
