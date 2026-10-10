/*
 * Flyover Country — hand-authored SVG art for the dog faction leaders and faction emblems.
 *
 * Dependency-free. Every exported function returns a complete, standalone `<svg>` markup string
 * (suitable for innerHTML, an <img> data URI, or a PixiJS texture source).
 *
 * Style: bold flat cartoon vector, ~3px near-black outline, flat base + one shadow + one highlight
 * per color. Portraits use a fixed 240x240 viewBox (the Colorado duo uses 320x240).
 * All gradient / clipPath ids are unique per call (module-level counter), so any number of
 * portraits can share one page.
 */

export type LeaderId = 'donut' | 'huckleberry' | 'griffin' | 'pan' | 'tank' | 'lucky' | 'sorghum' | 'biscuit';
export type Mood = 'neutral' | 'happy' | 'alarmed';
export type FactionKey = 'Pacific' | 'Cascadia' | 'FrontRange' | 'Texas' | 'Dreamland' | 'GrainBelt' | 'Beehive';

export interface PortraitOptions {
  /** Rendered height in px (width too, for square portraits). Default 240. */
  size?: number;
  /** Draw the rounded navy backdrop with a faction-colored glow. Default false (transparent). */
  bg?: boolean;
}

export interface EmblemOptions {
  /** Rendered width/height in px. Default 64. */
  size?: number;
}

export const FACTION_COLORS: Record<FactionKey, string> = {
  Pacific: '#FF5FA2',
  Cascadia: '#3DDC84',
  FrontRange: '#4CC9F0',
  Texas: '#FF4D4D',
  Dreamland: '#B388FF',
  GrainBelt: '#F2E86D',
  Beehive: '#FF9F1C',
};

/** Which faction each dog leads (Griffin and Pan co-lead the Front Range). */
export const LEADER_FACTION: Record<LeaderId, FactionKey> = {
  donut: 'Pacific',
  huckleberry: 'Cascadia',
  griffin: 'FrontRange',
  pan: 'FrontRange',
  tank: 'Texas',
  lucky: 'Dreamland',
  sorghum: 'GrainBelt',
  biscuit: 'Beehive',
};

const LEADER_NAMES: Record<LeaderId, string> = {
  donut: 'Princess Donut',
  huckleberry: 'Ranger Huckleberry',
  griffin: 'Griffin',
  pan: 'Pan',
  tank: 'Coach Tank',
  lucky: 'Lucky',
  sorghum: 'Agent Sorghum',
  biscuit: 'Biscuit',
};

// ---------------------------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------------------------

type Pt = readonly [number, number];

const OL = '#14101a'; // outline ink
const MOUTH_IN = '#4d1529';
const TONGUE = '#ff6f94';
const TONGUE_SH = '#d6456c';
const PUPIL = '#0a0710';

let uidCounter = 0;

/** Round for compact markup. */
const n = (v: number): string => String(Math.round(v * 100) / 100);

const P = (d: string, fill: string, extra = ''): string => `<path d="${d}" fill="${fill}"${extra}/>`;
/** Filled + outlined path. */
const PO = (d: string, fill: string, sw = 3, stroke = OL, extra = ''): string =>
  `<path d="${d}" fill="${fill}" stroke="${stroke}" stroke-width="${sw}" stroke-linejoin="round" stroke-linecap="round"${extra}/>`;
/** Stroke-only path. */
const LN = (d: string, stroke = OL, sw = 2.5, extra = ''): string =>
  `<path d="${d}" fill="none" stroke="${stroke}" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round"${extra}/>`;
const E = (cx: number, cy: number, rx: number, ry: number, fill: string, extra = ''): string =>
  `<ellipse cx="${n(cx)}" cy="${n(cy)}" rx="${n(rx)}" ry="${n(ry)}" fill="${fill}"${extra}/>`;
const EO = (cx: number, cy: number, rx: number, ry: number, fill: string, sw = 3, stroke = OL, extra = ''): string =>
  `<ellipse cx="${n(cx)}" cy="${n(cy)}" rx="${n(rx)}" ry="${n(ry)}" fill="${fill}" stroke="${stroke}" stroke-width="${sw}"${extra}/>`;
const C = (cx: number, cy: number, r: number, fill: string, extra = ''): string =>
  `<circle cx="${n(cx)}" cy="${n(cy)}" r="${n(r)}" fill="${fill}"${extra}/>`;
const CO = (cx: number, cy: number, r: number, fill: string, sw = 3, stroke = OL): string =>
  `<circle cx="${n(cx)}" cy="${n(cy)}" r="${n(r)}" fill="${fill}" stroke="${stroke}" stroke-width="${sw}"/>`;
const G = (inner: string, transform?: string, extra = ''): string =>
  transform ? `<g transform="${transform}"${extra}>${inner}</g>` : `<g${extra}>${inner}</g>`;
const rot = (a: number, x: number, y: number): string => `rotate(${n(a)} ${n(x)} ${n(y)})`;
const rotA = (a: number, x: number, y: number): string => ` transform="${rot(a, x, y)}"`;

/**
 * Mirror an absolute-command path (M L C Q S T H V A Z, uppercase only) across x = cx.
 * Lets each dog be authored as one half and reflected for perfect symmetry.
 */
function mirror(d: string, cx = 120): string {
  const toks = d.match(/[A-Za-z]|-?\d*\.?\d+/g) ?? [];
  const out: string[] = [];
  let cmd = '';
  let i = 0;
  for (const t of toks) {
    if (/[A-Za-z]/.test(t)) {
      cmd = t;
      i = 0;
      out.push(t);
      continue;
    }
    let v = parseFloat(t);
    if (cmd === 'H') v = 2 * cx - v;
    else if (cmd === 'V') {
      /* y unchanged */
    } else if (cmd === 'A') {
      const k = i % 7;
      if (k === 2) v = -v;
      else if (k === 4) v = 1 - v;
      else if (k === 5) v = 2 * cx - v;
    } else if (i % 2 === 0) v = 2 * cx - v;
    i++;
    out.push(n(v));
  }
  return out.join(' ');
}

function hexRgb(h: string): [number, number, number] {
  const s = h.replace('#', '');
  const f = s.length === 3 ? s.split('').map((c) => c + c).join('') : s;
  return [parseInt(f.slice(0, 2), 16), parseInt(f.slice(2, 4), 16), parseInt(f.slice(4, 6), 16)];
}
/** Linear blend of two hex colors (t = 0 -> a, 1 -> b). */
function mix(a: string, b: string, t: number): string {
  const x = hexRgb(a);
  const y = hexRgb(b);
  const c = x.map((v, i) => Math.round(v + (y[i] - v) * t));
  return '#' + c.map((v) => v.toString(16).padStart(2, '0')).join('');
}

/** Small deterministic PRNG (mulberry32) so speckles are identical on every render. */
function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Scatter of little dots inside an ellipse (heeler ticking, grey chin hairs...). */
function speckles(seed: number, count: number, cx: number, cy: number, rx: number, ry: number, rMin: number, rMax: number, color: string, opacity = 1): string {
  const rand = rng(seed);
  let s = '';
  for (let i = 0; i < count; i++) {
    const a = rand() * Math.PI * 2;
    const d = Math.sqrt(rand());
    const r = rMin + rand() * (rMax - rMin);
    const x = cx + Math.cos(a) * d * rx;
    const y = cy + Math.sin(a) * d * ry;
    s += `<ellipse cx="${n(x)}" cy="${n(y)}" rx="${n(r)}" ry="${n(r * 0.8)}"/>`;
  }
  return `<g fill="${color}"${opacity < 1 ? ` opacity="${opacity}"` : ''}>${s}</g>`;
}

/** Bumpy circle (pompoms, clouds, fluff). */
function bumpy(cx: number, cy: number, r: number, amp: number, k: number, phase = 0): string {
  let d = '';
  for (let i = 0; i <= k; i++) {
    const a = phase + (i / k) * Math.PI * 2;
    const x = cx + Math.cos(a) * r;
    const y = cy + Math.sin(a) * r;
    if (i === 0) d += `M ${n(x)} ${n(y)}`;
    else {
      const m = phase + ((i - 0.5) / k) * Math.PI * 2;
      d += ` Q ${n(cx + Math.cos(m) * (r + amp * 2))} ${n(cy + Math.sin(m) * (r + amp * 2))} ${n(x)} ${n(y)}`;
    }
  }
  return d + ' Z';
}

/** Per-call rendering context: unique id generator + collected <defs>. */
class Ctx {
  readonly uid: string;
  readonly defs: string[] = [];
  private k = 0;
  constructor() {
    uidCounter += 1;
    this.uid = 'fcd' + uidCounter.toString(36);
  }
  id(tag: string): string {
    this.k += 1;
    return `${this.uid}${tag}${this.k.toString(36)}`;
  }
  /** Register a clipPath from raw shape markup; returns its id. */
  clip(shape: string): string {
    const id = this.id('c');
    this.defs.push(`<clipPath id="${id}">${shape}</clipPath>`);
    return id;
  }
  /**
   * A colored region: base fill, interior detail clipped to the shape (shading, markings),
   * and the outline drawn last so markings never spill over the silhouette.
   */
  region(d: string, fill: string, inner = '', sw = 3, stroke = OL): string {
    let s = `<path d="${d}" fill="${fill}"/>`;
    if (inner) s += `<g clip-path="url(#${this.clip(`<path d="${d}"/>`)})">${inner}</g>`;
    s += `<path d="${d}" fill="none" stroke="${stroke}" stroke-width="${sw}" stroke-linejoin="round" stroke-linecap="round"/>`;
    return s;
  }
}

// ---------------------------------------------------------------------------------------------
// Face parts
// ---------------------------------------------------------------------------------------------

interface EyeSpec {
  x: number;
  y: number;
  /** vertical radius */
  r: number;
  iris: string;
  /** rx / ry */
  ratio?: number;
  /** gaze, each axis -1..1 */
  look?: Pt;
  pupil?: number;
  /** 0..1 of the eye covered by the upper lid */
  lid?: number;
  lidFill?: string;
  /** px; positive lowers the lid's left end, raises its right end */
  lidTilt?: number;
  /** 0..1 lower lid (smiling eyes) */
  low?: number;
  lowFill?: string;
  /** ring color around the eye (for dark fur) */
  rim?: string;
  shine?: number;
}

function eye(ctx: Ctx, e: EyeSpec): string {
  const ry = e.r;
  const rx = e.r * (e.ratio ?? 0.92);
  const [lx, ly] = e.look ?? [0, 0];
  const ix = e.x + lx * rx * 0.26;
  const iy = e.y + ly * ry * 0.22;
  const irx = rx * 0.8;
  const iry = ry * 0.82;
  const shine = e.shine ?? 1;
  const lid = e.lid ?? 0;
  const lidY = e.y - ry + 2 * ry * lid;
  const eyeClip = ctx.clip(E(e.x, e.y, rx, ry, '#000'));
  const irisClip = ctx.clip(E(ix, iy, irx, iry, '#000'));

  let inner = E(ix, iy, irx, iry, e.iris);
  inner += `<g clip-path="url(#${irisClip})">${E(ix, iy + iry * 0.75, irx * 0.85, iry * 0.55, mix(e.iris, '#ffffff', 0.32))}${E(
    ix,
    iy - iry * 0.75,
    irx * 1.1,
    iry * 0.55,
    mix(e.iris, '#000000', 0.38),
  )}</g>`;
  inner += C(ix, iy, ry * 0.42 * (e.pupil ?? 1), PUPIL);
  // catchlights (kept below a heavy lid so sleepy eyes still sparkle)
  const hy = Math.max(iy - ry * 0.36, lidY + ry * 0.3);
  inner += E(ix - rx * 0.3, hy, ry * 0.3 * shine, ry * 0.26 * shine, '#ffffff', rotA(-25, ix - rx * 0.3, hy));
  inner += C(ix + rx * 0.33, iy + ry * 0.33, ry * 0.12 * shine, '#ffffff');

  const x0 = e.x - rx - 3;
  const x1 = e.x + rx + 3;
  if (lid > 0) {
    const t = e.lidTilt ?? 0;
    const bul = ry * 0.32;
    const edge = `L ${n(x1)} ${n(lidY - t)} Q ${n(e.x)} ${n(lidY + bul)} ${n(x0)} ${n(lidY + t)}`;
    inner += P(`M ${n(x0)} ${n(e.y - ry - 4)} L ${n(x1)} ${n(e.y - ry - 4)} ${edge} Z`, e.lidFill ?? OL);
    inner += LN(`M ${n(x1)} ${n(lidY - t)} Q ${n(e.x)} ${n(lidY + bul)} ${n(x0)} ${n(lidY + t)}`, OL, 3.2);
  }
  const low = e.low ?? 0;
  if (low > 0) {
    const yb = e.y + ry - 2 * ry * low;
    inner += P(
      `M ${n(x0)} ${n(e.y + ry + 4)} L ${n(x1)} ${n(e.y + ry + 4)} L ${n(x1)} ${n(yb + 2)} Q ${n(e.x)} ${n(yb - ry * 0.55)} ${n(x0)} ${n(yb + 2)} Z`,
      e.lowFill ?? OL,
    );
    inner += LN(`M ${n(x1)} ${n(yb + 2)} Q ${n(e.x)} ${n(yb - ry * 0.55)} ${n(x0)} ${n(yb + 2)}`, OL, 2.6);
  }
  let s = '';
  if (e.rim) s += E(e.x, e.y, rx + 1.8, ry + 1.8, e.rim);
  s += E(e.x, e.y, rx, ry, OL);
  s += `<g clip-path="url(#${eyeClip})">${inner}</g>`;
  return s;
}

