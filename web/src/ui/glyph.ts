// Small inline SVG icons for HTML, drawn the same way as the map: flat ink
// fills with a dark outline.

import type { FactionId, SiteKind, UnitKind } from '../types';
import { FACTION_HEX } from '../theme';

const INK = '#231e1a';
const PAPER = '#f3ead7';

export function planeSvg(kind: UnitKind, color: string, size = 28, angle = -90): string {
  const fighter = 'M22,0 L2,4 L-4,19 L-9,19 L-7,4 L-16,4.4 L-19,10 L-21,10 L-19,0 L-21,-10 L-19,-10 L-16,-4.4 L-7,-4 L-9,-19 L-4,-19 L2,-4 Z';
  const bomber = 'M20,0 L15,3.2 L4,3.2 L1,23 L-5,23 L-4,3.2 L-15,3.2 L-18.4,10 L-21,10 L-19.6,0 L-21,-10 L-18.4,-10 L-15,-3.2 L-4,-3.2 L-5,-23 L1,-23 L4,-3.2 L15,-3.2 Z';
  return `<svg viewBox="-25 -25 50 50" width="${size}" height="${size}" aria-hidden="true"><path d="${kind === 'Fighter' ? fighter : bomber}" transform="rotate(${angle})" fill="${color}" stroke="${INK}" stroke-width="2.4" stroke-linejoin="round"/></svg>`;
}

export function factionPlaneSvg(f: FactionId, kind: UnitKind, size = 28): string {
  return planeSvg(kind, FACTION_HEX[f], size);
}

function starPts(cx: number, cy: number, o: number, i: number): string {
  const p: string[] = [];
  for (let k = 0; k < 10; k++) {
    const a = (Math.PI / 5) * k - Math.PI / 2;
    const r = k % 2 === 0 ? o : i;
    p.push(`${(cx + Math.cos(a) * r).toFixed(1)},${(cy + Math.sin(a) * r).toFixed(1)}`);
  }
  return p.join(' ');
}

/** A site token: paper disc, ink ring, pictogram in the owner's ink. */
export function siteSvg(kind: SiteKind, color: string, size = 26, down = false): string {
  const fill = down ? '#e7d9bb' : color;
  let art = '';
  switch (kind) {
    case 'HQ':
      art = `<polygon points="${starPts(20, 21, 10, 4.4)}" fill="${fill}" stroke="${INK}" stroke-width="1.6"/>`;
      break;
    case 'Factory':
      art = `<path d="M9,28 L9,19 L14,14 L14,19 L19,14 L19,19 L24,14 L24,10 L30,10 L30,28 Z" fill="${fill}" stroke="${INK}" stroke-width="1.6" stroke-linejoin="round"/>`;
      break;
    case 'Airfield':
      art = `<g transform="rotate(-34 20 20)"><rect x="8" y="16" width="24" height="8" fill="${fill}" stroke="${INK}" stroke-width="1.6"/><path d="M11,20 h3 M17,20 h3 M23,20 h3" stroke="${PAPER}" stroke-width="1.4"/></g>`;
      break;
    case 'Jewel':
      art = `<polygon points="20,9 30,18 20,31 10,18" fill="${fill}" stroke="${INK}" stroke-width="1.6" stroke-linejoin="round"/><path d="M10,18 H30" stroke="${INK}" stroke-width="1"/>`;
      break;
  }
  const x = down ? `<path d="M12,12 L28,28 M28,12 L12,28" stroke="#b8321e" stroke-width="3"/>` : '';
  return `<svg viewBox="0 0 40 40" width="${size}" height="${size}" aria-hidden="true"><circle cx="21" cy="21.5" r="17" fill="${INK}" opacity="0.3"/><circle cx="20" cy="20" r="17" fill="${PAPER}" stroke="${INK}" stroke-width="2"/>${art}${x}</svg>`;
}

/** A prize pennant in its holder's ink (paper when nobody holds it). */
export function prizeSvg(color: string | null, size = 26): string {
  return `<svg viewBox="0 0 40 40" width="${size}" height="${size}" aria-hidden="true"><circle cx="21" cy="21.5" r="17" fill="${INK}" opacity="0.3"/><circle cx="20" cy="20" r="17" fill="${PAPER}" stroke="${INK}" stroke-width="2"/><path d="M15,30 V9" stroke="${INK}" stroke-width="2"/><path d="M15,9 L29,14 L15,19 Z" fill="${color ?? '#e7d9bb'}" stroke="${INK}" stroke-width="1.6" stroke-linejoin="round"/></svg>`;
}

/** A Supply crate. */
export function crateSvg(size = 20): string {
  return `<svg viewBox="0 0 24 24" width="${size}" height="${size}" aria-hidden="true"><rect x="3" y="5" width="18" height="15" rx="1.5" fill="#c79a5b" stroke="${INK}" stroke-width="1.8"/><path d="M3,10 H21 M8,5 V20 M16,5 V20" stroke="${INK}" stroke-width="1.3"/></svg>`;
}

/** Health pips as squares: filled for health left, hollow for lost. */
export function pipsHtml(health: number, max: number): string {
  let s = '<span class="pips">';
  for (let i = 0; i < max; i++) s += `<i class="${i < health ? 'on' : ''}"></i>`;
  return s + '</span>';
}
