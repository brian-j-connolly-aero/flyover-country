// Reads a night's public event log: important moments for the camera, the
// morning report's scorecard, and the newspaper headline.

import type { Content, EventRec, FactionId, GameEvent, P } from '../types';
import { faction, headline, nodeName, zoneName } from '../text';

export interface Moment {
  tick: number;
  pos: P;
  importance: number;
  /** Ticks the camera should hold on this. */
  hold: number;
  kind: string;
  label: string;
}

export interface NightSummary {
  night: number;
  factions: FactionId[];
  gained: number[];
  squadronsLost: number[];
  stepsDealt: number[];
  stepsTaken: number[];
  kills: number[];
  headline: string;
  highlights: string[];
  play: Moment | null;
  jewelHit: boolean[];
}

function short(factions: FactionId[], seat: number | null | undefined): string {
  if (seat === null || seat === undefined) return 'Nobody';
  return faction(factions[seat]).short;
}

export function moments(content: Content, events: EventRec[], factions: FactionId[]): Moment[] {
  const out: Moment[] = [];
  const pos = (n: number): P => content.nodes[n].pos;
  for (const r of events) {
    const e = r.e;
    const t = r.tick;
    switch (e.t) {
      case 'Strike': {
        if (e.steps <= 0) {
          out.push({ tick: t, pos: pos(e.node), importance: 25, hold: 6, kind: 'miss', label: `${short(factions, e.seat)} misses ${nodeName(e.node)}` });
          break;
        }
        const kind = content.nodes[e.node].kind;
        const imp = e.jewel ? 100 : kind === 'City' || kind === 'Factory' ? 60 : 45;
        out.push({ tick: t, pos: pos(e.node), importance: imp, hold: e.jewel ? 22 : 10, kind: e.jewel ? 'jewel' : 'strike', label: `${short(factions, e.seat)} hits ${nodeName(e.node)}` });
        break;
      }
      case 'MassDriver':
      case 'Revenge':
        out.push({ tick: t, pos: pos(e.node), importance: 75, hold: 14, kind: 'massdriver', label: `${short(factions, e.seat)} ${e.t === 'MassDriver' ? 'fires the Mass Driver at' : 'takes revenge on'} ${nodeName(e.node)}` });
        break;
      case 'Engage': {
        const splashes = e.after.filter((s) => s.strength === 0).length;
        const imp = e.ace_duel ? 85 : 38 + 6 * splashes;
        out.push({
          tick: t, pos: [e.x, e.y], importance: imp, hold: e.ace_duel ? 18 : 8, kind: e.ace_duel ? 'duel' : 'dogfight',
          label: e.ace_duel ? 'Ace duel' : `${short(factions, e.a_seat)} and ${short(factions, e.b_seat)} tangle`,
        });
        break;
      }
      case 'Splash':
        if (e.ace) out.push({ tick: t, pos: [e.x, e.y], importance: 80, hold: 12, kind: 'acedown', label: `${content.factions.find((f) => f.id === factions[e.seat])?.ace} is down` });
        break;
      case 'Flak':
        if (e.shots.some((s) => s.hit)) out.push({ tick: t, pos: [e.x, e.y], importance: 18, hold: 4, kind: 'flak', label: 'Flak' });
        break;
      case 'LostInStatic':
      case 'Vanished':
        out.push({ tick: t, pos: [e.x, e.y], importance: 50, hold: 10, kind: e.t, label: e.t === 'Vanished' ? 'The saucer is gone' : 'Lost in the Static' });
        break;
      case 'Pivot':
        out.push({ tick: t, pos: pos(e.node), importance: 48, hold: 8, kind: 'pivot', label: 'PIVOT pivots' });
        break;
      case 'BetWin':
        out.push({ tick: t, pos: e.node !== null ? pos(e.node) : [content.width / 2, content.height / 2], importance: 55, hold: 8, kind: 'bet', label: "Dreamland's bet pays off" });
        break;
      case 'Capture':
        out.push({ tick: t, pos: pos(e.node), importance: 30, hold: 6, kind: 'capture', label: `${short(factions, e.seat)} takes ${nodeName(e.node)}` });
        break;
    }
  }
  return out;
}