/** Arched eyebrow stroke. */
function brow(x: number, y: number, w: number, ang: number, arch: number, color: string, th = 5): string {
  return `<path d="M ${n(x - w / 2)} ${n(y)} Q ${n(x)} ${n(y - arch * 2)} ${n(x + w / 2)} ${n(y)}" fill="none" stroke="${color}" stroke-width="${th}" stroke-linecap="round"${rotA(ang, x, y)}/>`;
}

/** Dog nose centered at (x, y); returns markup. Bottom edge is y + h/2. */
function nose(x: number, y: number, w: number, h: number, fill: string, hi: string, nostril: string): string {
  const hw = w / 2;
  const d = `M ${n(x - hw)} ${n(y - h * 0.12)} C ${n(x - hw)} ${n(y - h * 0.62)}, ${n(x + hw)} ${n(y - h * 0.62)}, ${n(x + hw)} ${n(y - h * 0.12)} C ${n(x + hw)} ${n(y + h * 0.22)}, ${n(x + w * 0.16)} ${n(y + h * 0.5)}, ${n(x)} ${n(y + h * 0.5)} C ${n(x - w * 0.16)} ${n(y + h * 0.5)}, ${n(x - hw)} ${n(y + h * 0.22)}, ${n(x - hw)} ${n(y - h * 0.12)} Z`;
  let s = PO(d, fill, 2.6);
  s += E(x - w * 0.14, y - h * 0.26, w * 0.2, h * 0.12, hi, ` opacity="0.85"${rotA(-12, x - w * 0.14, y - h * 0.26)}`);
  const ny = y + h * 0.04;
  s += P(`M ${n(x - w * 0.34)} ${n(ny)} Q ${n(x - w * 0.22)} ${n(ny - h * 0.16)} ${n(x - w * 0.1)} ${n(ny + h * 0.06)} Q ${n(x - w * 0.24)} ${n(ny + h * 0.12)} ${n(x - w * 0.34)} ${n(ny)} Z`, nostril);
  s += P(mirror(`M ${n(x - w * 0.34)} ${n(ny)} Q ${n(x - w * 0.22)} ${n(ny - h * 0.16)} ${n(x - w * 0.1)} ${n(ny + h * 0.06)} Q ${n(x - w * 0.24)} ${n(ny + h * 0.12)} ${n(x - w * 0.34)} ${n(ny)} Z`, x), nostril);
  return s;
}

type MouthKind = 'smile' | 'grin' | 'flat' | 'smirk' | 'open' | 'o';
interface MouthSpec {
  kind: MouthKind;
  /** nose bottom */
  x: number;
  y: number;
  /** half width */
  w: number;
  /** philtrum length */
  h?: number;
  depth?: number;
  tongue?: number;
  tongueX?: number;
  flip?: boolean;
  ink?: string;
}

function mouth(m: MouthSpec): string {
  const { x, y, w } = m;
  const h = m.h ?? 5;
  const top = y + h;
  const ink = m.ink ?? OL;
  const sw = 2.8;
  const phil = `M ${n(x)} ${n(y)} L ${n(x)} ${n(top)}`;
  switch (m.kind) {
    case 'smile':
      return LN(`${phil} M ${n(x - w)} ${n(top - 3)} Q ${n(x - w * 0.5)} ${n(top + 3.6)} ${n(x)} ${n(top)} Q ${n(x + w * 0.5)} ${n(top + 3.6)} ${n(x + w)} ${n(top - 3)}`, ink, sw);
    case 'grin':
      return (
        LN(`${phil} M ${n(x - w)} ${n(top - 5)} Q ${n(x - w * 0.5)} ${n(top + 5)} ${n(x)} ${n(top)} Q ${n(x + w * 0.5)} ${n(top + 5)} ${n(x + w)} ${n(top - 5)}`, ink, sw) +
        LN(`M ${n(x - w - 2)} ${n(top - 8)} Q ${n(x - w - 0.5)} ${n(top - 5)} ${n(x - w + 1)} ${n(top - 3)} M ${n(x + w + 2)} ${n(top - 8)} Q ${n(x + w + 0.5)} ${n(top - 5)} ${n(x + w - 1)} ${n(top - 3)}`, ink, 2.2)
      );
    case 'flat':
      return LN(`${phil} M ${n(x - w * 0.85)} ${n(top + 1.5)} Q ${n(x - w * 0.4)} ${n(top + 2)} ${n(x)} ${n(top)} Q ${n(x + w * 0.4)} ${n(top + 2)} ${n(x + w * 0.85)} ${n(top + 1.5)}`, ink, sw);
    case 'smirk': {
      const d = `${phil} M ${n(x - w * 0.8)} ${n(top + 0.5)} Q ${n(x - w * 0.4)} ${n(top + 2.6)} ${n(x)} ${n(top)} Q ${n(x + w * 0.55)} ${n(top + 3.8)} ${n(x + w + 1)} ${n(top - 5)}`;
      return LN(m.flip ? mirror(d, x) : d, ink, sw);
    }
    case 'open': {
      const dp = m.depth ?? w * 1.15;
      const lip = `M ${n(x - w)} ${n(top - 3)} Q ${n(x - w * 0.5)} ${n(top + 3)} ${n(x)} ${n(top)} Q ${n(x + w * 0.5)} ${n(top + 3)} ${n(x + w)} ${n(top - 3)}`;
      const open = `${lip} C ${n(x + w * 0.95)} ${n(top + dp)}, ${n(x - w * 0.95)} ${n(top + dp)}, ${n(x - w)} ${n(top - 3)} Z`;
      let s = PO(open, MOUTH_IN, sw);
      const tl = m.tongue ?? 13;
      if (tl > 0) {
        const tx = m.tongueX ?? 0;
        const ty = top + dp * 0.3;
        const tw = w * 0.56;
        const td = `M ${n(x - tw + tx)} ${n(ty)} C ${n(x - tw - 1 + tx * 1.2)} ${n(ty + dp * 0.5 + tl)}, ${n(x + tw + 1 + tx * 1.2)} ${n(ty + dp * 0.5 + tl)}, ${n(x + tw + tx)} ${n(ty)} Q ${n(x + tx)} ${n(ty - 4)} ${n(x - tw + tx)} ${n(ty)} Z`;
        s += PO(td, TONGUE, 2.5);
        s += LN(`M ${n(x + tx)} ${n(ty + 1)} L ${n(x + tx * 1.15)} ${n(ty + dp * 0.25 + tl * 0.5)}`, TONGUE_SH, 2);
        s += E(x - tw * 0.45 + tx, ty + 4 + tl * 0.25, tw * 0.22, 2.4, '#ffb3c6', ' opacity="0.9"');
      }
      s += LN(phil, ink, sw);
      return s;
    }
    case 'o': {
      const oy = y + 3;
      const orx = Math.max(4, w * 0.36);
      const ory = Math.max(5, w * 0.44);
      return (
        LN(`M ${n(x)} ${n(y)} L ${n(x)} ${n(oy)}`, ink, sw) +
        EO(x, oy + ory, orx, ory, MOUTH_IN, 2.6, ink) +
        E(x, oy + ory * 1.45, orx * 0.55, ory * 0.32, TONGUE)
      );
    }
  }
}

// ---------------------------------------------------------------------------------------------
// The dogs. Each draws into a 240x240 space; head centered near (120, 100), chest at the bottom.
// ---------------------------------------------------------------------------------------------

interface DrawOpts {
  /** gaze override for both eyes */
  look?: Pt;
  /** extra head tilt in degrees */
  tilt?: number;
}

type DrawFn = (ctx: Ctx, mood: Mood, o: DrawOpts) => string;

// ---- Princess Donut: chocolate & tan Doberman/hound, liver nose, tan brow dots, gold tiara ----
const drawDonut: DrawFn = (ctx, mood, o) => {
  const c = {
    fur: '#5e3426', sh: '#43241a', hi: '#82503a',
    tan: '#cf7f40', tanSh: '#a8602c', tanHi: '#eaa868',
    inner: '#b4766d', nose: '#a8645e', noseHi: '#dba69d', nostril: '#5a2a2c',
    iris: '#c8832e', collar: '#2a2233', collarHi: '#4a3f5a',
    gold: '#f6c445', goldSh: '#cf931c', goldHi: '#fff3b0', gem: '#ff5fa2',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // shoulders + slim elegant neck
  const body = 'M 99 140 C 97 176, 76 194, 50 210 C 38 218, 33 234, 31 252 L 209 252 C 207 234, 202 218, 190 210 C 164 194, 143 176, 141 140 Z';
  s += ctx.region(
    body,
    c.fur,
    E(120, 166, 36, 20, c.sh) +
      P('M 164 198 C 186 210, 201 226, 209 252 L 158 252 C 169 236, 173 216, 164 198 Z', c.sh) +
      P('M 72 204 C 58 214, 49 230, 47 252 L 59 252 C 61 234, 67 220, 79 208 Z', c.hi),
  );
  // collar with a little gold ring
  s += PO('M 96 168 Q 120 182 144 168 L 146 181 Q 120 197 94 181 Z', c.collar, 2.6);
  s += LN('M 100 174 Q 120 186 140 174', c.collarHi, 1.6);
  s += CO(120, 196, 5.2, c.gold, 2.4) + C(118.5, 194.5, 1.6, c.goldHi);

  // ---- head group ----
  let h = '';
  // soft ears, folded back along the skull
  const earL = 'M 103 52 C 89 43, 70 47, 61 60 C 54 71, 54 88, 60 104 C 67 95, 76 85, 87 78 C 91 69, 97 59, 103 52 Z';
  const earIn = 'M 98 54 C 87 49, 74 53, 66 62 C 75 62, 84 65, 91 70 Z';
  const earSh = 'M 61 60 C 54 72, 54 88, 60 104 L 50 106 L 50 58 Z';
  const ea = A ? 16 : H ? -4 : 0;
  h += G(ctx.region(earL, c.fur, P(earIn, c.inner) + P(earSh, c.sh) + LN('M 66 64 C 64 76, 64 88, 66 96', c.sh, 2.2)), rot(ea, 96, 58));
  h += G(ctx.region(mirror(earL), c.fur, P(mirror(earIn), c.inner) + P(mirror('M 87 78 C 76 85, 67 95, 60 104 L 72 108 L 92 82 Z'), c.sh)), rot(-ea, 144, 58));

  const head = 'M 120 44 C 95 44, 78 58, 76 80 C 74 96, 79 108, 88 118 C 95 128, 99 142, 103 153 C 107 164, 133 164, 137 153 C 141 142, 145 128, 152 118 C 161 108, 166 96, 164 80 C 162 58, 145 44, 120 44 Z';
  h += ctx.region(
    head,
    c.fur,
    P('M 150 50 C 168 66, 170 98, 154 122 C 146 136, 142 152, 138 174 L 178 174 L 178 40 Z', c.sh) +
      E(103, 60, 15, 7, c.hi, rotA(-25, 103, 60)) +
      // tan cheeks + muzzle sides + chin
      P('M 70 106 C 84 108, 96 113, 106 119 L 134 119 C 144 113, 156 108, 170 106 L 170 190 L 70 190 Z', c.tan) +
      P('M 158 110 C 152 126, 146 144, 140 166 L 172 166 L 172 108 Z', c.tanSh) +
      E(96, 124, 5, 9, c.tanHi, rotA(-20, 96, 124)) +
      // chocolate bridge down to the nose
      P('M 104 100 C 108 112, 111 124, 112 136 L 128 136 C 129 124, 132 112, 136 100 Z', c.fur) +
      P('M 112 104 C 114 114, 115 122, 116 130 L 119 130 C 119 122, 119 114, 118 104 Z', c.hi, ' opacity="0.8"'),
  );
  // tan eyebrow dots (one cocked, naturally)
  const byL = A ? 65 : H ? 72 : 75;
  const byR = A ? 65 : H ? 72 : 70;
  const bA = A ? 22 : H ? 12 : 8;
  h += E(99, byL, 8, 5.6, c.tan, rotA(-bA, 99, byL)) + E(97.5, byL - 1.6, 3.6, 2.1, c.tanHi, rotA(-bA, 99, byL));
  h += E(141, byR, 8, 5.6, c.tan, rotA(bA + (A || H ? 0 : 10), 141, byR)) + E(139.5, byR - 1.6, 3.6, 2.1, c.tanHi, rotA(bA, 141, byR));

  const er = A ? 13.4 : H ? 12.8 : 12.4;
  const lid = A || H ? 0 : 0.34;
  const look = o.look ?? (A ? [0, -0.2] : [0, 0]);
  const eb = { r: er, iris: c.iris, lidFill: c.fur, look, pupil: A ? 0.72 : 1, shine: H ? 1.3 : 1 };
  h += eye(ctx, { ...eb, x: 99, y: 92, lid, lidTilt: -1.5 });
  h += eye(ctx, { ...eb, x: 141, y: 92, lid: lid * 0.8, lidTilt: 1.5 });

  h += nose(120, 140, 28, 17, c.nose, c.noseHi, c.nostril);
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smirk', x: 120, y: 148.5, w: 12, h: 4, depth: 15, tongue: 12 });

  // tiara (a little crooked, as is her right)
  const tiara = 'M 96 54 L 96 46 L 102 34 L 108 43 L 114 38 L 120 23 L 126 38 L 132 43 L 138 34 L 144 46 L 144 54 Q 120 46 96 54 Z';
  let t = ctx.region(tiara, c.gold, P('M 94 56 Q 120 48 146 56 L 146 49 Q 120 41 94 49 Z', c.goldSh) + P('M 120 23 L 114 38 L 118 40 Z', c.goldHi) + P('M 102 34 L 99 42 L 102 43 Z', c.goldHi), 2.6);
  t += CO(102, 34, 2.6, c.gold, 2) + CO(138, 34, 2.6, c.gold, 2) + CO(120, 22, 3, c.gold, 2);
  t += PO('M 120 35 L 125 41.5 L 120 48 L 115 41.5 Z', c.gem, 2.2) + P('M 120 36.5 L 117 41.5 L 120 41.5 Z', '#ffd3e6');
  h += G(t, rot(-9 + (A ? -4 : 0), 120, 52) + (A ? ' translate(0 -5)' : ''));

  const tilt = (A ? 3 : H ? -3 : -6) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 140));
  return s;
};

