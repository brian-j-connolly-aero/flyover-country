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
const tr = (a: number): string => ` transform="${a}"`;
void tr;
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
  if (e.rim) s += E(e.x, e.y, rx + 2.6, ry + 2.6, e.rim);
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
    tan: '#cd7d3f', tanSh: '#a8602c', tanHi: '#e8a464',
    inner: '#b0706a', nose: '#a8645e', noseHi: '#dba69d', nostril: '#5a2a2c',
    iris: '#c27d2c', collar: '#2a2233', collarHi: '#4a3f5a',
    gold: '#f6c445', goldSh: '#cf931c', goldHi: '#fff3b0', gem: '#ff5fa2',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // shoulders + slim neck
  const body = 'M 98 140 C 96 176, 74 194, 48 210 C 36 218, 31 234, 29 252 L 211 252 C 209 234, 204 218, 192 210 C 166 194, 144 176, 142 140 Z';
  s += ctx.region(
    body,
    c.fur,
    E(120, 168, 38, 20, c.sh) +
      P('M 166 198 C 188 210, 203 226, 211 252 L 160 252 C 171 236, 175 216, 166 198 Z', c.sh) +
      P('M 70 204 C 56 214, 47 230, 45 252 L 57 252 C 59 234, 65 220, 77 208 Z', c.hi) +
      E(101, 230, 9, 7, c.tan, rotA(20, 101, 230)) +
      E(139, 230, 9, 7, c.tan, rotA(-20, 139, 230)),
  );
  // collar with a little gold ring
  s += PO('M 93 160 Q 120 174 147 160 L 149 174 Q 120 190 91 174 Z', c.collar, 2.6);
  s += LN('M 97 166 Q 120 178 143 166', c.collarHi, 1.6);
  s += CO(120, 189, 5.2, c.gold, 2.4) + C(118.5, 187.5, 1.6, c.goldHi);

  // ---- head group ----
  let h = '';
  const earL = 'M 100 54 C 84 46, 62 50, 50 66 C 42 80, 44 98, 56 114 C 62 104, 72 94, 84 86 C 88 74, 94 62, 100 54 Z';
  const earIn = 'M 95 56 C 80 52, 66 58, 58 68 C 68 66, 78 68, 88 73 Z';
  const earSh = 'M 52 66 C 44 82, 46 98, 56 114 L 40 118 L 36 60 Z';
  const ea = A ? 18 : H ? -5 : 0;
  h += G(ctx.region(earL, c.fur, P(earIn, c.inner) + P(earSh, c.sh)), rot(ea, 95, 58));
  h += G(ctx.region(mirror(earL), c.fur, P(mirror(earIn), c.inner) + P('M 186 64 C 196 80, 194 98, 184 114 L 204 118 L 204 60 Z', c.sh)), rot(-ea, 145, 58));

  const head = 'M 120 42 C 96 42, 80 56, 78 78 C 76 96, 80 108, 88 120 C 96 134, 100 152, 104 164 C 108 174, 132 174, 136 164 C 140 152, 144 134, 152 120 C 160 108, 164 96, 162 78 C 160 56, 144 42, 120 42 Z';
  h += ctx.region(
    head,
    c.fur,
    P('M 150 48 C 168 64, 170 98, 154 122 C 146 136, 142 154, 138 176 L 178 176 L 178 40 Z', c.sh) +
      E(103, 58, 15, 7, c.hi, rotA(-25, 103, 58)) +
      // tan cheeks + muzzle sides + chin
      P('M 70 108 C 84 111, 97 117, 106 124 L 134 124 C 143 117, 156 111, 170 108 L 170 190 L 70 190 Z', c.tan) +
      P('M 157 112 C 151 130, 145 150, 139 172 L 172 172 L 172 110 Z', c.tanSh) +
      E(96, 132, 5, 9, c.tanHi, rotA(-20, 96, 132)) +
      // chocolate bridge down to the nose
      P('M 101 98 C 107 118, 109 138, 109 154 L 131 154 C 131 138, 133 118, 139 98 Z', c.fur) +
      P('M 131 104 C 130 122, 130 140, 131 154 L 139 154 L 139 100 Z', c.sh) +
      P('M 112 104 C 114 120, 115 134, 115 146 L 119 146 C 119 134, 119 120, 118 104 Z', c.hi),
  );
  // tan eyebrow dots
  const byL = A ? 68 : H ? 76 : 79;
  const byR = A ? 68 : H ? 76 : 74;
  const bA = A ? 22 : H ? 12 : 8;
  h += E(100, byL, 7.5, 5.2, c.tan, rotA(-bA, 100, byL)) + E(98.5, byL - 1.5, 3.4, 2, c.tanHi, rotA(-bA, 100, byL));
  h += E(140, byR, 7.5, 5.2, c.tan, rotA(bA + (A || H ? 0 : 10), 140, byR)) + E(138.5, byR - 1.5, 3.4, 2, c.tanHi, rotA(bA, 140, byR));

  const er = A ? 11.6 : 10.4;
  const lid = A ? 0 : H ? 0 : 0.34;
  const look = o.look ?? (A ? [0, -0.2] : [0, 0]);
  h += eye(ctx, { x: 100, y: 95, r: er, iris: c.iris, lid, lidTilt: -1.5, lidFill: c.fur, low: H ? 0.3 : 0, lowFill: c.fur, look, pupil: A ? 0.72 : 1, shine: H ? 1.15 : 1 });
  h += eye(ctx, { x: 140, y: 95, r: er, iris: c.iris, lid: lid * 0.85, lidTilt: 1.5, lidFill: c.fur, low: H ? 0.3 : 0, lowFill: c.fur, look, pupil: A ? 0.72 : 1, shine: H ? 1.15 : 1 });

  h += nose(120, 155, 27, 17, c.nose, c.noseHi, c.nostril);
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smirk', x: 120, y: 163.5, w: 12, h: 4, depth: 15, tongue: 12 });

  // tiara (a little crooked, as is her right)
  const tiara = 'M 96 52 L 96 44 L 102 32 L 108 41 L 114 36 L 120 21 L 126 36 L 132 41 L 138 32 L 144 44 L 144 52 Q 120 44 96 52 Z';
  let t = ctx.region(tiara, c.gold, P('M 94 54 Q 120 46 146 54 L 146 47 Q 120 39 94 47 Z', c.goldSh) + P('M 120 21 L 114 36 L 118 38 Z', c.goldHi) + P('M 102 32 L 99 40 L 102 41 Z', c.goldHi), 2.6);
  t += CO(102, 32, 2.6, c.gold, 2) + CO(138, 32, 2.6, c.gold, 2) + CO(120, 20, 3, c.gold, 2);
  t += PO('M 120 33 L 125 39.5 L 120 46 L 115 39.5 Z', c.gem, 2.2) + P('M 120 34.5 L 117 39.5 L 120 39.5 Z', '#ffd3e6');
  h += G(t, rot(-9 + (A ? -4 : 0), 120, 50) + (A ? ' translate(0 -5)' : ''));

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
    iris: '#a8642c', rim: '#4b4958', brow: '#77748a',
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
      speckles(11, 70, 120, 222, 86, 36, 0.8, 2.3, c.tick) +
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
      speckles(7, 95, 120, 140, 30, 22, 0.7, 2, c.tick) +
      speckles(8, 30, 84, 124, 12, 10, 0.7, 1.9, c.tick) +
      speckles(9, 30, 156, 124, 12, 10, 0.7, 1.9, c.tick) +
      speckles(10, 10, 120, 74, 5, 16, 0.6, 1.3, c.tickLt) +
      // black mask patches with a white blaze between
      P(patchL, c.b) +
      P(mirror(patchL), c.b) +
      P('M 80 62 C 88 56, 98 56, 106 60 C 98 60, 90 62, 84 68 Z', c.bHi) +
      P(mirror('M 80 62 C 88 56, 98 56, 106 60 C 98 60, 90 62, 84 68 Z'), c.bHi) +
      speckles(12, 14, 74, 118, 10, 6, 0.7, 1.7, c.b) +
      speckles(13, 14, 166, 118, 10, 6, 0.7, 1.7, c.b),
  );

  const look = o.look ?? (A ? [0, -0.25] : [0.1, -0.1]);
  const er = A ? 12 : 10.8;
  const eyeBase = { r: er, iris: c.iris, rim: c.rim, lidFill: c.b, look, pupil: A ? 0.7 : 1, low: H ? 0.3 : 0, lowFill: c.b, shine: H ? 1.15 : 1 };
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
    f: '#26252e', sh: '#141319', hi: '#3c4259', gl: '#6474a3', mz: '#302f3a',
    grey: '#9a98a8', nose: '#0e0d12', noseHi: '#59607a', iris: '#8a5224', rim: '#4d4c5a', brow: '#5d6384',
    hat: '#3fb4e6', hatSh: '#2584b4', hatHi: '#94e0ff', stripe: '#f3f7fb', pom: '#f5f0e4', pomSh: '#d6cdb9',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  const body = 'M 86 148 C 80 180, 54 196, 28 212 C 16 222, 12 236, 10 252 L 230 252 C 228 236, 224 222, 212 212 C 186 196, 160 180, 154 148 Z';
  s += ctx.region(
    body,
    c.f,
    E(120, 174, 50, 22, c.sh) +
      P('M 36 212 C 50 200, 68 194, 86 196 C 70 202, 54 212, 44 226 Z', c.hi) +
      P('M 196 206 C 214 216, 226 232, 230 252 L 190 252 C 200 236, 202 220, 196 206 Z', c.sh) +
      speckles(21, 34, 120, 220, 18, 15, 0.7, 1.6, c.grey, 0.85),
  );

  let h = '';
  // soft floppy ears
  const ea = A ? 10 : H ? -4 : 0;
  const earL = 'M 102 60 C 86 48, 63 50, 51 62 C 41 74, 40 96, 46 118 C 58 112, 71 99, 86 84 Z';
  h += G(ctx.region(earL, c.f, P('M 55 64 C 48 78, 47 96, 50 110 C 45 96, 45 78, 55 64 Z', c.gl) + P('M 86 84 C 74 96, 62 108, 46 118 L 60 124 L 92 90 Z', c.sh) + LN('M 62 60 C 70 66, 78 74, 84 82', c.sh, 3)), rot(ea, 92, 66));
  h += G(ctx.region(mirror(earL), c.f, P(mirror('M 55 64 C 48 78, 47 96, 50 110 C 45 96, 45 78, 55 64 Z'), c.hi) + P(mirror('M 86 84 C 74 96, 62 108, 46 118 L 60 124 L 92 90 Z'), c.sh) + LN(mirror('M 62 60 C 70 66, 78 74, 84 82'), c.sh, 3)), rot(-ea, 148, 66));

  const head = 'M 120 48 C 86 48, 64 68, 62 98 C 60 122, 70 140, 88 152 C 98 166, 106 176, 120 176 C 134 176, 142 166, 152 152 C 170 140, 180 122, 178 98 C 176 68, 154 48, 120 48 Z';
  const muzzle = 'M 97 116 C 92 132, 92 152, 100 164 C 108 175, 132 175, 140 164 C 148 152, 148 132, 143 116 C 135 110, 105 110, 97 116 Z';
  h += ctx.region(
    head,
    c.f,
    P('M 156 58 C 178 76, 182 112, 166 136 C 156 150, 146 164, 136 180 L 190 180 L 190 50 Z', c.sh) +
      E(100, 66, 20, 8, c.hi, rotA(-18, 100, 66)) +
      E(92, 64, 9, 3.4, c.gl, rotA(-22, 92, 64)) +
      P(muzzle, c.mz) +
      E(108, 128, 5, 10, c.hi, rotA(-8, 108, 128)) +
      speckles(22, 46, 120, 170, 20, 7, 0.6, 1.5, c.grey) +
      speckles(23, 10, 104, 160, 6, 6, 0.5, 1.1, c.grey, 0.8) +
      speckles(24, 10, 136, 160, 6, 6, 0.5, 1.1, c.grey, 0.8),
  );

  const look = o.look ?? (A ? [0, -0.2] : [0, 0.15]);
  const lid = A ? 0.12 : H ? 0.36 : 0.5;
  const er = A ? 11 : 10.2;
  const eb = { r: er, iris: c.iris, rim: c.rim, lid, lidFill: c.f, look, low: H ? 0.26 : 0, lowFill: c.f, pupil: A ? 0.8 : 1 };
  h += eye(ctx, { ...eb, x: 98, y: 104, lidTilt: 2 });
  h += eye(ctx, { ...eb, x: 142, y: 104, lidTilt: -2 });

  const by = A ? 82 : H ? 88 : 90;
  h += brow(98, by, 16, A ? -10 : -4, A ? 3 : 1, c.brow, 4.5);
  h += brow(142, by, 16, A ? 10 : 4, A ? 3 : 1, c.brow, 4.5);

  h += nose(120, 147, 30, 19, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smile', x: 120, y: 156.5, w: 15, h: 5, depth: 16, tongue: 14, tongueX: H ? 4 : 0, ink: '#08070b' });

  // knit beanie
  let b = '';
  const crown = 'M 75 72 C 72 40, 95 22, 120 22 C 145 22, 168 40, 165 72 Z';
  b += ctx.region(crown, c.hat, P('M 144 26 C 160 36, 168 52, 166 72 L 150 72 C 154 56, 152 40, 144 26 Z', c.hatSh) + P('M 75 50 C 100 44, 140 44, 166 50 L 166 58 C 140 52, 100 52, 75 58 Z', c.stripe) + P('M 92 34 C 100 28, 110 26, 118 26 C 108 30, 100 34, 94 40 Z', c.hatHi));
  const cuff = 'M 70 70 C 92 62, 148 62, 170 70 L 168 86 C 146 77, 94 77, 72 86 Z';
  let ribs = '';
  for (let x = 78; x <= 162; x += 7) ribs += `M ${x} ${70 - Math.sin(((x - 70) / 100) * Math.PI) * 6} L ${x} ${85 - Math.sin(((x - 70) / 100) * Math.PI) * 8}`;
  b += ctx.region(cuff, c.hat, LN(ribs, c.hatSh, 2.2) + P('M 150 66 C 160 67, 166 69, 170 70 L 168 86 C 162 82, 156 80, 150 79 Z', c.hatSh, ' opacity="0.7"'));
  b += ctx.region(bumpy(120, 18, 9, 1.4, 11), c.pom, C(124, 22, 5, c.pomSh) + C(116, 14, 3.4, '#ffffff'), 2.6);
  h += G(b, rot(7, 120, 70) + (A ? ' translate(0 -4)' : ''));

  const tilt = (A ? -2 : H ? 3 : 4) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 160));
  return s;
};

