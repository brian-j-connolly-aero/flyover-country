// Postcard West: one palette and one way of drawing, shared by the map, the
// show and the screens. Paper and ink by day; the same map dimmed to twilight
// at night, with faction inks that read on both.

import { Graphics } from 'pixi.js';
import type { Content, FactionId, SiteKind, UnitKind, P } from './types';

export const PAPER = 0xf3ead7;
export const PAPER_2 = 0xe7d9bb;
export const INK = 0x231e1a;
export const INK_2 = 0x51463c;
export const MUTED = 0x8b7e70;
export const OCEAN = 0xa9c2c9;
export const LAND = 0xeee2c6;
export const FRINGE = 0xe0d2b3;
/** Night is a multiply wash: cream turns twilight blue, colors stay apart. */
export const NIGHT = 0x4e5d94;
export const HIT_RED = 0xb8321e;
export const GOOD = 0x2e7d4f;

export const FACTION_HEX: Record<FactionId, string> = {
  Pacific: '#D9467A',
  Cascadia: '#1E8A6E',
  FrontRange: '#2E6DB4',
  Texas: '#C2471F',
  Dreamland: '#7A55C2',
  GrainBelt: '#7E9A2E',
  Beehive: '#E2A21A',
};

/** Faction inks come from factions.ron; this keeps the drawing code in step. */
export function setFactionColors(c: Content) {
  for (const f of c.factions) FACTION_HEX[f.id] = f.color;
}

export function hexNum(h: string): number {
  return parseInt(h.replace('#', ''), 16);
}

export function factionColor(f: FactionId | null | undefined): number {
  return f ? hexNum(FACTION_HEX[f]) : MUTED;
}

/** Mix two 0xRRGGBB colors: t = 0 gives a, 1 gives b. */
export function mix(a: number, b: number, t: number): number {
  const ch = (s: number) => {
    const x = (a >> s) & 255;
    const y = (b >> s) & 255;
    return Math.round(x + (y - x) * t) << s;
  };
  return ch(16) | ch(8) | ch(0);
}

export const SHAPES: Record<FactionId, 'circle' | 'triangle' | 'diamond' | 'star' | 'ring' | 'square' | 'hexagon'> = {
  Pacific: 'circle',
  Cascadia: 'triangle',
  FrontRange: 'diamond',
  Texas: 'star',
  Dreamland: 'ring',
  GrainBelt: 'square',
  Beehive: 'hexagon',
};

/** A faction's shape (for color-blind players), centered at (x, y). */
export function drawShape(g: Graphics, f: FactionId | null, x: number, y: number, r: number, color: number, alpha = 1) {
  const shape = f ? SHAPES[f] : 'circle';
  switch (shape) {
    case 'circle':
      g.circle(x, y, r).fill({ color, alpha });
      break;
    case 'ring':
      g.circle(x, y, r * 0.85).stroke({ width: r * 0.45, color, alpha });
      break;
    case 'triangle':
      g.poly([x, y - r * 1.1, x + r, y + r * 0.75, x - r, y + r * 0.75]).fill({ color, alpha });
      break;
    case 'diamond':
      g.poly([x, y - r * 1.15, x + r, y, x, y + r * 1.15, x - r, y]).fill({ color, alpha });
      break;
    case 'square':
      g.rect(x - r * 0.85, y - r * 0.85, r * 1.7, r * 1.7).fill({ color, alpha });
      break;
    case 'hexagon': {
      const pts: number[] = [];
      for (let i = 0; i < 6; i++) {
        const a = (Math.PI / 3) * i + Math.PI / 6;
        pts.push(x + Math.cos(a) * r, y + Math.sin(a) * r);
      }
      g.poly(pts).fill({ color, alpha });
      break;
    }
    case 'star':
      g.poly(star(x, y, r * 1.2, r * 0.5, 5)).fill({ color, alpha });
      break;
  }
}

function star(x: number, y: number, outer: number, inner: number, points: number): number[] {
  const pts: number[] = [];
  for (let i = 0; i < points * 2; i++) {
    const a = (Math.PI / points) * i - Math.PI / 2;
    const r = i % 2 === 0 ? outer : inner;
    pts.push(x + Math.cos(a) * r, y + Math.sin(a) * r);
  }
  return pts;
}

export const SITE_LABEL: Record<SiteKind, string> = {
  HQ: 'HQ',
  Factory: 'Factory',
  Airfield: 'Airfield',
  Jewel: 'Crown Jewel',
};