export function summarize(content: Content, events: EventRec[], factions: FactionId[], night: number, salt: number): NightSummary {
  const n = factions.length;
  const gained = new Array(n).fill(0);
  const squadronsLost = new Array(n).fill(0);
  const stepsDealt = new Array(n).fill(0);
  const stepsTaken = new Array(n).fill(0);
  const kills = new Array(n).fill(0);
  const jewelHit = new Array(n).fill(false);
  const highlights: string[] = [];
  const pub = events.filter((r) => r.public);
  let bigStrike: (GameEvent & { t: 'Strike' }) | null = null;
  const splashByZone = new Map<number, number>();
  const heads: { pri: number; text: string | null }[] = [];
  const h = (pri: number, key: string, params: Record<string, string>) => heads.push({ pri, text: headline(key, params, salt + pri) });
  for (const r of pub) {
    const e = r.e;
    switch (e.t) {
      case 'Score':
        gained[e.seat] += e.delta;
        break;
      case 'Splash':
        squadronsLost[e.seat] += 1;
        if (e.by_seat !== null) kills[e.by_seat] += 1;
        {
          let zi = 0;
          let bd = Infinity;
          for (const z of content.zones) {
            const d = (z.center[0] - e.x) ** 2 + (z.center[1] - e.y) ** 2;
            if (d < bd) {
              bd = d;
              zi = z.idx;
            }
          }
          splashByZone.set(zi, (splashByZone.get(zi) ?? 0) + 1);
        }
        if (e.ace) {
          const ace = content.factions.find((f) => f.id === factions[e.seat])!.ace;
          h(80, 'ace_down', { ace, attacker: short(factions, e.by_seat) });
          highlights.push(`${ace} went down${e.by_seat !== null ? ` to ${short(factions, e.by_seat)}` : ''}. Good chute.`);
        }
        break;
      case 'Strike':
        if (e.victim !== null) {
          stepsDealt[e.seat] += e.steps;
          stepsTaken[e.victim] += e.steps;
        }
        if (e.jewel && e.steps > 0 && e.victim !== null) {
          jewelHit[e.victim] = true;
          h(100, `jewel_hit_${factions[e.victim]}`, { attacker: short(factions, e.seat) });
          highlights.push(`${short(factions, e.seat)} hit ${nodeName(e.node)}!`);
        } else if (e.steps > 0 && (!bigStrike || e.steps > bigStrike.steps)) {
          bigStrike = e;
        }
        if (e.kinds.includes('Longhorn') && e.steps > 0) h(52, 'longhorn', {});
        if (e.kinds.includes('Drone') && e.steps > 0) h(42, 'drones', {});
        break;
      case 'MassDriver':
        h(70, 'mass_driver', { attacker: short(factions, e.seat), node: nodeName(e.node) });
        highlights.push(`${short(factions, e.seat)} fired the White Sands Mass Driver at ${nodeName(e.node)}.`);
        if (e.victim !== null) {
          stepsDealt[e.seat] += e.steps;
          stepsTaken[e.victim] += e.steps;
        }
        break;
      case 'Revenge':
        highlights.push(`The Grain Belt took its revenge on ${nodeName(e.node)}.`);
        break;
      case 'Capture':
        h(35, 'capture', { attacker: short(factions, e.seat), node: nodeName(e.node) });
        highlights.push(`${short(factions, e.seat)} took ${nodeName(e.node)}.`);
        break;
      case 'BetWin':
        h(58, 'bet', {});
        highlights.push(`Dreamland's bet paid ${e.payout} Supply.`);
        break;
      case 'NightStart':
        if (e.harvest_winner !== null) {
          h(30, 'harvest', { attacker: short(factions, e.harvest_winner) });
          highlights.push(`${short(factions, e.harvest_winner)} won the Harvest Shipment for ${e.harvest_bid} Supply.`);
        }
        break;
      case 'LostInStatic':
        h(60, 'static', {});
        highlights.push('A squadron flew into the Static. Its pilots refuse to talk about it.');
        break;
      case 'Vanished':
        h(56, 'prototype', {});
        highlights.push("Dreamland's Prototype simply didn't come back.");
        break;
      case 'JewelEffect':
        break;
      case 'AceReturns':
        highlights.push(`${content.factions.find((f) => f.id === factions[e.seat])!.ace} is back.`);
        break;
      case 'Casserole':
        highlights.push(`Utah earned a thank-you casserole${e.host !== null ? ` from ${short(factions, e.host)}` : ''}.`);
        break;
    }
  }
  if (bigStrike && bigStrike.victim !== null) {
    h(50, 'big_strike', { attacker: short(factions, bigStrike.seat), victim: short(factions, bigStrike.victim), node: nodeName(bigStrike.node) });
  }
  let topZone: [number, number] | null = null;
  for (const [z, c] of splashByZone) if (!topZone || c > topZone[1]) topZone = [z, c];
  if (topZone && topZone[1] >= 3) h(45 + topZone[1], 'dogfight', { count: String(topZone[1]), zone: zoneName(topZone[0]) });
  const scrappyBig = gained.indexOf(Math.max(...gained));
  if (gained[scrappyBig] > 0 && pub.some((r) => r.e.t === 'Dawn')) {
    const dawn = pub.find((r) => r.e.t === 'Dawn')!.e as GameEvent & { t: 'Dawn' };
    void dawn;
  }
  if (heads.length === 0) h(1, 'quiet', {});
  heads.sort((a, b) => b.pri - a.pri);
  const hl = heads.find((x) => x.text)?.text ?? 'A QUIET NIGHT OVER THE WEST';
  const ms = moments(content, pub, factions);
  ms.sort((a, b) => b.importance - a.importance || a.tick - b.tick);
  return { night, factions, gained, squadronsLost, stepsDealt, stepsTaken, kills, headline: hl, highlights: highlights.slice(0, 6), play: ms[0] ?? null, jewelHit };
}

export function finalHeadline(factions: FactionId[], scores: number[], salt: number): string {
  const best = scores.indexOf(Math.max(...scores));
  return headline('final', { attacker: faction(factions[best]).short }, salt) ?? `${faction(factions[best]).short.toUpperCase()} TAKES THE BRAGGING RIGHTS`;
}

export function leaderHeadline(factions: FactionId[], scores: number[], salt: number): string | null {
  const best = scores.indexOf(Math.max(...scores));
  if (scores[best] <= 0) return null;
  return headline('leader', { attacker: faction(factions[best]).short }, salt);
}