// ---- Ranger Huckleberry: shaggy black-and-brown Newfie, campaign hat, yellow slicker, rain ----
const drawHuckleberry: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#2a2120', sh: '#171110', hi: '#4b3c37', br: '#7d4c2d', brSh: '#5b3519', brHi: '#a26a42',
    nose: '#110d0e', noseHi: '#5b4c4a', iris: '#6f411f', rim: '#4f403c',
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
  const lid = A ? 0.06 : H ? 0.32 : 0.56;
  const look = o.look ?? [0, A ? -0.2 : 0];
  const eb = { r: A ? 10.8 : 9.8, iris: c.iris, rim: c.rim, lid, lidFill: c.f, look, low: H ? 0.24 : 0, lowFill: c.f, pupil: A ? 0.75 : 1 };
  h += eye(ctx, { ...eb, x: 98, y: 102, lidTilt: 1.5 });
  h += eye(ctx, { ...eb, x: 142, y: 102, lidTilt: -1.5 });
  if (!H) h += LN('M 90 116 Q 98 120 106 116 M 134 116 Q 142 120 150 116', '#5e4b46', 2);
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
  const crown = 'M 82 64 C 82 46, 92 32, 106 20 C 112 24, 116 26, 120 21 C 124 26, 128 24, 134 20 C 148 32, 158 46, 158 64 Z';
  hat += ctx.region(
    crown,
    c.hat,
    P('M 140 26 C 152 38, 158 50, 158 64 L 144 64 C 144 50, 142 38, 134 22 Z', c.hatSh) +
      LN('M 106 22 C 104 34, 104 44, 106 54 M 134 22 C 136 34, 136 44, 134 54', c.hatSh, 2.2) +
      P('M 112 28 C 116 30, 118 32, 120 30 L 120 50 C 116 46, 113 40, 112 28 Z', c.hatHi, ' opacity="0.8"') +
      P('M 80 54 C 100 58, 140 58, 160 54 L 160 66 L 80 66 Z', c.band),
  );
  hat += PO('M 120 52 L 126 62 L 122 62 L 122 64.5 L 118 64.5 L 118 62 L 114 62 Z', c.pine, 1.6);
  // rain on the brim
  const drop = (x: number, y: number, k: number): string =>
    PO(`M ${x} ${y - 7 * k} C ${x + 4.5 * k} ${y - 1 * k}, ${x + 4.5 * k} ${y + 4 * k}, ${x} ${y + 4 * k} C ${x - 4.5 * k} ${y + 4 * k}, ${x - 4.5 * k} ${y - 1 * k}, ${x} ${y - 7 * k} Z`, c.drop, 1.8) +
    E(x - 1.4 * k, y + 0.6 * k, 1.1 * k, 1.6 * k, c.dropHi);
  hat += drop(54, 86, 1) + drop(196, 90, 0.9) + E(96, 59, 2.6, 1.4, c.dropHi, ' opacity="0.9"') + E(150, 60, 2, 1.1, c.dropHi, ' opacity="0.9"');
  h += G(hat, A ? 'translate(0 -6)' : '');

  const tilt = (A ? 2 : H ? -2 : 0) + (o.tilt ?? 0);
  s += G(h, rot(tilt, 120, 160));
  // a few raindrops in the air
  s += drop(26, 36, 1.1) + drop(214, 30, 1) + drop(206, 140, 0.8) + drop(30, 168, 0.8);
  return s;
};

