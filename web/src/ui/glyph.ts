// Small inline SVG glyphs for HTML (squadron cards, legends).

import type { FactionId, UnitKind } from '../types';
import { FACTION_HEX, SHAPES } from '../theme';

function shapeSvg(f: FactionId, cx: number, cy: number, r: number, color: string): string {
  switch (SHAPES[f]) {
    case 'circle':
      return `<circle cx="${cx}" cy="${cy}" r="${r}" fill="${color}"/>`;
    case 'ring':
      return `<circle cx="${cx}" cy="${cy}" r="${r * 0.8}" fill="none" stroke="${color}" stroke-width="${r * 0.5}"/>`;
    case 'triangle':
      return `<polygon points="${cx},${cy - r * 1.1} ${cx + r},${cy + r * 0.75} ${cx - r},${cy + r * 0.75}" fill="${color}"/>`;
    case 'diamond':
      return `<polygon points="${cx},${cy - r * 1.2} ${cx + r},${cy} ${cx},${cy + r * 1.2} ${cx - r},${cy}" fill="${color}"/>`;
    case 'square':
      return `<rect x="${cx - r * 0.85}" y="${cy - r * 0.85}" width="${r * 1.7}" height="${r * 1.7}" fill="${color}"/>`;
    case 'hexagon': {
      const p = [...Array(6)].map((_, i) => {
        const a = (Math.PI / 3) * i + Math.PI / 6;
        return `${(cx + Math.cos(a) * r).toFixed(1)},${(cy + Math.sin(a) * r).toFixed(1)}`;
      });
      return `<polygon points="${p.join(' ')}" fill="${color}"/>`;
    }
    case 'star': {
      const p = [...Array(10)].map((_, i) => {
        const a = (Math.PI / 5) * i - Math.PI / 2;
        const rr = i % 2 === 0 ? r * 1.2 : r * 0.5;
        return `${(cx + Math.cos(a) * rr).toFixed(1)},${(cy + Math.sin(a) * rr).toFixed(1)}`;
      });
      return `<polygon points="${p.join(' ')}" fill="${color}"/>`;
    }
  }
}

export function glyphSvg(f: FactionId, kind: UnitKind, size = 28, ghost = false): string {
  const color = FACTION_HEX[f];
  const op = ghost ? 0.45 : 1;
  let body: string;
  switch (kind) {
    case 'Bomber':
    case 'Longhorn':
    case 'Floatplane':
      body = `<polygon points="14,2 25,22 14,17 3,22" fill="#071022" stroke="${color}" stroke-width="2.2" stroke-linejoin="round"/>`;
      break;
    case 'Scout':
      body = `<polygon points="14,1 21,24 7,24" fill="#071022" stroke="${color}" stroke-width="2" stroke-linejoin="round"/>`;
      break;
    case 'Prototype':
      body = `<ellipse cx="14" cy="15" rx="12" ry="6" fill="#071022" stroke="${color}" stroke-width="2.2"/>`;
      break;
    case 'Drone':
      body = `<polygon points="14,5 22,22 6,22" fill="#071022" stroke="${color}" stroke-width="2" stroke-dasharray="3 2"/>`;
      break;
    default:
      body = `<polygon points="14,2 24,24 4,24" fill="#071022" stroke="${color}" stroke-width="2.2" stroke-linejoin="round"/>`;
  }
  let extra = '';
  if (kind === 'Watchtower') extra = `<circle cx="14" cy="16" r="9" fill="none" stroke="${color}" stroke-width="1.2"/>`;
  if (kind === 'Longhorn') extra = `<path d="M9 9 L4 4 M19 9 L24 4" stroke="#fff3d6" stroke-width="2"/>`;
  if (kind === 'Jammer') extra = `<path d="M2 26 L6 22 L10 26 L14 22 L18 26 L22 22 L26 26" stroke="${color}" stroke-width="1.4" fill="none"/>`;
  if (kind === 'CropDuster') extra = `<circle cx="22" cy="24" r="2.5" fill="#bfc7d5" opacity="0.8"/><circle cx="6" cy="24" r="2.5" fill="#bfc7d5" opacity="0.8"/>`;
  if (kind === 'Gull') extra = `<path d="M6 10 Q10 6 14 10 Q18 6 22 10" stroke="#fff" stroke-width="1.5" fill="none"/>`;
  return `<svg viewBox="0 0 28 28" width="${size}" height="${size}" xmlns="http://www.w3.org/2000/svg" opacity="${op}">${body}${extra}${shapeSvg(f, 14, 16, 3.6, color)}</svg>`;
}