// ---- Griffin: heeler mix, white with black ticking, black mask/ears, white blaze, one floppy tip ----
const drawGriffin: DrawFn = (ctx, mood, o) => {
  const c = {
    w: '#f4f1ea', wSh: '#cdd0da', wHi: '#ffffff',
    b: '#1e1d25', bHi: '#3d3c4c', tick: '#3a3844', tickLt: '#7d7b89',
    inner: '#6c4c5c', innerHi: '#8e6a7a', nose: '#17151c', noseHi: '#5c5a68',
    iris: '#a8642c', rim: '#34323f', brow: '#8a87a0',
    band: '#4CC9F0', bandSh: '#2a9ccc', bandHi: '#a6ecff',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // lean shoulders
  const body = 'M 99 138 C 97 174, 76 190, 52 206 C 40 214, 35 232, 33 252 L 207 252 C 205 232, 200 214, 188 206 C 164 190, 143 174, 141 138 Z';
  s += ctx.region(
    body,
    c.w,
    E(120, 164, 34, 16, c.wSh) +
      P('M 172 202 C 190 212, 202 228, 207 252 L 168 252 C 177 236, 180 218, 172 202 Z', c.wSh) +
      speckles(11, 150, 120, 224, 88, 36, 0.5, 1.5, c.tickLt) + speckles(14, 70, 120, 224, 88, 36, 0.5, 1.4, c.tick) +
      P('M 26 206 C 42 194, 62 196, 72 212 C 78 228, 74 242, 72 254 L 20 254 Z', c.b) +
      P('M 33 208 C 44 201, 56 202, 64 210 C 52 208, 42 212, 36 218 Z', c.bHi),
  );
  // sky-blue bandana with a radar-dish pin
  const band = 'M 93 158 Q 120 172 147 158 L 149 168 C 141 184, 132 198, 120 208 C 108 198, 99 184, 91 168 Z';
  s += ctx.region(band, c.band, P('M 133 170 C 129 184, 125 196, 120 208 L 152 166 Z', c.bandSh) + LN('M 96 164 Q 120 176 144 164', c.bandHi, 1.8), 2.6);
  s += G(
    CO(0, 0, 7.5, '#eef3f9', 2.2) +
      P('M -4.6 -3 Q -3.6 3.8 3.4 3.2 Z', '#1f5d99') +
      LN('M -1 0 L 1.8 -2.6', '#1f5d99', 1.2) +
      LN('M -1.4 1.4 L -2.4 5.2 M -4.6 5.4 L 0.6 5.4', '#1f5d99', 1.3) +
      LN('M 2.6 -5.2 Q 5.2 -4.6 5.4 -1.8', '#2a9ccc', 1.1),
    'translate(120 184)',
  );

  // ---- head ----
  let h = '';
  // ears: big semi-erect triangles; viewer-right ear has a flopped tip
  const ea = A ? 9 : H ? -7 : 0;
  const earL = 'M 84 86 C 78 64, 70 42, 65 17 C 84 25, 102 42, 113 62 Z';
  const earLIn = 'M 89 78 C 85 62, 79 46, 74 31 C 86 39, 98 51, 106 63 Z';
  h += G(
    ctx.region(earL, c.b, P(earLIn, c.inner) + P('M 80 72 C 82 58, 78 44, 74 31 L 78 32 C 84 46, 88 60, 89 76 Z', c.innerHi) + LN('M 94 74 L 90 64 M 99 71 L 96 62', '#d8d4dd', 1.4) + P('M 65 17 C 70 42, 78 64, 84 86 L 76 88 L 58 16 Z', c.bHi)),
    rot(ea, 98, 74),
  );
  const earR = 'M 156 86 C 160 70, 165 54, 170 40 L 178 34 C 168 34, 150 44, 127 62 Z';
  const earRIn = 'M 151 78 C 155 64, 160 52, 165 42 C 156 46, 144 54, 134 63 Z';
  const flap = 'M 167 39 C 172 33, 178 31, 182 33 C 187 39, 191 48, 193 58 C 186 53, 178 46, 171 42 Z';
  h += G(
    ctx.region(earR, c.b, P(earRIn, c.inner) + LN('M 146 74 L 150 64 M 141 71 L 144 62', '#d8d4dd', 1.4)) + ctx.region(flap, c.b, P('M 180 33 C 186 40, 189 48, 191 56 L 196 56 L 186 30 Z', c.bHi)),
    rot(-ea, 142, 74),
  );

  const head = 'M 120 54 C 94 54, 78 68, 76 90 C 74 108, 80 120, 92 132 C 98 146, 104 160, 120 164 C 136 160, 142 146, 148 132 C 160 120, 166 108, 164 90 C 162 68, 146 54, 120 54 Z';
  const patchL = 'M 54 40 L 112 40 C 110 50, 113 60, 113 70 C 113 82, 112 94, 109 104 C 105 113, 97 118, 87 119 C 78 119, 70 113, 62 104 Z';
  h += ctx.region(
    head,
    c.w,
    // muzzle shading + ticking (the heeler "roan")
    P('M 146 116 C 144 134, 138 152, 120 166 L 168 166 L 168 110 Z', c.wSh) +
      E(104, 136, 7, 12, c.wHi, rotA(-15, 104, 136)) +
      E(120, 142, 30, 21, '#e2e1e6') +
      speckles(7, 150, 120, 140, 31, 23, 0.5, 1.5, c.tickLt) +
      speckles(17, 110, 120, 142, 30, 22, 0.5, 1.4, c.tick) +
      speckles(8, 50, 84, 124, 13, 10, 0.5, 1.4, c.tick) +
      speckles(9, 50, 156, 124, 13, 10, 0.5, 1.4, c.tick) +
      speckles(10, 14, 120, 76, 5, 16, 0.5, 1.1, c.tickLt) +
      // black mask patches with a white blaze between
      P(patchL, c.b) +
      P(mirror(patchL), c.b) +
      P('M 80 62 C 88 56, 98 56, 106 60 C 98 60, 90 62, 84 68 Z', c.bHi) +
      P(mirror('M 80 62 C 88 56, 98 56, 106 60 C 98 60, 90 62, 84 68 Z'), c.bHi) +
      speckles(12, 14, 74, 118, 10, 6, 0.7, 1.7, c.b) +
      speckles(13, 14, 166, 118, 10, 6, 0.7, 1.7, c.b) +
      E(120, 158, 15, 6, '#e2e1e6'),
  );

  const look = o.look ?? (A ? [0, -0.25] : [0.1, -0.1]);
  const er = A ? 12.6 : H ? 11.6 : 11.2;
  const eyeBase = { r: er, iris: c.iris, rim: c.rim, lidFill: c.b, look, pupil: A ? 0.7 : 1, shine: H ? 1.3 : 1 };
  h += eye(ctx, { ...eyeBase, x: 99, y: 97 });
  h += eye(ctx, { ...eyeBase, x: 141, y: 97 });

  const by = A ? 75 : H ? 80 : 82;
  const ba = A ? 14 : H ? 4 : -10;
  h += brow(99, by, 15, -ba, A ? 3 : 2, c.brow, 4.5);
  h += brow(141, by, 15, ba, A ? 3 : 2, c.brow, 4.5);

  h += nose(120, 144, 24, 16, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smile', x: 120, y: 152, w: 13, h: 5, depth: 16, tongue: 13 });

  const tilt = (A ? -3 : H ? 4 : 2) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 150));
  return s;
};

