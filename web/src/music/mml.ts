// A small, pure MML (Music Macro Language) parser for the Flyover Country
// soundtrack. No DOM, no WebAudio: it turns channel strings into timed events
// (in beats) that the synth schedules. See songs.ts for the music itself.
//
// Dialect summary (one channel = one monophonic voice):
//   c d e f g a b   notes; accidental right after the letter: + or # sharp, - flat
//   <len>[.[.]]     lengths 1 2 4 8 16 32 (1 = whole = 4 beats), triplets 3 6 12 24
//   r               rest (same length syntax)
//   l<len>[.]       default length (default l4)
//   o<n> > <        octave 0..8 (o4 c = MIDI 60), up / down one octave (default o4)
//   ^<len>          tie: extend the previous note or rest
//   v<n>            volume 0..15 (default 12)
//   q<n>            gate 1..8: notes sound for q/8 of their length (default 7)
//   [ ... ]<n>      repeat block n times (default 2), nesting up to depth 3
//   $               loop point (once per channel, not inside a repeat)
//   |               bar line (ignored by playback; checked by the validator)
//   ; comment       to end of line. Whitespace is ignored everywhere.
// Drum channels (inst 'noise') use drum letters instead of notes:
//   k kick, s snare, h closed hat, o open hat, c crash, t low tom, m mid tom, r rest
// and have no octave commands or accidentals.
//
// Time is counted internally in integer ticks (768 per beat), so triplets and
// dotted values add up exactly; beats are only produced at the output.

export type Inst = 'pulse12' | 'pulse25' | 'pulse50' | 'tri' | 'saw' | 'pad' | 'noise';
export type Drum = 'k' | 's' | 'h' | 'o' | 'c' | 't' | 'm';

export interface ChannelDef {
  inst: Inst;
  mml: string;
}

export interface SongDef {
  id: string;
  title: string;
  bpm: number;
  channels: Record<string, ChannelDef>;
}

/** t, dur and gate are in beats (quarter notes); gate = sounding length = dur * q / 8. */
export interface ParsedNote {
  t: number;
  dur: number;
  gate: number;
  midi: number;
  vol: number;
}

export interface ParsedDrum {
  t: number;
  drum: Drum;
  vol: number;
}

export interface ParsedChannel {
  name: string;
  inst: Inst;
  notes: ParsedNote[];
  drums: ParsedDrum[];
  /** Total length in beats. */
  length: number;
  /** Beat of the `$` loop point, or null if the channel has none. */
  loopAt: number | null;
  /** Beat length of each |-delimited bar, in order; empty if no bar lines. */
  bars: number[];
}

export interface ParsedSong {
  id: string;
  title: string;
  bpm: number;
  /** Max channel length, in beats. */
  length: number;
  /** Common `$` position in beats, or 0 when no channel has one. */
  loopAt: number;
  channels: ParsedChannel[];
}

export const INSTRUMENTS: readonly Inst[] = ['pulse12', 'pulse25', 'pulse50', 'tri', 'saw', 'pad', 'noise'];
export const DRUMS: readonly Drum[] = ['k', 's', 'h', 'o', 'c', 't', 'm'];

/** Ticks per beat. 768 = 3 * 256: exact for triplets and up to several dots. */
const TPQ = 768;
const WHOLE = TPQ * 4;
const LENGTHS = new Set([1, 2, 3, 4, 6, 8, 12, 16, 24, 32]);
const SEMITONE: Record<string, number> = { c: 0, d: 2, e: 4, f: 5, g: 7, a: 9, b: 11 };
const MAX_DEPTH = 3;
const DEFAULTS = { octave: 4, length: TPQ, volume: 12, gate: 7 };

interface Tok {
  ch: string;
  i: number;
}

/** A length as written: explicit number (or null for the default) plus dots. */
interface Len {
  n: number | null;
  dots: number;
  i: number;
}

type Node =
  | { k: 'note'; i: number; semi: number; len: Len }
  | { k: 'drum'; i: number; drum: Drum; len: Len }
  | { k: 'rest'; i: number; len: Len }
  | { k: 'tie'; i: number; len: Len }
  | { k: 'deflen'; i: number; len: Len }
  | { k: 'oct'; i: number; n: number }
  | { k: 'shift'; i: number; d: number }
  | { k: 'vol'; i: number; n: number }
  | { k: 'gate'; i: number; n: number }
  | { k: 'rep'; i: number; n: number; body: Node[] }
  | { k: 'loop'; i: number }
  | { k: 'bar'; i: number };

interface RawNote {
  t: number;
  dur: number;
  q: number;
  midi: number;
  vol: number;
}

