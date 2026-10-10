// Colors, shapes and small drawing helpers shared by the map and the show.

import { Graphics } from 'pixi.js';
import type { Content, FactionId, NodeKind, UnitKind } from './types';

export const NAVY = 0x071022;
export const NEUTRAL = 0x8a9bb8;
export const STATIC_EDGE = 0xb9c6dd;

export const FACTION_HEX: Record<FactionId, string> = {
  Pacific: '#FF5FA2',
  Cascadia: '#3DDC84',
  FrontRange: '#4CC9F0',
  Texas: '#FF4D4D',
  Dreamland: '#B388FF',
  GrainBelt: '#F2E86D',
  Beehive: '#FF9F1C',
};

export function hexNum(h: string): number {
  return parseInt(h.replace('#', ''), 16);
}

export function factionColor(f: FactionId | null | undefined): number {
  return f ? hexNum(FACTION_HEX[f]) : NEUTRAL;
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

/** Draw a faction's colorblind-safe shape centered at (x, y). */
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
    case 'star': {
      const pts: number[] = [];
      for (let i = 0; i < 10; i++) {
        const a = (Math.PI / 5) * i - Math.PI / 2;
        const rr = i % 2 === 0 ? r * 1.2 : r * 0.5;
        pts.push(x + Math.cos(a) * rr, y + Math.sin(a) * rr);
      }
      g.poly(pts).fill({ color, alpha });
      break;
    }
  }
}

/** A unit glyph: a triangle pointing along `angle` with the faction shape inside. */
export function drawGlyph(g: Graphics, f: FactionId | null, kind: UnitKind, x: number, y: number, size: number, angle = -Math.PI / 2, alpha = 1) {
  const color = factionColor(f);
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  const pt = (fx: number, fy: number): [number, number] => [x + (fx * c - fy * s) * size, y + (fx * s + fy * c) * size];
  let body: [number, number][];
  switch (kind) {
    case 'Bomber':
    case 'Longhorn':
    case 'Floatplane':
      body = [pt(1.1, 0), pt(-0.7, 1.0), pt(-0.4, 0), pt(-0.7, -1.0)];
      break;
    case 'Scout':
      body = [pt(1.2, 0), pt(-0.6, 0.5), pt(-0.6, -0.5)];
      break;
    case 'Drone':
      body = [pt(0.8, 0), pt(-0.5, 0.6), pt(-0.5, -0.6)];
      break;
    case 'Prototype':
      g.ellipse(x, y, size * 1.1, size * 0.55).fill({ color, alpha: alpha * 0.9 }).stroke({ width: size * 0.18, color: 0xffffff, alpha: alpha * 0.6 });
      return;
    case 'Watchtower':
    case 'Jammer':
    case 'CropDuster':
      body = [pt(1.0, 0), pt(-0.6, 0.85), pt(-0.6, -0.85)];
      break;
    default:
      body = [pt(1.15, 0), pt(-0.65, 0.75), pt(-0.65, -0.75)];
  }
  g.poly(body.flat()).fill({ color: NAVY, alpha: alpha * 0.85 }).stroke({ width: Math.max(1, size * 0.22), color, alpha });
  drawShape(g, f, x - c * size * 0.05, y - s * size * 0.05, size * 0.32, color, alpha);
  if (kind === 'Watchtower') g.circle(x, y, size * 0.75).stroke({ width: size * 0.12, color, alpha: alpha * 0.8 });
  if (kind === 'Jammer') {
    const a = pt(-0.9, -0.5);
    const b = pt(-1.2, 0);
    const d = pt(-0.9, 0.5);
    g.moveTo(a[0], a[1]).lineTo(b[0], b[1]).lineTo(d[0], d[1]).stroke({ width: size * 0.15, color, alpha });
  }
  if (kind === 'Longhorn') {
    const l1 = pt(0.6, -0.4);
    const l2 = pt(1.0, -0.95);
    const r1 = pt(0.6, 0.4);
    const r2 = pt(1.0, 0.95);
    g.moveTo(l1[0], l1[1]).lineTo(l2[0], l2[1]).moveTo(r1[0], r1[1]).lineTo(r2[0], r2[1]).stroke({ width: size * 0.2, color: 0xfff3d6, alpha });
  }
}

export const NODE_LABEL: Record<NodeKind, string> = {
  Capital: 'Capital',
  City: 'City',
  Fuel: 'Fuel site',
  Factory: 'Factory',
  Airbase: 'Airbase',
  Radar: 'Radar site',
  AirDefense: 'Air defense',
  Junction: 'Junction',
  CrownJewel: 'Crown jewel',
  Lake: 'Lake',
  Boneyard: 'The Boneyard',
  MassDriver: 'Mass Driver',
  TruckStop: 'Truck stop',
};