// ---- Pan: big glossy black lab/shepherd, floppy ears, grey chin, sleepy eyes, knit beanie ----
const drawPan: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#2b2a35', sh: '#17161d', hi: '#464d6b', gl: '#7d8dc4', mz: '#3a3947', mzHi: '#505672',
    grey: '#a3a1b2', nose: '#0e0d12', noseHi: '#6a7294', iris: '#a2622a', rim: '#1c1b22', brow: '#7179a0',
    hat: '#3fb4e6', hatSh: '#2584b4', hatHi: '#94e0ff', stripe: '#f3f7fb', pom: '#f5f0e4', pomSh: '#d6cdb9',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // big broad shoulders
  const body = 'M 86 148 C 80 180, 54 196, 28 212 C 16 222, 12 236, 10 252 L 230 252 C 228 236, 224 222, 212 212 C 186 196, 160 180, 154 148 Z';
  s += ctx.region(
    body,
    c.f,
    E(120, 174, 44, 18, c.sh, ' opacity="0.7"') +
      P('M 30 222 C 42 206, 62 198, 86 196 C 66 204, 50 214, 40 230 Z', c.hi) +
      LN('M 44 214 C 54 206, 66 202, 78 200', c.gl, 2.6, ' opacity="0.6"') +
      P('M 196 206 C 214 216, 226 232, 230 252 L 190 252 C 200 236, 202 220, 196 206 Z', c.sh),
  );

  let h = '';
  // soft floppy ears
  const ea = A ? 10 : H ? -4 : 0;
  const earL = 'M 102 60 C 86 48, 63 50, 51 62 C 41 74, 40 96, 46 118 C 58 112, 71 99, 86 84 Z';
  const earGl = 'M 55 64 C 48 78, 47 96, 50 110 C 45 96, 45 78, 55 64 Z';
  const earSh = 'M 86 84 C 74 96, 62 108, 46 118 L 60 124 L 92 90 Z';
  h += G(ctx.region(earL, c.f, P('M 51 62 C 41 74, 40 96, 46 118 L 54 112 C 50 96, 50 78, 58 64 Z', c.hi) + P(earGl, c.gl) + P(earSh, c.sh) + LN('M 62 60 C 70 66, 78 74, 84 82', c.sh, 3)), rot(ea, 92, 66));
  h += G(ctx.region(mirror(earL), c.f, P(mirror(earGl), c.hi) + P(mirror(earSh), c.sh) + LN(mirror('M 62 60 C 70 66, 78 74, 84 82'), c.sh, 3)), rot(-ea, 148, 66));

  const head = 'M 120 48 C 86 48, 64 68, 62 98 C 60 120, 70 138, 87 150 C 97 166, 106 178, 120 178 C 134 178, 143 166, 153 150 C 170 138, 180 120, 178 98 C 176 68, 154 48, 120 48 Z';
  const muzzle = 'M 99 114 C 94 130, 94 152, 101 166 C 108 178, 132 178, 139 166 C 146 152, 146 130, 141 114 C 134 108, 106 108, 99 114 Z';
  h += ctx.region(
    head,
    c.f,
    P('M 156 58 C 178 76, 182 112, 166 136 C 156 150, 146 166, 136 182 L 190 182 L 190 50 Z', c.sh) +
      // glossy sheen: forehead + rim light along the left of the skull
      E(102, 66, 22, 9, c.hi, rotA(-18, 102, 66)) +
      E(94, 63, 10, 3.6, c.gl, rotA(-22, 94, 63)) +
      LN('M 67 112 C 64 96, 68 80, 78 68', c.gl, 3, ' opacity="0.75"') +
      P(muzzle, c.mz) +
      P('M 103 116 C 99 130, 100 146, 104 158 C 106 146, 108 130, 112 116 Z', c.mzHi) +
      // grey chin ticking
      speckles(22, 70, 120, 172, 20, 7, 0.5, 1.3, c.grey) +
      speckles(23, 14, 105, 162, 6, 6, 0.4, 1, c.grey, 0.8) +
      speckles(24, 14, 135, 162, 6, 6, 0.4, 1, c.grey, 0.8),
  );

  const look = o.look ?? (A ? [0, -0.2] : [0, 0.15]);
  const lid = A ? 0.1 : H ? 0.3 : 0.44;
  const er = A ? 12.2 : 11.4;
  const eb = { r: er, iris: c.iris, rim: c.rim, lid, lidFill: c.f, look, pupil: A ? 0.8 : 0.95, shine: H ? 1.25 : 1.1 };
  h += eye(ctx, { ...eb, x: 98, y: 102, lidTilt: 2.4 });
  h += eye(ctx, { ...eb, x: 142, y: 102, lidTilt: -2.4 });

  const by = A ? 80 : H ? 86 : 88;
  h += brow(98, by, 16, A ? -10 : -4, A ? 3 : 1, c.brow, 4.5);
  h += brow(142, by, 16, A ? 10 : 4, A ? 3 : 1, c.brow, 4.5);

  h += nose(120, 150, 28, 18, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smile', x: 120, y: 159, w: 15, h: 5, depth: 16, tongue: 14, tongueX: H ? 4 : 0, ink: '#08070b' });

  // knit beanie
  let b = '';
  const crown = 'M 75 72 C 72 40, 95 22, 120 22 C 145 22, 168 40, 165 72 Z';
  b += ctx.region(crown, c.hat, P('M 144 26 C 160 36, 168 52, 166 72 L 150 72 C 154 56, 152 40, 144 26 Z', c.hatSh) + P('M 75 50 C 100 44, 140 44, 166 50 L 166 58 C 140 52, 100 52, 75 58 Z', c.stripe) + P('M 92 34 C 100 28, 110 26, 118 26 C 108 30, 100 34, 94 40 Z', c.hatHi));
  const cuff = 'M 70 70 C 92 62, 148 62, 170 70 L 168 86 C 146 77, 94 77, 72 86 Z';
  let ribs = '';
  for (let x = 78; x <= 162; x += 7) ribs += `M ${x} ${n(70 - Math.sin(((x - 70) / 100) * Math.PI) * 6)} L ${x} ${n(85 - Math.sin(((x - 70) / 100) * Math.PI) * 8)}`;
  b += ctx.region(cuff, c.hat, LN(ribs, c.hatSh, 2.2) + P('M 150 66 C 160 67, 166 69, 170 70 L 168 86 C 162 82, 156 80, 150 79 Z', c.hatSh, ' opacity="0.7"'));
  b += ctx.region(bumpy(120, 19, 9, 1.4, 11), c.pom, C(124, 23, 5, c.pomSh) + C(116, 15, 3.4, '#ffffff'), 2.6);
  h += G(b, rot(7, 120, 70) + (A ? ' translate(0 -4)' : ''));

  const tilt = (A ? -2 : H ? 3 : 4) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 160));
  return s;
};

// ---- Ranger Huckleberry: shaggy black-and-brown Newfie, campaign hat, yellow slicker, rain ----
const drawHuckleberry: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#2a2120', sh: '#171110', hi: '#4b3c37', br: '#7d4c2d', brSh: '#5b3519', brHi: '#a26a42',
    nose: '#110d0e', noseHi: '#5b4c4a', iris: '#8a5426', rim: '#1a1413',
    hat: '#c69c59', hatSh: '#9d773d', hatHi: '#e3c287', band: '#4a3020',
    sl: '#ffd23f', slSh: '#e0a20e', slHi: '#fff1a0', drop: '#8fd8ff', dropHi: '#e8f8ff',
    pine: '#3DDC84',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // slicker
  const slicker = 'M 24 252 C 26 232, 32 214, 50 204 C 70 192, 94 186, 120 186 C 146 186, 170 192, 190 204 C 208 214, 214 232, 216 252 Z';
  s += ctx.region(slicker, c.sl, P('M 176 196 C 198 206, 212 226, 216 252 L 172 252 C 182 232, 186 214, 176 196 Z', c.slSh) + P('M 52 210 C 40 220, 34 234, 32 252 L 44 252 C 46 236, 50 222, 60 212 Z', c.slHi));
  // shaggy chest fur in the open collar
  s += ctx.region('M 92 168 L 148 168 L 136 210 L 128 214 L 124 226 L 120 218 L 116 226 L 112 214 L 104 210 Z', c.f, P('M 104 186 L 136 186 L 130 214 L 110 214 Z', c.br) + LN('M 112 192 L 114 204 M 126 192 L 124 204', c.brSh, 1.8));
  // popped collar
  const flapL = 'M 118 230 L 84 204 C 74 192, 68 178, 68 162 L 98 170 C 102 190, 108 210, 118 230 Z';
  s += ctx.region(flapL, c.sl, P('M 68 162 L 76 165 C 78 184, 86 198, 98 210 L 84 204 C 74 192, 68 178, 68 162 Z', c.slHi));
  s += ctx.region(mirror(flapL), c.sl, P(mirror('M 98 170 C 102 190, 108 210, 118 230 L 110 228 C 102 210, 98 192, 94 170 Z'), c.slSh));
  s += CO(120, 240, 3.2, '#3a3a44', 2) + CO(120, 252, 3.2, '#3a3a44', 2);

  let h = '';
  // big droopy ears
  const ea = A ? 12 : 0;
  const earL = 'M 88 64 C 66 60, 50 74, 45 96 C 41 116, 43 134, 48 150 C 54 146, 57 152, 61 146 C 66 148, 70 140, 70 134 C 74 118, 80 96, 92 74 Z';
  h += G(ctx.region(earL, c.f, P('M 47 96 C 44 116, 46 134, 50 148 L 36 150 L 38 90 Z', c.hi) + LN('M 60 100 C 58 114, 58 126, 60 138', c.sh, 2.4)), rot(ea, 86, 70));
  h += G(ctx.region(mirror(earL), c.f, P(mirror('M 70 134 C 74 118, 80 96, 92 74 L 96 80 C 86 100, 80 120, 76 140 Z'), c.sh) + LN(mirror('M 60 100 C 58 114, 58 126, 60 138'), c.sh, 2.4)), rot(-ea, 154, 70));

  const head = 'M 120 50 C 88 50, 66 66, 62 94 C 60 104, 57 112, 52 120 C 58 120, 61 123, 58 131 C 64 131, 67 135, 65 143 C 72 143, 76 148, 77 154 C 86 152, 94 162, 100 170 C 108 177, 132 177, 140 170 C 146 162, 154 152, 163 154 C 164 148, 168 143, 175 143 C 173 135, 176 131, 182 131 C 179 123, 182 120, 188 120 C 183 112, 180 104, 178 94 C 174 66, 152 50, 120 50 Z';
  const muzzle = 'M 93 118 C 86 128, 84 148, 92 160 C 102 172, 138 172, 148 160 C 156 148, 154 128, 147 118 C 138 110, 102 110, 93 118 Z';
  h += ctx.region(
    head,
    c.f,
    P('M 160 70 C 178 90, 180 120, 168 140 C 160 152, 150 164, 140 180 L 196 180 L 196 60 Z', c.sh) +
      E(120, 78, 64, 12, c.sh) +
      P('M 66 104 C 62 114, 60 124, 64 134 C 66 124, 68 114, 72 108 Z', c.hi) +
      P(muzzle, c.br) +
      P('M 140 116 C 150 126, 152 146, 144 162 L 160 160 L 156 116 Z', c.brSh) +
      E(104, 128, 6, 10, c.brHi, rotA(-15, 104, 128)) +
      LN('M 98 156 C 102 164, 110 168, 118 168 M 142 156 C 138 164, 130 168, 122 168', c.brSh, 2) +
      // shaggy brown cheek fluff
      P('M 70 132 C 76 128, 82 130, 86 134 C 80 136, 78 140, 80 146 C 74 142, 70 140, 70 132 Z', c.br) +
      P(mirror('M 70 132 C 76 128, 82 130, 86 134 C 80 136, 78 140, 80 146 C 74 142, 70 140, 70 132 Z'), c.brSh),
  );
  // tired eyes with bags
  const lid = A ? 0.04 : H ? 0.26 : 0.54;
  const look = o.look ?? [0, A ? -0.2 : 0];
  const eb = { r: A ? 11.6 : 10.6, iris: c.iris, rim: c.rim, lid, lidFill: c.f, look, pupil: A ? 0.75 : 1, shine: H ? 1.25 : 1 };
  h += eye(ctx, { ...eb, x: 98, y: 102, lidTilt: 1.5 });
  h += eye(ctx, { ...eb, x: 142, y: 102, lidTilt: -1.5 });
  if (!H) h += LN('M 89 117 Q 98 121 107 117 M 133 117 Q 142 121 151 117', '#6a5550', 2);
  // brown brow dots
  const by = A ? 80 : H ? 86 : 89;
  const bAng = A ? 16 : 6;
  h += E(97, by, 8, 5, c.br, rotA(-bAng, 97, by)) + E(143, by, 8, 5, c.br, rotA(bAng, 143, by));

  h += nose(120, 132, 30, 20, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'flat', x: 120, y: 142, w: 16, h: 7, depth: 14, tongue: 11 });

  // flat-brimmed campaign hat (Montana peak) with a little pine pin
  let hat = '';
  const brim = 'M 36 68 C 36 56, 204 56, 204 68 C 204 80, 36 80, 36 68 Z';
  hat += ctx.region(brim, c.hat, P('M 36 70 C 60 78, 180 78, 204 70 L 204 82 L 36 82 Z', c.hatSh) + LN('M 50 64 C 90 59, 150 59, 190 64', c.hatHi, 2));
  // Montana-peak crown: four dents pinched to a point
  const crown = 'M 82 64 C 83 48, 92 34, 104 26 C 110 22, 116 18, 120 14 C 124 18, 130 22, 136 26 C 148 34, 157 48, 158 64 Z';
  hat += ctx.region(
    crown,
    c.hat,
    P('M 136 26 C 148 34, 157 48, 158 64 L 140 64 C 140 50, 138 38, 130 24 Z', c.hatSh) +
      P('M 120 14 C 114 26, 110 40, 110 56 L 104 56 C 102 42, 104 30, 110 22 Z', c.hatHi, ' opacity="0.85"') +
      P('M 120 14 C 124 26, 128 40, 130 56 L 124 56 C 124 40, 122 28, 120 14 Z', c.hatSh, ' opacity="0.8"') +
      LN('M 101 30 C 98 38, 96 46, 96 54 M 139 30 C 142 38, 144 46, 144 54', c.hatSh, 2.2) +
      P('M 80 54 C 100 58, 140 58, 160 54 L 160 66 L 80 66 Z', c.band),
  );
  hat += PO('M 120 52 L 126 62 L 122 62 L 122 64.5 L 118 64.5 L 118 62 L 114 62 Z', c.pine, 1.6);
  // rain on the brim
  const drop = (x: number, y: number, k: number): string =>
    PO(`M ${x} ${y - 7 * k} C ${x + 4.5 * k} ${y - 1 * k}, ${x + 4.5 * k} ${y + 4 * k}, ${x} ${y + 4 * k} C ${x - 4.5 * k} ${y + 4 * k}, ${x - 4.5 * k} ${y - 1 * k}, ${x} ${y - 7 * k} Z`, c.drop, 1.8) +
    E(x - 1.4 * k, y + 0.6 * k, 1.1 * k, 1.6 * k, c.dropHi);
  // beads of rain sitting on the brim (falling drops stay well away from the face)
  hat += E(96, 59, 2.6, 1.4, c.dropHi, ' opacity="0.9"') + E(150, 60, 2, 1.1, c.dropHi, ' opacity="0.9"') + E(62, 65, 2, 1.1, c.dropHi, ' opacity="0.8"') + E(182, 66, 2.2, 1.2, c.dropHi, ' opacity="0.8"');
  h += G(hat, A ? 'translate(0 -6)' : '');

  const tilt = (A ? 2 : H ? -2 : 0) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 160));
  // a few raindrops in the air
  s += drop(26, 36, 1.1) + drop(214, 30, 1) + drop(62, 18, 0.8) + drop(212, 150, 0.8) + drop(26, 166, 0.8);
  return s;
};