/** Parse one channel. Throws an Error naming the channel and character index on bad syntax. */
export function parseChannel(name: string, def: ChannelDef): ParsedChannel {
  const src = typeof def?.mml === 'string' ? def.mml : '';

  const fail = (i: number, msg: string): never => {
    const upto = src.slice(0, Math.max(0, i));
    const line = upto.split('\n').length;
    const col = i - (upto.lastIndexOf('\n') + 1) + 1;
    const near = src.slice(Math.max(0, i), i + 12).replace(/\s+/g, ' ');
    throw new Error(`MML channel "${name}" at index ${i} (line ${line}, col ${col}, near "${near}"): ${msg}`);
  };

  if (!def || typeof def.mml !== 'string') fail(0, 'channel definition needs an "mml" string');
  if (!INSTRUMENTS.includes(def.inst)) fail(0, `unknown instrument "${String(def.inst)}"`);
  const drumMode = def.inst === 'noise';

  // 1. Strip whitespace and comments, keeping each character's source index.
  const toks: Tok[] = [];
  for (let i = 0; i < src.length; i++) {
    const ch = src[i];
    if (ch === ';') {
      while (i < src.length && src[i] !== '\n') i++;
      continue;
    }
    if (/\s/.test(ch)) continue;
    toks.push({ ch: ch.toLowerCase(), i });
  }

  // 2. Recursive-descent parse into a tree (repeats nest).
  let p = 0;
  let loopSeen = -1;
  const peek = (): Tok | undefined => toks[p];
  const endIndex = src.length;

  const readInt = (): number | null => {
    let s = '';
    while (p < toks.length && /[0-9]/.test(toks[p].ch)) s += toks[p++].ch;
    return s === '' ? null : Number(s);
  };

  const readLen = (i: number): Len => {
    const at = toks[p]?.i ?? endIndex;
    const n = readInt();
    if (n !== null && !LENGTHS.has(n)) {
      fail(at, `invalid length ${n} (use 1 2 4 8 16 32, or triplets 3 6 12 24)`);
    }
    let dots = 0;
    while (peek()?.ch === '.') {
      p++;
      dots++;
    }
    return { n, dots, i };
  };

  const readArg = (cmd: string, i: number, lo: number, hi: number): number => {
    const n = readInt();
    if (n === null) return fail(i, `"${cmd}" needs a number`);
    if (n < lo || n > hi) return fail(i, `"${cmd}${n}" out of range ${lo}..${hi}`);
    return n;
  };

  const parseSeq = (depth: number): Node[] => {
    const out: Node[] = [];
    while (p < toks.length) {
      const { ch, i } = toks[p];
      if (ch === ']') {
        if (depth === 0) fail(i, 'unmatched "]"');
        return out;
      }
      p++;
      if (ch === '[') {
        if (depth + 1 > MAX_DEPTH) fail(i, `repeats nested deeper than ${MAX_DEPTH}`);
        const body = parseSeq(depth + 1);
        if (peek()?.ch !== ']') fail(i, 'unclosed "["');
        p++; // ]
        const n = readInt();
        if (n !== null && n < 1) fail(i, 'repeat count must be at least 1');
        out.push({ k: 'rep', i, n: n ?? 2, body });
      } else if (ch === '$') {
        if (depth > 0) fail(i, 'loop point "$" cannot be inside a repeat');
        if (loopSeen >= 0) fail(i, `second loop point "$" (first at index ${loopSeen})`);
        loopSeen = i;
        out.push({ k: 'loop', i });
      } else if (ch === '|') {
        out.push({ k: 'bar', i });
      } else if (ch === 'r') {
        out.push({ k: 'rest', i, len: readLen(i) });
      } else if (ch === '^') {
        out.push({ k: 'tie', i, len: readLen(i) });
      } else if (ch === 'l') {
        const len = readLen(i);
        if (len.n === null) fail(i, '"l" needs a length');
        out.push({ k: 'deflen', i, len });
      } else if (ch === 'v') {
        out.push({ k: 'vol', i, n: readArg('v', i, 0, 15) });
      } else if (ch === 'q') {
        out.push({ k: 'gate', i, n: readArg('q', i, 1, 8) });
      } else if (drumMode) {
        if ((DRUMS as readonly string[]).includes(ch)) {
          const nx = peek()?.ch;
          if (nx === '+' || nx === '#' || nx === '-') fail(toks[p].i, 'accidentals do not apply to drums');
          out.push({ k: 'drum', i, drum: ch as Drum, len: readLen(i) });
        } else if (ch === '<' || ch === '>') {
          fail(i, 'octave commands do not exist in drum channels');
        } else {
          fail(i, `unexpected "${src[i]}" in drum channel (drums: k s h o c t m, rest r)`);
        }
      } else if (ch in SEMITONE) {
        let semi = SEMITONE[ch];
        const nx = peek()?.ch;
        if (nx === '+' || nx === '#') {
          semi++;
          p++;
        } else if (nx === '-') {
          semi--;
          p++;
        }
        out.push({ k: 'note', i, semi, len: readLen(i) });
      } else if (ch === 'o') {
        out.push({ k: 'oct', i, n: readArg('o', i, 0, 8) });
      } else if (ch === '>' || ch === '<') {
        out.push({ k: 'shift', i, d: ch === '>' ? 1 : -1 });
      } else {
        fail(i, `unexpected "${src[i]}"`);
      }
    }
    if (depth > 0) fail(endIndex, 'unclosed "[" at end of channel');
    return out;
  };

  const tree = parseSeq(0);

  // 3. Execute the tree in time order.
  let octave = DEFAULTS.octave;
  let deflen = DEFAULTS.length;
  let vol = DEFAULTS.volume;
  let q = DEFAULTS.gate;
  let t = 0;
  let loopAt: number | null = null;
  let lastBar = 0;
  let sawBar = false;
  const barTicks: number[] = [];
  const notes: RawNote[] = [];
  const drums: { t: number; drum: Drum; vol: number }[] = [];
  // What a "^" extends: the last note (by index), or a rest/drum (time only).
  let last: { note: RawNote | null } | null = null;

  const ticksOf = (len: Len): number => {
    const base = len.n === null ? deflen : WHOLE / len.n;
    let total = base;
    let add = base;
    for (let d = 0; d < len.dots; d++) {
      add /= 2;
      if (!Number.isInteger(add)) fail(len.i, 'too many dots for this length');
      total += add;
    }
    return total;
  };

  const run = (nodes: Node[]): void => {
    for (const nd of nodes) {
      switch (nd.k) {
        case 'note': {
          const dur = ticksOf(nd.len);
          const midi = (octave + 1) * 12 + nd.semi;
          if (midi < 0 || midi > 127) fail(nd.i, `note out of MIDI range (${midi})`);
          const n: RawNote = { t, dur, q, midi, vol };
          notes.push(n);
          last = { note: n };
          t += dur;
          break;
        }
        case 'drum': {
          drums.push({ t, drum: nd.drum, vol });
          last = { note: null };
          t += ticksOf(nd.len);
          break;
        }
        case 'rest':
          last = { note: null };
          t += ticksOf(nd.len);
          break;
        case 'tie': {
          if (!last) fail(nd.i, '"^" tie has nothing before it to extend (ties cannot cross "$")');
          const add = ticksOf(nd.len);
          if (last!.note) last!.note.dur += add;
          t += add;
          break;
        }
        case 'deflen':
          deflen = ticksOf(nd.len);
          break;
        case 'oct':
          octave = nd.n;
          break;
        case 'shift':
          octave += nd.d;
          if (octave < 0 || octave > 8) fail(nd.i, `octave ${octave} out of range 0..8`);
          break;
        case 'vol':
          vol = nd.n;
          break;
        case 'gate':
          q = nd.n;
          break;
        case 'rep':
          for (let r = 0; r < nd.n; r++) run(nd.body);
          break;
        case 'loop':
          loopAt = t;
          last = null;
          break;
        case 'bar':
          sawBar = true;
          if (t > lastBar) {
            barTicks.push(t - lastBar);
            lastBar = t;
          }
          break;
      }
    }
  };
  run(tree);
  if (sawBar && t > lastBar) barTicks.push(t - lastBar);

  const beats = (ticks: number) => ticks / TPQ;
  return {
    name,
    inst: def.inst,
    notes: notes.map((n) => ({
      t: beats(n.t),
      dur: beats(n.dur),
      gate: beats((n.dur * n.q) / 8),
      midi: n.midi,
      vol: n.vol,
    })),
    drums: drums.map((d) => ({ t: beats(d.t), drum: d.drum, vol: d.vol })),
    length: beats(t),
    loopAt: loopAt === null ? null : beats(loopAt),
    bars: barTicks.map(beats),
  };
}