/**
 * A site pictogram on a paper token with an ink outline: a capitol star for
 * the HQ, a sawtooth roof for the Factory, a runway for the Airfield and a
 * gem for the Crown Jewel. `health` 0 draws it knocked flat.
 */
export function drawSite(g: Graphics, kind: SiteKind, x: number, y: number, r: number, color: number, health: number, dim = false) {
  const a = dim ? 0.4 : 1;
  const down = health <= 0;
  const ink = { width: Math.max(1, r * 0.16), color: INK, alpha: a };
  const fill = down ? PAPER_2 : color;
  // Token
  g.circle(x + r * 0.12, y + r * 0.14, r).fill({ color: INK, alpha: 0.35 * a });
  g.circle(x, y, r).fill({ color: PAPER, alpha: a }).stroke(ink);
  const s = r * 0.62;
  switch (kind) {
    case 'HQ':
      g.poly(star(x, y + s * 0.06, s * 1.05, s * 0.45, 5)).fill({ color: fill, alpha: a }).stroke({ ...ink, width: ink.width * 0.7 });
      break;
    case 'Factory': {
      const b = y + s * 0.7;
      const l = x - s;
      g.poly([l, b, l, y - s * 0.1, l + s * 0.5, y - s * 0.55, l + s * 0.5, y - s * 0.1, l + s, y - s * 0.55, l + s, y - s * 0.1, l + s * 1.5, y - s * 0.55, l + s * 1.5, y - s * 0.95, l + s * 2, y - s * 0.95, l + s * 2, b])
        .fill({ color: fill, alpha: a })
        .stroke({ ...ink, width: ink.width * 0.7 });
      break;
    }
    case 'Airfield': {
      const ang = -0.6;
      const c = Math.cos(ang);
      const sn = Math.sin(ang);
      const pt = (u: number, v: number) => [x + u * c - v * sn, y + u * sn + v * c];
      const w = s * 0.42;
      const L = s * 1.15;
      g.poly([...pt(-L, -w), ...pt(L, -w), ...pt(L, w), ...pt(-L, w)]).fill({ color: fill, alpha: a }).stroke({ ...ink, width: ink.width * 0.7 });
      for (let i = -2; i <= 2; i++) {
        const [x1, y1] = pt(i * s * 0.42 - s * 0.12, 0);
        const [x2, y2] = pt(i * s * 0.42 + s * 0.12, 0);
        g.moveTo(x1, y1).lineTo(x2, y2);
      }
      g.stroke({ width: Math.max(0.8, r * 0.1), color: PAPER, alpha: a });
      break;
    }
    case 'Jewel':
      g.poly([x, y - s * 1.05, x + s * 0.95, y - s * 0.2, x, y + s * 1.0, x - s * 0.95, y - s * 0.2]).fill({ color: fill, alpha: a }).stroke({ ...ink, width: ink.width * 0.7 });
      g.moveTo(x - s * 0.95, y - s * 0.2).lineTo(x + s * 0.95, y - s * 0.2).stroke({ width: ink.width * 0.5, color: INK, alpha: 0.6 * a });
      break;
  }
  if (down) {
    g.moveTo(x - r * 0.62, y - r * 0.62).lineTo(x + r * 0.62, y + r * 0.62).moveTo(x + r * 0.62, y - r * 0.62).lineTo(x - r * 0.62, y + r * 0.62).stroke({ width: r * 0.2, color: HIT_RED, alpha: 0.9 * a });
  }
}

/** Health pips in a row under a site: filled ink for health left, hollow for lost. */
export function drawPips(g: Graphics, x: number, y: number, health: number, max: number, size: number, onDark = false) {
  const gap = size * 1.45;
  const x0 = x - ((max - 1) * gap) / 2;
  const line = onDark ? PAPER : INK;
  for (let i = 0; i < max; i++) {
    const px = x0 + i * gap;
    if (i < health) g.rect(px - size / 2, y - size / 2, size, size).fill({ color: line });
    else g.rect(px - size / 2, y - size / 2, size, size).stroke({ width: Math.max(0.6, size * 0.2), color: line, alpha: 0.75 });
  }
}