// ---- Coach Tank: Blue Lacy, slate coat, amber eyes, folded ears, ball cap, whistle on lanyard ----
const drawTank: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#707e8f', sh: '#566272', hi: '#97a6b8', mz: '#7f8c9c',
    nose: '#3c434e', noseHi: '#7d8794', iris: '#f0a531', rim: '#4b5563', brow: '#46505e',
    cap: '#e8403f', capSh: '#b72d2d', capHi: '#ff7d73',
    polo: '#25314f', poloSh: '#18213a', poloHi: '#36466f', lan: '#ff4d4d',
    wh: '#d2d9e2', whSh: '#9aa4b0', whHi: '#ffffff',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // neck fur, then the coach polo
  s += ctx.region('M 96 140 L 144 140 L 150 200 L 90 200 Z', c.f, E(120, 160, 30, 16, c.sh));
  const polo = 'M 22 252 C 24 230, 32 212, 50 202 C 68 192, 86 186, 96 178 L 108 196 L 120 206 L 132 196 L 144 178 C 154 186, 172 192, 190 202 C 208 212, 216 230, 218 252 Z';
  s += ctx.region(polo, c.polo, P('M 176 196 C 198 206, 212 226, 218 252 L 172 252 C 182 232, 186 214, 176 196 Z', c.poloSh) + P('M 52 208 C 40 218, 34 234, 32 252 L 44 252 C 46 236, 52 222, 62 212 Z', c.poloHi) + LN('M 120 206 L 120 236', c.poloSh, 2) + C(124, 216, 2, c.poloHi) + C(124, 228, 2, c.poloHi));
  // collar points
  const colL = 'M 96 176 L 120 206 L 104 214 L 86 186 Z';
  s += ctx.region(colL, c.polo, P('M 86 186 L 104 214 L 98 212 L 84 190 Z', c.poloHi));
  s += ctx.region(mirror(colL), c.polo, P(mirror('M 96 176 L 120 206 L 114 208 L 94 182 Z'), c.poloSh));
  // lanyard + whistle
  s += LN('M 100 176 C 104 196, 114 212, 124 222 M 140 176 C 138 196, 132 212, 126 222', OL, 5.2) + LN('M 100 176 C 104 196, 114 212, 124 222 M 140 176 C 138 196, 132 212, 126 222', c.lan, 2.6);
  s += G(
    PO('M -12 -6 L 6 -6 C 12 -6, 14 -2, 14 2 C 14 8, 10 10, 6 10 C 0 10, -2 6, -2 4 L -12 4 Z', c.wh, 2.4) +
      P('M -11 1 L -2 1 C -1 6, 2 8, 6 8 C 10 8, 12 6, 13 3 C 13 8, 10 10, 6 10 C 0 10, -2 6, -2 4 L -12 4 Z', c.whSh) +
      LN('M -10 -3.5 L 4 -3.5', c.whHi, 1.6) +
      E(4, -2, 3, 1.5, OL) +
      CO(-6, -8, 2.6, 'none', 1.8, '#9aa4b0'),
    'translate(126 230) rotate(-12)',
  );

  let h = '';
  // high-set ears that fold over and hang beside the cheeks
  const ea = A ? 16 : H ? -4 : 0;
  const earL = 'M 98 70 C 84 62, 66 64, 58 74 C 52 86, 54 102, 60 116 C 66 106, 74 96, 86 90 Z';
  const earFold = 'M 96 70 C 84 65, 70 67, 62 76 C 72 76, 80 79, 88 84 Z';
  h += G(ctx.region(earL, c.f, P(earFold, c.sh) + LN('M 62 76 C 72 76, 80 79, 88 84', OL, 1.8) + P('M 58 74 C 52 86, 54 102, 60 116 L 50 116 L 50 72 Z', c.hi)), rot(ea, 90, 74));
  h += G(ctx.region(mirror(earL), c.f, P(mirror(earFold), c.sh) + LN(mirror('M 62 76 C 72 76, 80 79, 88 84'), OL, 1.8) + P(mirror('M 86 90 C 74 96, 66 106, 60 116 L 70 120 L 90 94 Z'), c.sh)), rot(-ea, 150, 74));

  const head = 'M 120 54 C 94 54, 77 70, 76 92 C 75 108, 80 120, 90 132 C 97 147, 104 160, 120 165 C 136 160, 143 147, 150 132 C 160 120, 165 108, 164 92 C 163 70, 146 54, 120 54 Z';
  h += ctx.region(
    head,
    c.f,
    P('M 150 64 C 166 80, 168 110, 152 132 C 144 146, 138 158, 128 170 L 176 170 L 176 60 Z', c.sh) +
      E(120, 86, 46, 9, c.sh, ' opacity="0.8"') +
      P('M 99 116 C 94 128, 96 146, 104 156 C 110 164, 130 164, 136 156 C 144 146, 146 128, 141 116 C 134 110, 106 110, 99 116 Z', c.mz) +
      E(106, 128, 5, 10, c.hi, rotA(-12, 106, 128)),
  );

  const look = o.look ?? [0, A ? -0.25 : 0];
  const er = A ? 12.4 : 11.2;
  const lid = A ? 0 : H ? 0 : 0.2;
  const eb = { r: er, iris: c.iris, rim: c.rim, lid, lidFill: c.f, look, pupil: A ? 0.62 : 0.85, shine: H ? 1.25 : 1.05 };
  h += eye(ctx, { ...eb, x: 100, y: 102, lidTilt: -2 });
  h += eye(ctx, { ...eb, x: 140, y: 102, lidTilt: 2 });
  const by = A ? 80 : H ? 86 : 90;
  const ba = A ? -14 : H ? -8 : 8;
  h += brow(100, by, 17, ba, A ? 3 : 1, c.brow, 5.2);
  h += brow(140, by, 17, -ba, A ? 3 : 1, c.brow, 5.2);

  h += nose(120, 144, 25, 16, c.nose, c.noseHi, '#191c22');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'grin', x: 120, y: 152, w: H ? 15 : 16, h: 4, depth: 19, tongue: 10 });

  // ball cap with a lone star
  let cap = '';
  const crown = 'M 74 76 C 72 46, 94 26, 120 26 C 146 26, 168 46, 166 76 Z';
  cap += ctx.region(
    crown,
    c.cap,
    P('M 142 30 C 160 40, 168 58, 166 76 L 146 76 C 150 60, 150 44, 142 30 Z', c.capSh) +
      P('M 90 40 C 98 32, 108 28, 116 28 C 106 34, 98 40, 94 48 Z', c.capHi) +
      LN('M 120 27 C 112 40, 106 56, 104 76 M 120 27 C 128 40, 134 56, 136 76', c.capSh, 1.6),
  );
  cap += PO(starPath(120, 56, 9, 3.9), '#ffffff', 2);
  cap += CO(120, 26, 3.4, c.cap, 2.2);
  const bill = 'M 70 74 C 92 66, 148 66, 170 74 C 176 80, 172 88, 164 88 C 140 82, 100 82, 76 88 C 68 88, 64 80, 70 74 Z';
  cap += ctx.region(bill, c.cap, P('M 66 80 C 90 76, 150 76, 174 80 L 174 92 L 66 92 Z', c.capSh) + LN('M 80 74 C 100 70, 140 70, 160 74', c.capHi, 1.8));
  h += G(cap, rot(-4, 120, 70) + (A ? ' translate(0 -5)' : ''));

  const tilt = (A ? 2 : H ? -3 : -2) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 150));
  return s;
};

/** 5-point star centered at (cx, cy). */
function starPath(cx: number, cy: number, ro: number, ri: number): string {
  let d = '';
  for (let i = 0; i < 10; i++) {
    const r = i % 2 === 0 ? ro : ri;
    const a = -Math.PI / 2 + (i * Math.PI) / 5;
    d += `${i === 0 ? 'M' : 'L'} ${n(cx + Math.cos(a) * r)} ${n(cy + Math.sin(a) * r)} `;
  }
  return d + 'Z';
}