const EPS = 1e-9;

/** Parse every channel of a song. Throws if any channel is malformed or the `$` loop points disagree. */
export function parseSong(song: SongDef): ParsedSong {
  const id = song?.id ?? '?';
  if (!song || typeof song.bpm !== 'number' || !(song.bpm > 0)) {
    throw new Error(`song "${id}": bpm must be a positive number`);
  }
  const channels: ParsedChannel[] = [];
  for (const [name, def] of Object.entries(song.channels ?? {})) {
    try {
      channels.push(parseChannel(name, def));
    } catch (e) {
      throw new Error(`song "${id}": ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  const length = channels.reduce((m, c) => Math.max(m, c.length), 0);
  let loopAt = 0;
  const looped = channels.filter((c) => c.loopAt !== null);
  if (looped.length > 0) {
    const missing = channels.filter((c) => c.loopAt === null).map((c) => c.name);
    if (missing.length > 0) {
      throw new Error(`song "${id}": loop point "$" is missing from channel(s) ${missing.join(', ')}`);
    }
    loopAt = looped[0].loopAt!;
    for (const c of looped) {
      if (Math.abs(c.loopAt! - loopAt) > EPS) {
        throw new Error(
          `song "${id}": loop point of channel "${c.name}" is at beat ${c.loopAt}, but "${looped[0].name}" loops at beat ${loopAt}`,
        );
      }
    }
  }
  return { id: song.id, title: song.title, bpm: song.bpm, length, loopAt, channels };
}