/** A prize: a pennant on a pole, in its owner's ink (paper if nobody holds it). */
export function drawPrize(g: Graphics, x: number, y: number, r: number, owner: number | null, dim = false) {
  const a = dim ? 0.45 : 1;
  const ink = { width: Math.max(1, r * 0.15), color: INK, alpha: a };
  g.circle(x + r * 0.12, y + r * 0.14, r).fill({ color: INK, alpha: 0.35 * a });
  g.circle(x, y, r).fill({ color: PAPER, alpha: a }).stroke(ink);
  const px = x - r * 0.35;
  g.moveTo(px, y + r * 0.62).lineTo(px, y - r * 0.68).stroke({ ...ink, width: ink.width * 0.9 });
  g.poly([px, y - r * 0.68, px + r * 0.85, y - r * 0.4, px, y - r * 0.1]).fill({ color: owner ?? PAPER_2, alpha: a }).stroke({ ...ink, width: ink.width * 0.7 });
}

/** A plane silhouette pointing along `angle`: a dart for fighters, wide wings for bombers. */
export function drawPlane(g: Graphics, kind: UnitKind, x: number, y: number, size: number, angle: number, color: number, alpha = 1, outline = INK) {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  const pt = (u: number, v: number) => [x + (u * c - v * s) * size, y + (u * s + v * c) * size];
  const body =
    kind === 'Fighter'
      ? [pt(1.1, 0), pt(0.1, 0.2), pt(-0.2, 0.95), pt(-0.45, 0.95), pt(-0.35, 0.2), pt(-0.8, 0.22), pt(-0.95, 0.5), pt(-1.05, 0.5), pt(-0.95, 0), pt(-1.05, -0.5), pt(-0.95, -0.5), pt(-0.8, -0.22), pt(-0.35, -0.2), pt(-0.45, -0.95), pt(-0.2, -0.95), pt(0.1, -0.2)]
      : [pt(1.0, 0), pt(0.75, 0.16), pt(0.2, 0.16), pt(0.05, 1.15), pt(-0.25, 1.15), pt(-0.2, 0.16), pt(-0.75, 0.16), pt(-0.92, 0.5), pt(-1.05, 0.5), pt(-0.98, 0), pt(-1.05, -0.5), pt(-0.92, -0.5), pt(-0.75, -0.16), pt(-0.2, -0.16), pt(-0.25, -1.15), pt(0.05, -1.15), pt(0.2, -0.16), pt(0.75, -0.16)];
  g.poly(body.flat()).fill({ color, alpha }).stroke({ width: Math.max(0.8, size * 0.14), color: outline, alpha });
}

/** A dashed circle (patrol rings, last night's marks). */
export function dashedCircle(g: Graphics, x: number, y: number, r: number, dash: number, width: number, color: number, alpha: number) {
  const n = Math.max(8, Math.round((2 * Math.PI * r) / (dash * 2)));
  for (let i = 0; i < n; i++) {
    const a0 = (i / n) * Math.PI * 2;
    const a1 = a0 + (Math.PI / n);
    g.moveTo(x + Math.cos(a0) * r, y + Math.sin(a0) * r).arc(x, y, r, a0, a1);
  }
  g.stroke({ width, color, alpha });
}

/** A dashed straight line. */
export function dashedLine(g: Graphics, a: P, b: P, dash: number, width: number, color: number, alpha: number) {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len = Math.hypot(dx, dy);
  const n = Math.max(1, Math.floor(len / (dash * 2)));
  for (let i = 0; i < n; i++) {
    const t0 = (i * 2 * dash) / len;
    const t1 = Math.min(1, ((i * 2 + 1) * dash) / len);
    g.moveTo(a[0] + dx * t0, a[1] + dy * t0).lineTo(a[0] + dx * t1, a[1] + dy * t1);
  }
  g.stroke({ width, color, alpha });
}

/** A raid's path: a thick ink-edged ribbon with an arrowhead at the target. */
export function drawRaidPath(g: Graphics, from: P, to: P, color: number, width: number, alpha = 1, stopShort = 0) {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const len = Math.hypot(dx, dy) || 1;
  const ux = dx / len;
  const uy = dy / len;
  const end: P = [to[0] - ux * stopShort, to[1] - uy * stopShort];
  const head = width * 3.2;
  const base: P = [end[0] - ux * head, end[1] - uy * head];
  g.moveTo(from[0], from[1]).lineTo(base[0], base[1]).stroke({ width: width + 2, color: INK, alpha: alpha * 0.85, cap: 'round' });
  g.moveTo(from[0], from[1]).lineTo(base[0], base[1]).stroke({ width, color, alpha, cap: 'round' });
  const px = -uy * head * 0.62;
  const py = ux * head * 0.62;
  g.poly([end[0], end[1], base[0] + px, base[1] + py, base[0] - px, base[1] - py]).fill({ color, alpha }).stroke({ width: 1.2, color: INK, alpha: alpha * 0.85 });
}