// ---- Lucky: fawn pug with a black mask, green dealer's visor, red bow tie, pit-boss vest ----
const drawLucky: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#e3b677', sh: '#c4935a', hi: '#f5d6a2', wr: '#a87844',
    m: '#3b3038', mHi: '#5d4e5a', nose: '#0d090c', noseHi: '#8a7a88', iris: '#5a3319',
    vis: '#2fbf71', visSh: '#1d8a50', visHi: '#9ff5c4',
    bow: '#e3303d', bowSh: '#a81f2b', bowHi: '#ff7c86', shirt: '#f6f6fa', shirtSh: '#d5d6e2',
    vest: '#2d2444', vestSh: '#1c1630', vestHi: '#443a66', gold: '#f6c445',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // vest + shirt + bow tie
  s += ctx.region('M 90 140 L 150 140 L 156 196 L 84 196 Z', c.f, E(120, 160, 40, 16, c.sh));
  const vest = 'M 22 252 C 24 230, 32 210, 52 200 C 70 190, 86 184, 94 172 L 146 172 C 154 184, 170 190, 188 200 C 208 210, 216 230, 218 252 Z';
  s += ctx.region(vest, c.vest, P('M 176 196 C 198 206, 212 226, 218 252 L 172 252 C 182 232, 186 214, 176 196 Z', c.vestSh) + P('M 52 206 C 40 216, 34 232, 32 252 L 44 252 C 46 236, 52 220, 62 210 Z', c.vestHi));
  s += ctx.region('M 96 172 L 144 172 L 132 214 L 120 236 L 108 214 Z', c.shirt, P('M 132 172 L 144 172 L 132 214 L 126 224 Z', c.shirtSh) + LN('M 120 196 L 120 236', c.shirtSh, 1.6) + C(120, 206, 1.8, c.shirtSh) + C(120, 220, 1.8, c.shirtSh));
  // lapels
  s += PO('M 94 172 L 108 214 L 120 238 L 102 252 L 84 252 C 82 220, 86 192, 94 172 Z', c.vest, 2.6);
  s += PO(mirror('M 94 172 L 108 214 L 120 238 L 102 252 L 84 252 C 82 220, 86 192, 94 172 Z'), c.vest, 2.6);
  s += CO(103, 236, 2.4, c.gold, 1.6) + CO(137, 236, 2.4, c.gold, 1.6);
  // collar points + bow tie
  s += PO('M 98 170 L 118 182 L 104 192 Z', c.shirt, 2.2) + PO(mirror('M 98 170 L 118 182 L 104 192 Z'), c.shirt, 2.2);
  const wing = 'M 118 184 C 110 176, 100 172, 95 174 C 92 180, 92 190, 95 196 C 100 197, 110 192, 118 186 Z';
  s += ctx.region(wing, c.bow, P('M 95 189 C 102 190, 110 188, 118 185 L 118 200 L 90 200 Z', c.bowSh) + LN('M 99 178 C 104 178, 110 181, 114 183', c.bowHi, 1.6), 2.6);
  s += ctx.region(mirror(wing), c.bow, P(mirror('M 95 189 C 102 190, 110 188, 118 185 L 118 200 L 90 200 Z'), c.bowSh), 2.6);
  s += PO('M 114 179 L 126 179 C 128 183, 128 187, 126 191 L 114 191 C 112 187, 112 183, 114 179 Z', c.bow, 2.4) + P('M 116 181 L 122 181 L 121 184 L 116 184 Z', c.bowHi);

  let h = '';
  // small black button ears
  const ea = A ? 14 : 0;
  const earL = 'M 90 60 C 76 56, 60 62, 53 74 C 50 82, 51 90, 55 96 C 62 88, 72 80, 86 76 Z';
  const earFold = 'M 88 61 C 77 59, 65 64, 58 72 C 67 71, 76 72, 84 75 Z';
  h += G(ctx.region(earL, c.m, P(earFold, c.mHi) + LN('M 58 72 C 67 71, 76 72, 84 75', '#140f13', 1.6)), rot(ea, 84, 68));
  h += G(ctx.region(mirror(earL), c.m, P(mirror(earFold), c.mHi) + LN(mirror('M 58 72 C 67 71, 76 72, 84 75'), '#140f13', 1.6)), rot(-ea, 156, 68));

  const head = 'M 120 50 C 82 50, 61 76, 61 108 C 61 142, 86 168, 120 168 C 154 168, 179 142, 179 108 C 179 76, 158 50, 120 50 Z';
  const mask = 'M 120 100 C 104 100, 94 110, 94 124 C 94 146, 106 162, 120 162 C 134 162, 146 146, 146 124 C 146 110, 136 100, 120 100 Z';
  const eyeY = 106;
  h += ctx.region(
    head,
    c.f,
    P('M 156 62 C 178 80, 182 120, 166 146 C 156 160, 140 168, 120 172 L 190 172 L 190 56 Z', c.sh) +
      E(98, 64, 18, 7, c.hi, rotA(-20, 98, 64)) +
      E(70, 120, 6, 12, c.hi, ' opacity="0.7"') +
      // dark mask: eye rings + muzzle
      E(94, eyeY, 19, 18, c.m) +
      E(146, eyeY, 19, 18, c.m) +
      P(mask, c.m) +
      E(110, 116, 6, 8, c.mHi, ' opacity="0.9"') +
      // forehead wrinkles
      LN('M 104 84 Q 120 78 136 84 M 108 92 Q 120 88 132 92 M 120 76 L 120 92', c.wr, 2.4) +
      // cheek puff lines
      LN('M 82 140 Q 90 150 100 154 M 158 140 Q 150 150 140 154', c.sh, 2.2),
  );
  // big round eyes
  const look = o.look ?? [0, A ? -0.2 : 0];
  const er = A ? 14.2 : 12.6;
  const lidL = A ? 0 : H ? 0 : 0.38;
  const lidR = A ? 0 : H ? 0 : 0.18;
  const eb = { r: er, ratio: 1, iris: c.iris, lidFill: c.m, look, pupil: A ? 0.68 : 0.95, shine: H ? 1.3 : 1.15 };
  h += eye(ctx, { ...eb, x: 94, y: eyeY, lid: lidL, lidTilt: -2 });
  h += eye(ctx, { ...eb, x: 146, y: eyeY, lid: lidR, lidTilt: 1 });
  // brows
  const byL = A ? 80 : H ? 86 : 90;
  const byR = A ? 80 : H ? 86 : 85;
  h += brow(94, byL, 16, A ? -14 : H ? -6 : 8, A ? 3 : 1.5, c.wr, 4.6);
  h += brow(146, byR, 16, A ? 14 : H ? 6 : 10, A ? 3 : 2.2, c.wr, 4.6);

  // nose roll + flat nose
  h += LN('M 106 118 Q 120 110 134 118', c.mHi, 2.4);
  h += nose(120, 126, 24, 13, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smirk', x: 120, y: 132.5, w: 13, h: 6, depth: 14, tongue: 13, tongueX: H ? -3 : 0, ink: '#060406' });

  // green dealer's visor
  const visor = 'M 63 80 C 80 58, 160 58, 177 80 C 172 90, 160 94, 150 91 C 134 86, 106 86, 90 91 C 80 94, 68 90, 63 80 Z';
  let v = LN('M 66 78 C 78 64, 162 64, 174 78', c.visSh, 4);
  v += PO(visor, c.vis, 2.8, OL, ' fill-opacity="0.86"');
  v += LN('M 76 76 C 96 66, 144 66, 164 76', c.visHi, 2.4, ' opacity="0.8"');
  v += LN('M 70 84 C 74 88, 82 90, 90 89', c.visSh, 2);
  h += G(v, rot(-5, 120, 76) + (A ? ' translate(0 -5)' : ''));

  const tilt = (A ? 0 : H ? 3 : 5) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 150));
  return s;
};

// ---- Agent Sorghum: red-and-white Pembroke corgi, huge ears, mesh seed cap, wheat stalk ----
const drawSorghum: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#d8812e', sh: '#b2621b', hi: '#f1a659', w: '#fbf6ee', wSh: '#e2d4c0',
    inner: '#f2b6a0', innerSh: '#d98f7e', nose: '#17141a', noseHi: '#5f5a66', iris: '#5c3216', brow: '#8f4b14',
    capF: '#f6f0de', capFSh: '#d9cfb3', mesh: '#3f8f46', meshSh: '#2b6b32', patch: '#F2E86D',
    wheat: '#e9c45c', wheatSh: '#bf9733', stem: '#d9b250',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // red shoulders with a fluffy white bib
  const body = 'M 94 144 C 90 176, 68 190, 46 204 C 34 214, 30 232, 28 252 L 212 252 C 210 232, 206 214, 194 204 C 172 190, 150 176, 146 144 Z';
  s += ctx.region(
    body,
    c.f,
    P('M 174 198 C 194 210, 206 228, 212 252 L 170 252 C 178 234, 182 216, 174 198 Z', c.sh) +
      P('M 86 150 L 154 150 C 164 176, 170 204, 168 254 L 72 254 C 70 204, 76 176, 86 150 Z', c.w) +
      P('M 72 200 L 66 206 L 74 210 L 68 218 L 76 222 L 72 230 L 78 232 Z', c.w) +
      P(mirror('M 72 200 L 66 206 L 74 210 L 68 218 L 76 222 L 72 230 L 78 232 Z'), c.w) +
      E(120, 166, 34, 14, c.wSh) +
      P('M 150 176 C 158 200, 162 226, 162 254 L 168 254 C 170 220, 164 192, 154 162 Z', c.wSh),
  );

  let h = '';
  // big upright ears
  const ea = A ? 6 : H ? -8 : 0;
  const es = A ? 1.08 : 1;
  const earL = 'M 84 88 C 74 66, 68 42, 70 18 C 71 13, 76 12, 80 15 C 96 30, 106 46, 112 68 Z';
  const earIn = 'M 88 80 C 81 62, 77 44, 77 26 C 89 38, 97 52, 103 68 Z';
  const fluff = 'M 87 82 L 84 72 L 90 76 L 89 66 L 94 72 L 95 63 L 98 72 L 102 66 L 101 76 Z';
  h += G(ctx.region(earL, c.f, P(earIn, c.inner) + P('M 77 26 C 77 44, 81 62, 88 80 L 84 82 C 76 62, 72 44, 73 26 Z', c.innerSh) + P(fluff, c.w) + P('M 70 18 C 68 42, 74 66, 84 88 L 76 90 L 62 16 Z', c.hi)), `${rot(ea, 98, 80)} translate(98 80) scale(1 ${es}) translate(-98 -80)`);
  h += G(ctx.region(mirror(earL), c.f, P(mirror(earIn), c.inner) + P(mirror(fluff), c.w) + P(mirror('M 112 68 C 106 46, 96 30, 80 15 L 86 12 C 100 28, 110 46, 116 66 Z'), c.sh)), `${rot(-ea, 142, 80)} translate(142 80) scale(1 ${es}) translate(-142 -80)`);

  const head = 'M 120 62 C 92 62, 72 78, 70 100 C 68 112, 70 120, 74 128 C 68 130, 66 134, 70 138 C 76 138, 80 140, 84 142 C 94 152, 106 162, 120 163 C 134 162, 146 152, 156 142 C 160 140, 164 138, 170 138 C 174 134, 172 130, 166 128 C 170 120, 172 112, 170 100 C 168 78, 148 62, 120 62 Z';
  const blaze = 'M 114 58 L 126 58 C 126 78, 128 92, 136 104 C 146 114, 162 118, 178 124 L 182 180 L 58 180 L 62 124 C 78 118, 94 114, 104 104 C 112 92, 114 78, 114 58 Z';
  h += ctx.region(
    head,
    c.f,
    P('M 150 70 C 168 84, 172 106, 166 124 L 180 124 L 180 64 Z', c.sh) +
      E(100, 74, 15, 6, c.hi, rotA(-18, 100, 74)) +
      P(blaze, c.w) +
      P('M 140 116 C 146 132, 140 150, 122 166 L 170 166 L 170 120 Z', c.wSh) +
      E(118, 70, 2.2, 8, '#ffffff', ' opacity="0.7"'),
  );

  const look = o.look ?? [0, A ? -0.25 : 0];
  const er = A ? 12 : H ? 11.2 : 10.8;
  const eb = { r: er, iris: c.iris, lidFill: c.f, look, low: H || A ? 0 : 0.12, lowFill: c.f, pupil: A ? 0.72 : 1, shine: H ? 1.3 : 1 };
  h += eye(ctx, { ...eb, x: 99, y: 102 });
  h += eye(ctx, { ...eb, x: 141, y: 102 });
  const by = A ? 81 : H ? 86 : 88;
  h += brow(99, by, 14, A ? -14 : -6, A ? 3 : 2, c.brow, 4.6);
  h += brow(141, by, 14, A ? 14 : 6, A ? 3 : 2, c.brow, 4.6);

  h += nose(120, 138, 22, 15, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smile', x: 120, y: 145.5, w: 13, h: 5, depth: 15, tongue: 12 });

  if (mood === 'neutral') {
    // a wheat stalk, chewed thoughtfully
    let wh = LN('M 128 150 L 188 118', OL, 5.4) + LN('M 128 150 L 188 118', c.stem, 2.6);
    let grains = '';
    for (let i = 0; i < 5; i++) {
      const gx = 4 + i * 5.2;
      grains += EO(gx, -3.4, 4.2, 2.3, c.wheat, 1.5, OL, rotA(-28, gx, -3.4)) + EO(gx, 3.4, 4.2, 2.3, c.wheat, 1.5, OL, rotA(28, gx, 3.4));
    }
    grains += EO(30, 0, 4.4, 2.4, c.wheat, 1.5);
    grains += LN('M 31 0 L 40 -2 M 28 -4 L 36 -9 M 28 4 L 36 8', c.wheatSh, 1.2);
    wh += G(grains, 'translate(186 119) rotate(-28)');
    h += wh;
  }

  // mesh seed cap perched between the ears
  let cap = '';
  const crown = 'M 90 70 C 88 48, 102 32, 120 32 C 138 32, 152 48, 150 70 Z';
  let grid = '';
  for (let x = 84; x <= 156; x += 5) grid += `M ${x} 30 L ${x} 72 `;
  for (let y = 34; y <= 72; y += 5) grid += `M 84 ${y} L 156 ${y} `;
  cap += ctx.region(
    crown,
    c.mesh,
    LN(grid, c.meshSh, 1.2) +
      P('M 101 70 C 100 50, 108 35, 120 33 C 132 35, 140 50, 139 70 Z', c.capF) +
      P('M 130 37 C 137 46, 140 58, 139 70 L 132 70 C 133 58, 132 46, 128 38 Z', c.capFSh) +
      LN('M 101 70 C 100 50, 108 35, 120 33 C 132 35, 140 50, 139 70', OL, 1.8),
  );
  cap += `<rect x="110.5" y="44.5" width="19" height="14" rx="3.5" fill="${c.patch}" stroke="${OL}" stroke-width="1.8"/>` +
    LN('M 120 56.5 L 120 50.5', '#2b6b32', 1.6) +
    P('M 120 51.5 C 116 51.5, 113.6 49.6, 113.2 46.8 C 116.8 46.6, 119.2 48.4, 120 51.5 Z', '#3f8f46') +
    P(mirror('M 120 51.5 C 116 51.5, 113.6 49.6, 113.2 46.8 C 116.8 46.6, 119.2 48.4, 120 51.5 Z'), '#3f8f46');
  cap += CO(120, 32, 2.8, c.mesh, 2);
  const bill = 'M 86 70 C 100 62, 140 62, 154 70 C 158 76, 152 82, 146 82 C 132 77, 108 77, 94 82 C 88 82, 82 76, 86 70 Z';
  cap += ctx.region(bill, c.mesh, P('M 82 76 C 100 72, 140 72, 158 76 L 158 86 L 82 86 Z', c.meshSh) + LN('M 94 69 C 108 65, 132 65, 146 69', '#6fc076', 1.6));
  h += G(cap, rot(-7, 120, 66) + (A ? ' translate(0 -6)' : ''));

  const tilt = (A ? 0 : H ? 4 : 3) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 150));
  return s;
};

