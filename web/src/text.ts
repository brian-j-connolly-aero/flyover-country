// Words: picking lines from flavor.ron, filling in blanks, and naming things.

import type { Content, FactionId, GameEvent, Mission, NodeKind, PublicSeat, UnitKind } from './types';

let content: Content;
export function setContent(c: Content) {
  content = c;
}

export function faction(f: FactionId) {
  return content.factions.find((x) => x.id === f)!;
}

export function leader(f: FactionId) {
  return content.flavor.leaders[f];
}

/** Deterministic pick so replays say the same things. */
export function pick<T>(list: T[] | undefined, salt: number): T | undefined {
  if (!list || list.length === 0) return undefined;
  let h = (salt | 0) ^ 0x9e3779b9;
  h = Math.imul(h ^ (h >>> 16), 0x85ebca6b);
  h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35);
  h ^= h >>> 16;
  return list[(h >>> 0) % list.length];
}

let rot = Math.floor(Math.random() * 1000);
/** Non-deterministic pick for idle chatter (curtain filler, dawn quips). */
export function any<T>(list: T[] | undefined): T | undefined {
  rot++;
  return pick(list, rot * 7919);
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

export function nodeName(idx: number | null | undefined): string {
  if (idx === null || idx === undefined) return '';
  return content.nodes[idx]?.name ?? '';
}

export function zoneName(idx: number | null | undefined): string {
  if (idx === null || idx === undefined) return '';
  return content.zones[idx]?.name ?? '';
}

export function unitName(kind: UnitKind): string {
  return content.units.find((u) => u.kind === kind)?.name ?? kind;
}

export function unitDef(kind: UnitKind) {
  return content.units.find((u) => u.kind === kind)!;
}

export const KIND_LABEL: Record<NodeKind, string> = {
  Capital: 'Capital', City: 'City', Fuel: 'Fuel site', Factory: 'Factory', Airbase: 'Airbase',
  Radar: 'Radar site', AirDefense: 'Air defense site', Junction: 'Junction', CrownJewel: 'Crown jewel',
  Lake: 'Lake', Boneyard: 'Neutral objective', MassDriver: 'Neutral objective', TruckStop: 'Truck stop',
};

export function missionLabel(m: Mission, squads?: { id: number; label: string }[]): string {
  switch (m.t) {
    case 'Rest':
      return 'Rest at base';
    case 'Attack':
      return `Attack ${nodeName(m.node)}`;
    case 'Guard':
      return `Guard ${nodeName(m.node)}`;
    case 'GuardZone':
      return `Guard ${zoneName(m.zone)}`;
    case 'Escort':
      return `Escort ${squads?.find((s) => s.id === m.sq)?.label ?? 'a strike'}`;
    case 'Sweep':
      return `Sweep ${zoneName(m.zone)}`;
    case 'Scout':
      return `Scout ${zoneName(m.zone)}`;
    case 'JamZone':
      return `Jam ${zoneName(m.zone)}`;
    case 'JamEscort':
      return `Jam for ${squads?.find((s) => s.id === m.sq)?.label ?? 'a strike'}`;
    case 'Rebase':
      return `Rebase to ${nodeName(m.node)}`;
  }
}

export function stageName(stage: number): string {
  return content.rules.stage_names[stage] ?? '';
}

export function stageBlurb(stage: number): string {
  return [
    'Military targets only: airbases, radar and air defense. Race for the Big Empty.',
    'Cities, factories, fuel sites and junctions are fair game.',
    'Crown jewels and the Mass Driver are open. Strike scores double on the last night.',
  ][stage];
}

export function seatName(seats: PublicSeat[] | FactionId[], seat: number | null | undefined): string {
  if (seat === null || seat === undefined) return 'Nobody';
  const s = seats[seat];
  if (!s) return '';
  const f = typeof s === 'string' ? s : s.faction;
  return faction(f).short;
}

export function patchText(idx: number | null): { version: string; text: string } | null {
  if (idx === null || idx === undefined) return null;
  return content.patch_notes[idx] ?? null;
}

export function effectText(kind: string): string {
  return (
    {
      NoDrones: 'Scheduled maintenance: no drones launch.',
      FactoriesHalf: 'Factories at half power.',
      RadarSealed: 'Cheyenne Mountain sealed: radar blind.',
      FuelHalf: 'Fuel income halved.',
      StripDark: 'The Strip goes dark: zero Supply.',
      NoFirstShot: 'No fry sauce: guards lose their first shot.',
    } as Record<string, string>
  )[kind] ?? kind;
}

/** Advisor line for a warning, voiced by the faction's dog. */
export function advisorLine(f: FactionId, key: string, params: Record<string, string>, salt: number): string {
  const lines = content.flavor.advisor[f]?.[key];
  const line = pick(lines, salt);
  if (line) return fill(line, params);
  if (key === 'invalid') return `${params.unit ?? 'That squadron'}: ${params.reason ?? "can't do that."}`;
  return '';
}

export function chatter(f: FactionId, key: string, params: Record<string, string>, salt: number): string | null {
  const line = pick(content.flavor.chatter[f]?.[key], salt);
  return line ? fill(line, params) : null;
}

export function aceLine(f: FactionId, key: 'launch' | 'kill' | 'duel' | 'downed' | 'returns', salt: number): string | null {
  const a = content.flavor.aces[f];
  return a ? pick(a[key], salt) ?? null : null;
}

export function headline(key: string, params: Record<string, string>, salt: number): string | null {
  const line = pick(content.flavor.headlines[key], salt);
  if (!line) return null;
  const up: Record<string, string> = {};
  for (const [k, v] of Object.entries(params)) up[k] = v.toUpperCase();
  // "TEXAS'S THE GULF REFINERY" reads better as "TEXAS'S GULF REFINERY".
  return fill(line, up).replace(/'S THE /g, "'S ");
}

export function eventSalt(e: GameEvent, tick: number): number {
  const s = JSON.stringify(e);
  let h = tick * 31;
  for (let i = 0; i < s.length; i += 3) h = (Math.imul(h, 33) + s.charCodeAt(i)) | 0;
  return h;
}

export function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);
}