/** Draw a node icon. `health` 2/1/0; `color` is the holder's color. */
export function drawNode(g: Graphics, kind: NodeKind, x: number, y: number, r: number, color: number, health: number, active = true) {
  const a = active ? 1 : 0.35;
  const fillA = (health === 2 ? 0.95 : health === 1 ? 0.55 : 0.18) * a;
  const line = { width: Math.max(1, r * 0.22), color, alpha: a };
  switch (kind) {
    case 'Capital': {
      g.circle(x, y, r * 1.5).stroke({ width: r * 0.25, color, alpha: 0.6 * a });
      const pts: number[] = [];
      for (let i = 0; i < 10; i++) {
        const ang = (Math.PI / 5) * i - Math.PI / 2;
        const rr = i % 2 === 0 ? r * 1.1 : r * 0.48;
        pts.push(x + Math.cos(ang) * rr, y + Math.sin(ang) * rr);
      }
      g.poly(pts).fill({ color, alpha: fillA });
      break;
    }
    case 'City':
      g.circle(x, y, r).fill({ color, alpha: fillA }).stroke(line);
      break;
    case 'Fuel':
      g.poly([x, y - r * 1.2, x + r * 0.85, y + r * 0.3, x, y + r, x - r * 0.85, y + r * 0.3]).fill({ color, alpha: fillA }).stroke(line);
      break;
    case 'Factory':
      g.rect(x - r, y - r * 0.7, r * 2, r * 1.5).fill({ color, alpha: fillA }).stroke(line);
      g.rect(x + r * 0.35, y - r * 1.3, r * 0.45, r * 0.7).fill({ color, alpha: fillA });
      break;
    case 'Airbase':
    case 'Boneyard':
      g.poly([x - r * 1.2, y + r * 0.4, x + r * 0.9, y - r * 0.9, x + r * 1.2, y - r * 0.4, x - r * 0.9, y + r * 0.9]).fill({ color, alpha: fillA }).stroke(line);
      if (kind === 'Boneyard') g.circle(x, y, r * 1.6).stroke({ width: r * 0.2, color, alpha: 0.5 * a });
      break;
    case 'Radar':
      g.moveTo(x + Math.cos(Math.PI * 1.05) * r * 1.1, y + Math.sin(Math.PI * 1.05) * r * 1.1)
        .arc(x, y, r * 1.1, Math.PI * 1.05, Math.PI * 1.95)
        .stroke({ width: r * 0.35, color, alpha: a });
      g.circle(x, y + r * 0.2, r * 0.45).fill({ color, alpha: fillA });
      break;
    case 'AirDefense':
      g.poly([x, y - r * 1.1, x + r, y + r * 0.8, x - r, y + r * 0.8]).fill({ color, alpha: fillA }).stroke(line);
      break;
    case 'Junction':
      g.circle(x, y, r * 0.75).fill({ color: NAVY, alpha: 0.9 * a }).stroke(line);
      g.circle(x, y, r * 0.3).fill({ color, alpha: fillA });
      break;
    case 'CrownJewel':
      g.poly([x, y - r * 1.5, x + r * 1.1, y - r * 0.3, x, y + r * 1.3, x - r * 1.1, y - r * 0.3]).fill({ color, alpha: fillA }).stroke({ width: r * 0.25, color: 0xffffff, alpha: 0.85 * a });
      break;
    case 'Lake':
      g.ellipse(x, y, r * 1.3, r * 0.8).fill({ color: 0x2a6fdb, alpha: 0.5 }).stroke({ width: r * 0.15, color: 0x6fb2ff, alpha: 0.8 });
      break;
    case 'MassDriver':
      g.rect(x - r * 1.4, y - r * 0.35, r * 2.8, r * 0.7).fill({ color, alpha: fillA }).stroke(line);
      g.poly([x + r * 1.4, y - r * 0.8, x + r * 2.1, y, x + r * 1.4, y + r * 0.8]).fill({ color, alpha: fillA });
      break;
    case 'TruckStop':
      g.roundRect(x - r * 0.75, y - r * 0.9, r * 1.5, r * 1.8, r * 0.3).fill({ color, alpha: fillA }).stroke(line);
      break;
  }
  if (health === 0 && kind !== 'Lake') {
    g.moveTo(x - r, y - r).lineTo(x + r, y + r).moveTo(x + r, y - r).lineTo(x - r, y + r).stroke({ width: r * 0.3, color: 0xff3b3b, alpha: 0.9 });
  }
}

export function nodeRadius(kind: NodeKind): number {
  switch (kind) {
    case 'Capital':
      return 7;
    case 'CrownJewel':
      return 6.5;
    case 'City':
    case 'Factory':
      return 5.5;
    case 'Junction':
    case 'TruckStop':
      return 4.5;
    default:
      return 5;
  }
}

export function unitName(content: Content, kind: UnitKind): string {
  return content.units.find((u) => u.kind === kind)?.name ?? kind;
}