// ---- Biscuit: golden retriever, white short-sleeved shirt, skinny tie, name tag, bike helmet ----
const drawBiscuit: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#e8a84b', sh: '#c4852f', hi: '#f8d08a', mz: '#f2c27c',
    nose: '#1b151a', noseHi: '#5f5560', iris: '#5d3519', brow: '#b87326',
    shirt: '#f7f8fb', shirtSh: '#d3d9e5', tie: '#1f2a44', tieHi: '#36466e', tag: '#15131a',
    helm: '#4CC9F0', helmSh: '#2a96c2', helmHi: '#b5efff', vent: '#1d4d6b', stripe: '#FF9F1C',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  s += ctx.region('M 92 140 L 148 140 L 154 196 L 86 196 Z', c.f, E(120, 160, 36, 16, c.sh));
  const shirt = 'M 22 252 C 24 230, 32 212, 50 202 C 68 192, 86 186, 96 176 L 144 176 C 154 186, 172 192, 190 202 C 208 212, 216 230, 218 252 Z';
  s += ctx.region(
    shirt,
    c.shirt,
    P('M 176 196 C 198 206, 212 226, 218 252 L 174 252 C 182 232, 186 214, 176 196 Z', c.shirtSh) +
      // short sleeves: seam + hem, with golden forearms peeking out
      LN('M 56 204 C 50 220, 46 236, 46 252 M 184 204 C 190 220, 194 236, 194 252', c.shirtSh, 2.4) +
      P('M 22 252 C 22 244, 24 238, 28 234 L 40 252 Z', c.f) +
      P(mirror('M 22 252 C 22 244, 24 238, 28 234 L 40 252 Z'), c.sh) +
      LN('M 28 234 L 40 252 M 212 234 L 200 252', OL, 2.4) +
      // pocket
      LN('M 136 214 L 168 214 L 168 238 Q 152 242 136 238 Z', c.shirtSh, 2),
  );
  // collar
  const colL = 'M 96 174 L 118 192 L 106 206 L 86 184 Z';
  s += ctx.region(colL, c.shirt, P('M 86 184 L 106 206 L 100 204 L 84 188 Z', c.shirtSh), 2.6);
  s += ctx.region(mirror(colL), c.shirt, P(mirror('M 96 174 L 118 192 L 112 196 L 94 178 Z'), c.shirtSh), 2.6);
  // skinny tie
  s += ctx.region('M 115 190 L 125 190 L 127 248 L 120 256 L 113 248 Z', c.tie, P('M 120 190 L 125 190 L 127 248 L 120 256 Z', '#141c30') + LN('M 116 204 L 118 246', c.tieHi, 1.4), 2.4);
  s += PO('M 113 186 L 127 186 L 125 198 L 115 198 Z', c.tie, 2.4);
  // name tag
  s += `<rect x="134" y="210" width="38" height="13" rx="2.5" fill="${c.tag}" stroke="#3a3a48" stroke-width="1"/>`;
  s += `<text x="153" y="219.4" text-anchor="middle" font-family="Arial, Helvetica, sans-serif" font-size="7.4" font-weight="700" letter-spacing="0.5" fill="#ffffff">BISCUIT</text>`;

  let h = '';
  // helmet straps (tucked behind the ears)
  h += G(LN('M 76 84 C 79 100, 82 114, 85 128 M 164 84 C 161 100, 158 114, 155 128', OL, 4.2) + LN('M 76 84 C 79 100, 82 114, 85 128 M 164 84 C 161 100, 158 114, 155 128', '#3a3f55', 2), A ? 'translate(0 -5)' : '');
  // long feathered ears
  const ea = A ? 12 : H ? -3 : 0;
  const earL = 'M 88 64 C 70 62, 57 76, 54 98 C 52 116, 53 134, 57 148 C 60 154, 64 150, 67 156 C 71 160, 75 153, 79 156 C 83 138, 86 118, 90 98 C 92 84, 93 72, 88 64 Z';
  const feath = 'M 62 110 C 62 122, 64 132, 66 142 M 72 104 C 72 118, 74 130, 76 140';
  h += G(ctx.region(earL, c.f, P('M 56 98 C 54 116, 55 134, 59 148 L 48 150 L 46 96 Z', c.hi) + LN(feath, c.sh, 2.2) + P('M 90 98 C 86 118, 83 138, 79 156 L 92 156 L 96 96 Z', c.sh)), rot(ea, 86, 70));
  h += G(ctx.region(mirror(earL), c.f, LN(mirror(feath), c.sh, 2.2) + P(mirror('M 56 98 C 54 116, 55 134, 59 148 L 48 150 L 46 96 Z'), c.sh)), rot(-ea, 154, 70));

  const head = 'M 120 52 C 92 52, 74 68, 72 92 C 70 112, 78 126, 90 138 C 98 152, 106 164, 120 166 C 134 164, 142 152, 150 138 C 162 126, 170 112, 168 92 C 166 68, 148 52, 120 52 Z';
  const muzzle = 'M 98 116 C 92 130, 92 150, 102 160 C 110 168, 130 168, 138 160 C 148 150, 148 130, 142 116 C 134 110, 106 110, 98 116 Z';
  h += ctx.region(
    head,
    c.f,
    P('M 150 62 C 168 78, 170 110, 156 132 C 148 146, 140 158, 128 170 L 180 170 L 180 56 Z', c.sh) +
      E(100, 70, 16, 7, c.hi, rotA(-18, 100, 70)) +
      P(muzzle, c.mz) +
      P('M 138 118 C 146 132, 144 150, 134 162 L 150 160 L 148 118 Z', c.sh, ' opacity="0.6"') +
      E(106, 128, 5, 9, '#fde3b0', rotA(-10, 106, 128)),
  );

  const look = o.look ?? [0, A ? -0.25 : 0];
  const er = A ? 12.2 : H ? 11.4 : 11;
  const eb = { r: er, iris: c.iris, lidFill: c.f, look, lid: A || H ? 0 : 0.06, pupil: A ? 0.72 : 1, shine: H ? 1.3 : 1.15 };
  h += eye(ctx, { ...eb, x: 100, y: 102 });
  h += eye(ctx, { ...eb, x: 140, y: 102 });
  const by = A ? 80 : H ? 85 : 87;
  h += brow(100, by, 15, A ? -14 : -8, A ? 3 : 2, c.brow, 4.6);
  h += brow(140, by, 15, A ? 14 : 8, A ? 3 : 2, c.brow, 4.6);

  h += nose(120, 140, 25, 16, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smile', x: 120, y: 148, w: 14, h: 5, depth: 16, tongue: 13 });

  // bicycle helmet with chin straps
  let hel = '';
  const shell = 'M 70 84 C 66 52, 90 28, 120 28 C 150 28, 174 52, 170 84 C 154 74, 138 70, 120 70 C 102 70, 86 74, 70 84 Z';
  hel += ctx.region(
    shell,
    c.helm,
    P('M 148 34 C 166 46, 174 66, 170 84 L 154 80 C 158 62, 156 46, 148 34 Z', c.helmSh) +
      P('M 112 30 L 128 30 L 126 72 L 114 72 Z', c.stripe) +
      P('M 92 40 C 100 34, 108 31, 112 31 C 104 36, 98 42, 95 48 Z', c.helmHi) +
      PO('M 100 52 C 102 46, 105 43, 108 42 L 107.5 60 C 105 60, 102 61, 100 62 Z', c.vent, 1.2, c.vent) +
      PO(mirror('M 100 52 C 102 46, 105 43, 108 42 L 107.5 60 C 105 60, 102 61, 100 62 Z'), c.vent, 1.2, c.vent) +
      PO('M 82 66 C 84 60, 87 56, 90 54 L 91.5 65 C 89 66.5, 86 68, 83 70 Z', c.vent, 1.2, c.vent) +
      PO(mirror('M 82 66 C 84 60, 87 56, 90 54 L 91.5 65 C 89 66.5, 86 68, 83 70 Z'), c.vent, 1.2, c.vent),
  );
  hel += LN('M 72 82 C 88 72, 104 68, 120 68 C 136 68, 152 72, 168 82', c.helmSh, 2);
  h += G(hel, A ? 'translate(0 -5)' : '');

  const tilt = (A ? -2 : H ? 3 : 4) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 150));
  return s;
};

const DRAW: Record<LeaderId, DrawFn> = {
  donut: drawDonut,
  huckleberry: drawHuckleberry,
  griffin: drawGriffin,
  pan: drawPan,
  tank: drawTank,
  lucky: drawLucky,
  sorghum: drawSorghum,
  biscuit: drawBiscuit,
};

// ---------------------------------------------------------------------------------------------
// Framing
// ---------------------------------------------------------------------------------------------

/** Rounded navy card with a soft faction-colored glow behind the dog. */
function framed(ctx: Ctx, w: number, h: number, art: string, color: string): string {
  const glow = ctx.id('g');
  const navy = ctx.id('v');
  ctx.defs.push(
    `<radialGradient id="${glow}" gradientUnits="userSpaceOnUse" cx="${n(w / 2)}" cy="${n(h * 0.42)}" r="${n(h * 0.62)}"><stop offset="0" stop-color="${color}" stop-opacity="0.72"/><stop offset="0.45" stop-color="${color}" stop-opacity="0.26"/><stop offset="1" stop-color="${color}" stop-opacity="0"/></radialGradient>`,
    `<linearGradient id="${navy}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#11204a"/><stop offset="1" stop-color="#070f24"/></linearGradient>`,
  );
  const r = n(h * 0.15);
  const rect = `<rect x="0" y="0" width="${w}" height="${h}" rx="${r}" ry="${r}"`;
  const clip = ctx.clip(rect + '/>');
  return (
    rect + ` fill="url(#${navy})"/>` +
    rect + ` fill="url(#${glow})"/>` +
    `<g clip-path="url(#${clip})">${art}</g>` +
    `<rect x="1.5" y="1.5" width="${w - 3}" height="${h - 3}" rx="${n(h * 0.15 - 1.5)}" fill="none" stroke="${color}" stroke-opacity="0.45" stroke-width="2"/>`
  );
}

function svgDoc(ctx: Ctx, w: number, h: number, size: number, body: string, label: string): string {
  const defs = ctx.defs.length ? `<defs>${ctx.defs.join('')}</defs>` : '';
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${n((size * w) / h)}" height="${n(size)}" role="img" aria-label="${label}">${defs}${body}</svg>`;
}

// ---------------------------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------------------------

/** A single leader portrait (240x240 viewBox). */
export function dogSvg(id: LeaderId, mood: Mood = 'neutral', opts: PortraitOptions = {}): string {
  const ctx = new Ctx();
  const size = opts.size ?? 240;
  let art = DRAW[id](ctx, mood, {});
  if (opts.bg) art = framed(ctx, 240, 240, art, FACTION_COLORS[LEADER_FACTION[id]]);
  return svgDoc(ctx, 240, 240, size, art, LEADER_NAMES[id]);
}

/** Griffin and Pan together (320x240 viewBox): Pan, bigger, leans on his little brother. */
export function duoSvg(mood: Mood = 'neutral', opts: PortraitOptions = {}): string {
  const ctx = new Ctx();
  const size = opts.size ?? 240;
  const g = drawGriffin(ctx, mood, { look: mood === 'alarmed' ? [0.7, -0.5] : [0.85, -0.75], tilt: 4 });
  const p = drawPan(ctx, mood, { look: mood === 'alarmed' ? [-0.6, -0.1] : [-0.2, 0.25], tilt: -9 });
  let art = G(g, 'translate(-16 34) scale(0.84)') + G(p, 'translate(122 8) scale(0.94)');
  if (opts.bg) art = framed(ctx, 320, 240, art, FACTION_COLORS.FrontRange);
  return svgDoc(ctx, 320, 240, size, art, 'Griffin and Pan');
}