// ---- Coach Tank: Blue Lacy, slate coat, amber eyes, folded ears, ball cap, whistle on lanyard ----
const drawTank: DrawFn = (ctx, mood, o) => {
  const c = {
    f: '#707e8f', sh: '#566272', hi: '#97a6b8', mz: '#7f8c9c',
    nose: '#3c434e', noseHi: '#7d8794', iris: '#f0a531', rim: '#4d5664', brow: '#46505e',
    cap: '#e8403f', capSh: '#b72d2d', capHi: '#ff7d73',
    polo: '#25314f', poloSh: '#18213a', poloHi: '#36466f', lan: '#ff4d4d',
    wh: '#d2d9e2', whSh: '#9aa4b0', whHi: '#ffffff',
  };
  const A = mood === 'alarmed';
  const H = mood === 'happy';
  let s = '';

  // neck fur, then the coach polo
  s += ctx.region('M 96 140 L 144 140 L 150 200 L 90 200 Z', c.f, E(120, 160, 30, 16, c.sh) + P('M 108 176 L 132 176 L 120 196 Z', '#e9edf2'));
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
  // folded (rose) ears
  const ea = A ? 14 : H ? -4 : 0;
  const earL = 'M 102 66 C 90 54, 74 50, 62 52 C 54 58, 50 72, 52 86 C 62 80, 76 78, 90 80 Z';
  h += G(ctx.region(earL, c.f, P('M 62 52 C 72 58, 80 66, 88 76 L 90 80 C 76 76, 62 72, 52 62 Z', c.sh) + LN('M 62 53 C 72 60, 80 68, 88 78', OL, 1.8) + P('M 56 62 C 54 70, 53 78, 53 84 L 48 84 L 52 60 Z', c.hi)), rot(ea, 92, 72));
  h += G(ctx.region(mirror(earL), c.f, P(mirror('M 62 52 C 72 58, 80 66, 88 76 L 90 80 C 76 76, 62 72, 52 62 Z'), c.sh) + LN(mirror('M 62 53 C 72 60, 80 68, 88 78'), OL, 1.8)), rot(-ea, 148, 72));

  const head = 'M 120 54 C 94 54, 77 70, 76 92 C 75 108, 80 120, 90 132 C 97 147, 104 160, 120 165 C 136 160, 143 147, 150 132 C 160 120, 165 108, 164 92 C 163 70, 146 54, 120 54 Z';
  h += ctx.region(
    head,
    c.f,
    P('M 150 64 C 166 80, 168 110, 152 132 C 144 146, 138 158, 128 170 L 176 170 L 176 60 Z', c.sh) +
      E(120, 86, 46, 9, c.sh, ' opacity="0.8"') +
      P('M 99 116 C 94 128, 96 146, 104 156 C 110 164, 130 164, 136 156 C 144 146, 146 128, 141 116 C 134 110, 106 110, 99 116 Z', c.mz) +
      E(106, 128, 5, 10, c.hi, rotA(-12, 106, 128)) +
      LN('M 114 96 C 116 104, 118 110, 120 116 M 126 96 C 124 104, 122 110, 120 116', c.sh, 2),
  );

  const look = o.look ?? [0, A ? -0.25 : 0];
  const er = A ? 11.6 : 10.4;
  const lid = A ? 0 : H ? 0 : 0.26;
  const eb = { r: er, iris: c.iris, rim: c.rim, lid, lidFill: c.f, look, low: H ? 0.3 : 0, lowFill: c.f, pupil: A ? 0.62 : 0.85, shine: 1.05 };
  h += eye(ctx, { ...eb, x: 100, y: 102, lidTilt: -3.5 });
  h += eye(ctx, { ...eb, x: 140, y: 102, lidTilt: 3.5 });
  const by = A ? 80 : H ? 86 : 90;
  const ba = A ? -14 : H ? -6 : 14;
  h += brow(100, by, 17, ba, A ? 3 : 1, c.brow, 5.2);
  h += brow(140, by, 17, -ba, A ? 3 : 1, c.brow, 5.2);

  h += nose(120, 144, 25, 16, c.nose, c.noseHi, '#191c22');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'grin', x: 120, y: 152, w: 14, h: 4, depth: 18, tongue: 10 });

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
    m: '#2c2329', mHi: '#4c3e49', nose: '#120e12', noseHi: '#5d4f5a', iris: '#4a2a17',
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
  const earL = 'M 86 62 C 72 54, 56 58, 50 72 C 48 82, 52 90, 60 94 C 64 84, 72 76, 88 72 Z';
  h += G(ctx.region(earL, c.m, P('M 52 70 C 50 80, 54 88, 60 92 L 50 96 L 46 68 Z', c.mHi)), rot(ea, 82, 68));
  h += G(ctx.region(mirror(earL), c.m, P(mirror('M 88 72 C 72 76, 64 84, 60 94 L 70 96 L 92 76 Z'), '#1a1418')), rot(-ea, 158, 68));

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
  const eb = { r: er, ratio: 1, iris: c.iris, lidFill: c.m, look, low: H ? 0.24 : 0, lowFill: c.m, pupil: A ? 0.68 : 0.95, shine: 1.15 };
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
  const er = A ? 11.2 : 10;
  const eb = { r: er, iris: c.iris, lidFill: c.f, look, low: H ? 0.3 : A ? 0 : 0.14, lowFill: c.f, pupil: A ? 0.72 : 1, shine: H ? 1.15 : 1 };
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
  cap += EO(120, 52, 9, 7, c.patch, 1.8) + P('M 120 58 C 116 54, 116 49, 120 46 C 124 49, 124 54, 120 58 Z', '#3f8f46') + LN('M 120 58 L 120 47', '#2b6b32', 1);
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
    helm: '#4CC9F0', helmSh: '#2a96c2', helmHi: '#b5efff', vent: '#163a52', stripe: '#FF9F1C',
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
  const er = A ? 11.2 : 10.2;
  const eb = { r: er, iris: c.iris, lidFill: c.f, look, lid: A ? 0 : H ? 0 : 0.12, low: A ? 0 : H ? 0.3 : 0.18, lowFill: c.f, pupil: A ? 0.72 : 1, shine: 1.1 };
  h += eye(ctx, { ...eb, x: 100, y: 102, lidTilt: 1 });
  h += eye(ctx, { ...eb, x: 140, y: 102, lidTilt: -1 });
  const by = A ? 80 : H ? 85 : 87;
  h += brow(100, by, 15, A ? -14 : -8, A ? 3 : 2, c.brow, 4.6);
  h += brow(140, by, 15, A ? 14 : 8, A ? 3 : 2, c.brow, 4.6);

  h += nose(120, 140, 25, 16, c.nose, c.noseHi, '#000000');
  h += mouth({ kind: H ? 'open' : A ? 'o' : 'smile', x: 120, y: 148, w: 14, h: 5, depth: 16, tongue: 13 });

  // bicycle helmet with chin straps
  let hel = '';
  hel += LN('M 78 82 C 82 100, 86 114, 92 126 M 162 82 C 158 100, 154 114, 148 126', OL, 4.2) + LN('M 78 82 C 82 100, 86 114, 92 126 M 162 82 C 158 100, 154 114, 148 126', '#3a3f55', 2);
  const shell = 'M 70 84 C 66 52, 90 28, 120 28 C 150 28, 174 52, 170 84 C 154 74, 138 70, 120 70 C 102 70, 86 74, 70 84 Z';
  hel += ctx.region(
    shell,
    c.helm,
    P('M 148 34 C 166 46, 174 66, 170 84 L 154 80 C 158 62, 156 46, 148 34 Z', c.helmSh) +
      P('M 112 30 L 128 30 L 126 72 L 114 72 Z', c.stripe) +
      P('M 92 40 C 100 34, 108 31, 112 31 C 104 36, 98 42, 95 48 Z', c.helmHi) +
      PO('M 98 50 C 101 44, 106 41, 109 41 L 108 58 C 104 58, 100 58, 97 60 Z', c.vent, 1.2, c.vent) +
      PO(mirror('M 98 50 C 101 44, 106 41, 109 41 L 108 58 C 104 58, 100 58, 97 60 Z'), c.vent, 1.2, c.vent) +
      PO('M 80 64 C 82 58, 86 54, 90 52 L 92 64 C 88 66, 84 68, 81 70 Z', c.vent, 1.2, c.vent) +
      PO(mirror('M 80 64 C 82 58, 86 54, 90 52 L 92 64 C 88 66, 84 68, 81 70 Z'), c.vent, 1.2, c.vent),
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
  const g = drawGriffin(ctx, mood, { look: mood === 'alarmed' ? [0.7, -0.5] : [0.85, -0.75], tilt: 7 });
  const p = drawPan(ctx, mood, { look: mood === 'alarmed' ? [-0.6, -0.1] : [-0.2, 0.25], tilt: -9 });
  let art = G(g, 'translate(-4 34) scale(0.84)') + G(p, 'translate(106 8) scale(0.94)');
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
    const mtn = 'M 3 58 L 22 32 L 28 38 L 36 24 L 61 58 Z';
    const snow = 'M 36 24 L 29.5 35.5 L 33 34 L 36 38 L 39 34 L 43 36 Z';
    return (
      PO(mtn, col, 2.6) +
      P('M 36 24 L 61 58 L 40 58 L 38 40 Z', dark, ' opacity="0.45"') +
      PO(snow, '#ffffff', 2) +
      PO('M 22 32 L 18 37.5 L 21 36.5 L 23.5 38.5 L 25.5 35.5 Z', '#ffffff', 1.6) +
      LN('M 36 24 L 36 17', OL, 3.4) +
      PO('M 30 26 L 42 26 L 38 22 L 34 22 Z', dark, 2) +
      G(PO('M -12 0 A 12 7 0 0 0 12 0 Z', light, 2.4) + LN('M 0 3.4 L 0 -8', OL, 2) + CO(0, -9, 2, col, 1.6), 'translate(36 12) rotate(-30) translate(0 -2)') +
      LN('M 47 4 Q 52 6 53 11 M 50 0 Q 57 3 58 10', col, 2.4)
    );
  },
  // lone star with longhorns
  Texas: (_ctx, col, dark, light) => {
    const horn = 'M 22 27 C 14 27, 7 25, 2 14 C 4 26, 11 34, 22 36 Z';
    return (
      PO(horn, light, 2.4) +
      PO(mirror(horn, 32), light, 2.4) +
      P('M 22 33 C 12 32, 6 27, 3 18 C 6 30, 12 35, 22 36 Z', dark, ' opacity="0.35"') +
      P(mirror('M 22 33 C 12 32, 6 27, 3 18 C 6 30, 12 35, 22 36 Z', 32), dark, ' opacity="0.35"') +
      PO(starPath(32, 34, 19, 8), col, 2.8) +
      P(`M 32 15 L 32 34 L 50.07 28.13 L 39.62 30.48 Z`, dark, ' opacity="0.3"') +
      P(`M 32 15 L 27.3 30 L 32 34 Z`, light, ' opacity="0.6"')
    );
  },
  // flying saucer shaped like a poker chip
  Dreamland: (_ctx, col, dark, light) => {
    let edge = '';
    for (const x of [12, 22, 32, 42, 52]) edge += `M ${x - 2} ${44 + (x === 32 ? 3.6 : x === 22 || x === 42 ? 2.6 : 0.4)} L ${x + 2} ${44 + (x === 32 ? 3.6 : x === 22 || x === 42 ? 2.6 : 0.4)}`;
    return (
      P('M 22 50 L 42 50 L 52 63 L 12 63 Z', light, ' opacity="0.28"') +
      PO('M 20 32 C 20 16, 44 16, 44 32 Z', light, 2.4) +
      P('M 25 26 C 26 21, 30 19, 34 19 C 30 21, 28 24, 27.6 28 Z', '#ffffff', ' opacity="0.85"') +
      PO('M 4 36 L 4 41 C 4 53, 60 53, 60 41 L 60 36 Z', dark, 2.6) +
      LN(edge, '#ffffff', 3.2) +
      EO(32, 36, 28, 10, col, 2.6) +
      `<ellipse cx="32" cy="36" rx="21" ry="6.6" fill="none" stroke="#ffffff" stroke-width="3.2" stroke-dasharray="6.4 5.6"/>` +
      EO(32, 36, 13.5, 4.2, col, 1.8)
    );
  },
  // wheat sheaf crossed with a propeller
  GrainBelt: (_ctx, col, dark, light) => {
    const blade = 'M 0 -3 C -5 -10, -5 -22, 0 -29 C 5 -22, 5 -10, 0 -3 Z';
    const prop =
      G(PO(blade, dark, 2.4) + P('M 0 -6 C -3 -12, -3 -21, 0 -26 Z', light, ' opacity="0.5"'), 'translate(32 32) rotate(-45)') +
      G(PO(blade, dark, 2.4), 'translate(32 32) rotate(135)');
    const head = (x: number, y: number, a: number): string =>
      G(PO('M 0 -11 C 4.6 -7, 4.6 6, 0 9 C -4.6 6, -4.6 -7, 0 -11 Z', col, 2.2) + LN('M -2.6 -3 L 0 -1 L 2.6 -3 M -2.6 3 L 0 5 L 2.6 3', dark, 1.3), `translate(${x} ${y}) rotate(${a})`);
    return (
      prop +
      LN('M 32 60 L 32 24 M 30 60 L 21 22 M 34 60 L 43 22', OL, 5.2) +
      LN('M 32 60 L 32 24 M 30 60 L 21 22 M 34 60 L 43 22', col, 2.4) +
      head(20.5, 16, -14) + head(43.5, 16, 14) + head(32, 14, 0) +
      CO(32, 32, 4.5, light, 2.2) +
      PO('M 25 42 L 39 42 L 39 48 L 25 48 Z', dark, 2.2)
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
