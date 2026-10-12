// Words: naming things, filling in blanks, and drawing dog lines from
// flavor.ron without repeating any within a game.

import type { Content, FactionId, Mission, Squadron, Target, SiteKind } from './types';

let content: Content;
export function setContent(c: Content) {
  content = c;
}
export function getContent(): Content {
  return content;
}

export function faction(f: FactionId) {
  return content.factions.find((x) => x.id === f)!;
}

export function leaderLines(f: FactionId) {
  return content.flavor.leaders[f];
}

export function radio(f: FactionId) {
  return content.flavor.radio[f];
}

export function fill(template: string, params: Record<string, string | number | undefined>): string {
  return template.replace(/\{(\w+)\}/g, (_, k) => {
    const v = params[k];
    return v === undefined ? '' : String(v);
  });
}

/** Griffin & Pan lines come as "Griffin: ... | Pan: ...". Split into turns. */
export function turns(line: string): { who: string | null; text: string }[] {
  return line.split(' | ').map((part) => {
    const i = part.indexOf(': ');
    if (i > 0 && i < 12 && (part.startsWith('Griffin') || part.startsWith('Pan'))) {
      return { who: part.slice(0, i), text: part.slice(i + 2) };
    }
    return { who: null, text: part };
  });
}

function hash(s: string, seed: number): number {
  let h = seed ^ 0x9e3779b9;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x85ebca6b) ^ (h >>> 13);
  h = Math.imul(h ^ (h >>> 16), 0xc2b2ae35);
  return (h ^ (h >>> 16)) >>> 0;
}

/**
 * Picks lines from pools in a shuffled order that's fixed by the game's seed,
 * so a game never repeats a line until its pool runs out, and a replay says
 * the same things. `used` counts draws per pool and lives in the save.
 */
export class Voice {
  constructor(private seed: number, readonly used: Record<string, number> = {}) {}

  take(key: string, pool: string[] | undefined): string | undefined {
    if (!pool || pool.length === 0) return undefined;
    const n = this.used[key] ?? 0;
    this.used[key] = n + 1;
    const round = Math.floor(n / pool.length);
    const order = pool.map((_, i) => i).sort((a, b) => hash(`${key}:${round}:${a}`, this.seed) - hash(`${key}:${round}:${b}`, this.seed));
    return pool[order[n % pool.length]];
  }
}

export function siteName(i: number): string {
  return content.sites[i]?.name ?? '';
}

export function prizeName(i: number): string {
  return content.prizes[i]?.name ?? '';
}

export function targetName(t: Target): string {
  return t.t === 'Site' ? siteName(t.site) : prizeName(t.prize);
}

export function siteOwner(i: number): FactionId {
  return content.sites[i].faction;
}

export const KIND_NAME: Record<SiteKind, string> = {
  HQ: 'HQ',
  Factory: 'Factory',
  Airfield: 'Airfield',
  Jewel: 'Crown Jewel',
};

export function squadronName(sq: Squadron, f: FactionId): string {
  if (sq.ace) return faction(f).ace;
  return `${sq.kind === 'Fighter' ? 'Fighters' : 'Bombers'} ${sq.num}`;
}

export function missionLabel(m: Mission): string {
  switch (m.t) {
    case 'Rest':
      return 'Rest';
    case 'Patrol':
      return 'Patrol';
    case 'Fly':
      return m.target.t === 'Prize' ? `Claim ${targetName(m.target)}` : `→ ${targetName(m.target)}`;
  }
}

export function stageName(night: number): string {
  const r = content.rules;
  return night >= r.open_hqs ? r.stage_names[2] : night >= r.open_factories ? r.stage_names[1] : r.stage_names[0];
}

export function stageIndex(night: number): number {
  const r = content.rules;
  return night >= r.open_hqs ? 2 : night >= r.open_factories ? 1 : 0;
}

/** What opens tonight, if anything, in a few plain words. */
export function opensTonight(night: number): string | null {
  const r = content.rules;
  if (night === r.open_factories) return 'Factories can be hit from tonight.';
  if (night === r.open_hqs) return 'HQs and Crown Jewels can be hit from tonight.';
  return null;
}

export function plural(n: number, one: string, many = one + 's'): string {
  return `${n} ${n === 1 ? one : many}`;
}