const FACTION_LEADER: Record<FactionKey, LeaderId | null> = {
  Pacific: 'donut',
  Cascadia: 'huckleberry',
  FrontRange: null, // two leaders: see duoSvg
  Texas: 'tank',
  Dreamland: 'lucky',
  GrainBelt: 'sorghum',
  Beehive: 'biscuit',
};

/** The leader portrait for a faction (Front Range -> the Griffin & Pan duo). */
export function leaderSvg(faction: FactionKey, mood: Mood = 'neutral', opts: PortraitOptions = {}): string {
  const id = FACTION_LEADER[faction];
  return id ? dogSvg(id, mood, opts) : duoSvg(mood, opts);
}

// ---------------------------------------------------------------------------------------------
// Emblems (64x64 viewBox, faction color on transparent)
// ---------------------------------------------------------------------------------------------

type EmblemFn = (ctx: Ctx, col: string, dark: string, light: string) => string;

const EMBLEMS: Record<FactionKey, EmblemFn> = {
  // smiling sun wearing a little beta badge
  Pacific: (_ctx, col, dark, light) => {
    let rays = '';
    for (let i = 0; i < 12; i++) {
      const a = (i / 12) * Math.PI * 2 - Math.PI / 2;
      const p = (r: number, da: number): string => `${n(30 + Math.cos(a + da) * r)} ${n(30 + Math.sin(a + da) * r)}`;
      rays += `M ${p(15, -0.2)} L ${p(i % 2 ? 25 : 28.5, 0)} L ${p(15, 0.2)} Z `;
    }
    return (
      PO(rays, col, 2.4) +
      CO(30, 30, 16, col, 2.6) +
      P('M 20 22 C 23 17, 29 15, 34 16 C 28 18, 24 21, 22 26 Z', light) +
      E(24.5, 28, 1.9, 2.6, OL) + E(35.5, 28, 1.9, 2.6, OL) +
      LN('M 23 34 Q 30 41 37 34', OL, 2.4) +
      E(21, 34, 2.6, 1.6, dark, ' opacity="0.45"') + E(39, 34, 2.6, 1.6, dark, ' opacity="0.45"') +
      CO(47, 47, 10, '#ffffff', 2.4) +
      LN('M 44 56 L 44 42.6 C 44 39.4, 49.6 39.4, 49.6 42.6 C 49.6 44.6, 47.6 45.4, 45.6 45.4 C 49 45.4, 50.6 46.8, 50.6 49 C 50.6 52.2, 46.4 52.6, 44 50.6', dark, 2.4)
    );
  },
  // fir tree under a rain cloud
  Cascadia: (_ctx, col, dark, light) => {
    const tree = 'M 32 20 L 42 32 L 37 32 L 46 43 L 40 43 L 51 56 L 13 56 L 24 43 L 18 43 L 27 32 L 22 32 Z';
    const cloud = 'M 15 22 C 9 22, 8 14, 14 13 C 14 7, 21 4, 25 8 C 28 2, 38 2, 40 9 C 46 7, 52 11, 50 17 C 55 18, 54 24, 49 24 L 17 24 C 16 24, 15 23, 15 22 Z';
    const drop = (x: number, y: number): string => PO(`M ${x} ${y - 4.5} C ${x + 3} ${y}, ${x + 3} ${y + 2.6}, ${x} ${y + 2.6} C ${x - 3} ${y + 2.6}, ${x - 3} ${y}, ${x} ${y - 4.5} Z`, light, 1.8);
    return (
      PO('M 29 55 L 35 55 L 35 61 L 29 61 Z', dark, 2.2) +
      PO(tree, col, 2.6) +
      P('M 32 21 L 32 55 L 50 55 L 40 43.5 L 45 43.5 L 36.5 32.5 L 41 32.5 Z', dark, ' opacity="0.45"') +
      PO(cloud, light, 2.6) +
      P('M 14 14 C 15 9, 20 7, 24 9 C 20 9, 17 11, 16 15 Z', '#ffffff', ' opacity="0.8"') +
      drop(12, 34) + drop(53, 32) + drop(9, 47) + drop(56, 46)
    );
  },
  // radar dish on a snowy peak
  FrontRange: (_ctx, col, dark, light) => {
    const mtn = 'M 2 60 L 20 36 L 26 42 L 36 28 L 62 60 Z';
    const snow = 'M 36 28 L 29.6 38.6 L 33 37.4 L 36 41 L 39 37.4 L 42.6 39.4 Z';
    return (
      PO(mtn, col, 2.6) +
      P('M 36 28 L 62 60 L 41 60 L 38 42 Z', dark, ' opacity="0.45"') +
      PO(snow, '#ffffff', 2) +
      PO('M 20 36 L 16 41.5 L 19 40.6 L 21.6 42.4 L 23.6 39.6 Z', '#ffffff', 1.6) +
      PO('M 33 29 L 39 29 L 37.5 21 L 34.5 21 Z', dark, 2) +
      G(
        PO('M -15 0 C -15 11, 15 11, 15 0 Z', light, 2.6) +
          P('M -11 1.6 C -10 7, 10 7, 11 1.6 Z', col, ' opacity="0.55"') +
          LN('M 0 4.5 L 0 -9', OL, 2.4) +
          CO(0, -10, 2.6, '#ffffff', 2),
        'translate(35 16) rotate(-28)',
      ) +
      LN('M 48 6.5 Q 53 8 54 13 M 51.5 2 Q 59 4.4 60 12', col, 2.8)
    );
  },
  // lone star with longhorns (cream horns with dark tips)
  Texas: (_ctx, col, dark, light) => {
    const horn = 'M 22 27 C 15 27, 9.5 26, 6.5 22.5 C 4.5 20, 3.4 17, 2.6 13.5 C 1.4 20, 2.4 26.5, 6.4 30.6 C 10.4 34.4, 16 35.6, 22 35.6 Z';
    const tip = 'M 2.6 13.5 C 1.4 20, 2.4 24, 4.4 26.6 L 6 22 C 4.5 20, 3.4 17, 2.6 13.5 Z';
    const bone = '#f4e6c8';
    return (
      PO(horn, bone, 2.4) +
      P(tip, '#3b2b22') +
      PO(mirror(horn, 32), bone, 2.4) +
      P(mirror(tip, 32), '#3b2b22') +
      P('M 22 32.4 C 14 32, 9 30, 6 26.4 C 8 31, 14 34.6, 22 35.6 Z', '#d2b98c') +
      P(mirror('M 22 32.4 C 14 32, 9 30, 6 26.4 C 8 31, 14 34.6, 22 35.6 Z', 32), '#d2b98c') +
      PO(starPath(32, 35, 19, 8), col, 2.8) +
      P('M 32 16 L 32 35 L 50.07 29.13 L 39.6 31.5 Z', dark, ' opacity="0.3"') +
      P('M 32 16 L 27.3 31 L 32 35 Z', light, ' opacity="0.6"')
    );
  },
  // flying saucer shaped like a poker chip
  Dreamland: (_ctx, col, dark, light) => {
    let edge = '';
    for (const [x, dy] of [[10, 1.5], [21, 3.6], [32, 4.4], [43, 3.6], [54, 1.5]] as const) edge += `M ${x - 2.2} ${44 + dy} L ${x + 2.2} ${44 + dy} `;
    return (
      P('M 24 50 L 40 50 L 48 62 L 16 62 Z', col, ' opacity="0.35"') +
      PO('M 4 37 L 4 42 C 4 54, 60 54, 60 42 L 60 37 Z', dark, 2.6) +
      LN(edge, '#ffffff', 3.2) +
      EO(32, 37, 28, 10, col, 2.6) +
      `<ellipse cx="32" cy="37" rx="21.5" ry="6.8" fill="none" stroke="#ffffff" stroke-width="3.2" stroke-dasharray="6.2 5.4"/>` +
      PO('M 19 35 C 19 16, 45 16, 45 35 C 45 39.5, 19 39.5, 19 35 Z', light, 2.4) +
      P('M 23.5 30 C 24 23.5, 28 20.5, 33 20 C 29 22.5, 27 26, 26.6 31 Z', '#ffffff', ' opacity="0.9"') +
      C(15, 46.5, 1.6, '#fff6a8') + C(32, 48.6, 1.6, '#fff6a8') + C(49, 46.5, 1.6, '#fff6a8')
    );
  },
  // wheat sheaf crossed with a propeller
  GrainBelt: (_ctx, col, dark, _light) => {
    const blade = 'M 0 -4 C -7 -10, -7 -24, -1 -30 C 1 -31, 3 -30, 4 -28 C 6 -20, 5 -9, 0 -4 Z';
    const prop =
      G(PO(blade, '#c9cfdc', 2.4) + P('M 0 -7 C -4 -12, -4 -22, -1 -27 Z', '#ffffff', ' opacity="0.6"'), 'translate(32 33) rotate(-50)') +
      G(PO(blade, '#c9cfdc', 2.4) + P('M 0 -7 C -4 -12, -4 -22, -1 -27 Z', '#ffffff', ' opacity="0.6"'), 'translate(32 33) rotate(130)');
    const head = (x: number, y: number, a: number): string =>
      G(PO('M 0 -12 C 5.4 -8, 5.4 7, 0 10 C -5.4 7, -5.4 -8, 0 -12 Z', col, 2.2) + LN('M -3 -4 L 0 -1.6 L 3 -4 M -3 2.6 L 0 5 L 3 2.6', dark, 1.4) + LN('M 0 -12 L 0 -17', col, 1.6), `translate(${x} ${y}) rotate(${a})`);
    return (
      prop +
      LN('M 32 61 L 32 27 M 30 61 L 21 27 M 34 61 L 43 27', OL, 5.4) +
      LN('M 32 61 L 32 27 M 30 61 L 21 27 M 34 61 L 43 27', col, 2.6) +
      head(19.5, 20, -16) + head(44.5, 20, 16) + head(32, 18, 0) +
      CO(32, 33, 4.6, '#c9cfdc', 2.2) +
      PO('M 24.5 43 L 39.5 43 L 39.5 49 L 24.5 49 Z', dark, 2.2)
    );
  },
  // beehive skep with little wings
  Beehive: (_ctx, col, dark, light) => {
    const wing = 'M 15 24 C 6 14, 0 20, 3 26 C 5 30, 11 30, 16 28 Z';
    const wing2 = 'M 14 30 C 6 30, 3 36, 7 39 C 10 41, 14 37, 16 33 Z';
    const hive = 'M 12 54 C 9 44, 12 26, 22 18 C 26 14, 38 14, 42 18 C 52 26, 55 44, 52 54 Z';
    return (
      PO(wing, '#ffffff', 2.2, OL, ' fill-opacity="0.92"') +
      PO(wing2, '#ffffff', 2.2, OL, ' fill-opacity="0.92"') +
      PO(mirror(wing, 32), '#ffffff', 2.2, OL, ' fill-opacity="0.92"') +
      PO(mirror(wing2, 32), '#ffffff', 2.2, OL, ' fill-opacity="0.92"') +
      PO(hive, col, 2.6) +
      P('M 38 16 C 50 24, 55 42, 52 54 L 42 54 C 46 42, 46 28, 38 16 Z', dark, ' opacity="0.35"') +
      LN('M 18 24 Q 32 28 46 24 M 13 33 Q 32 38 51 33 M 11 43 Q 32 48 53 43', dark, 2.4) +
      P('M 20 26 C 22 21, 26 18, 30 17 C 26 21, 24 24, 23 28 Z', light, ' opacity="0.8"') +
      PO('M 8 54 L 56 54 L 56 60 L 8 60 Z', dark, 2.4) +
      PO('M 26 54 C 26 46, 38 46, 38 54 Z', OL, 1.4) +
      CO(32, 11, 3, col, 2)
    );
  },
};

/** Faction emblem: a bold icon in the faction color on a transparent 64x64 canvas. */
export function emblemSvg(faction: FactionKey, opts: EmblemOptions = {}): string {
  const ctx = new Ctx();
  const size = opts.size ?? 64;
  const col = FACTION_COLORS[faction];
  const art = EMBLEMS[faction](ctx, col, mix(col, '#14101a', 0.42), mix(col, '#ffffff', 0.55));
  return svgDoc(ctx, 64, 64, size, art, `${faction} emblem`);
}
