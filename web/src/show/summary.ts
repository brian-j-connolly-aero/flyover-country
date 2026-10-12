// What a night means in words: the morning headline, up to four plain
// highlights, a caption for each moment of the show, and which dog gets to
// speak at dawn. Lines come from the save's Voice, so nothing repeats.

import type { Content, EventRec, FactionId, GameState, Loss } from '../types';
import { faction, fill, leaderLines, prizeName, radio, siteName, type Voice, KIND_NAME, plural } from '../text';

export interface Caption {
  tick: number;
  /** Which event in the night's log this caption belongs to. */
  index: number;
  /** Plain words: who did what. */
  text: string;
  /** A short radio call from that side, sometimes. */
  radio: string | null;
  faction: FactionId;
}

export interface NightSummary {
  headline: string;
  highlights: string[];
  captions: Caption[];
  /** Bragging Rights gained tonight, per seat. */
  gained: number[];
  /** The dog who speaks next morning, and why. */
  speaker: { seat: number; pool: 'hit' | 'landed' | 'ahead' | 'behind' | 'back' | 'dawn'; params: Record<string, string> } | null;
  returned: number[];
}

function lost(list: Loss[]): number {
  return list.reduce((a, l) => a + l.planes, 0);
}

export function summarize(content: Content, events: EventRec[], factions: FactionId[], before: GameState, after: GameState, voice: Voice): NightSummary {
  const short = (seat: number) => faction(factions[seat]).short;
  const gained = factions.map(() => 0);
  const captions: Caption[] = [];
  const stories: { weight: number; key: string; params: Record<string, string>; line: string }[] = [];
  let radioGap = -999;
  const call = (tick: number, seat: number, pool: 'intercept' | 'jumped' | 'hit' | 'miss' | 'prize', params: Record<string, string>) => {
    // Radio at most every so often, so the screen stays quiet.
    if (tick - radioGap < 28) return null;
    radioGap = tick;
    const line = voice.take(`radio.${factions[seat]}.${pool}`, radio(factions[seat])[pool]);
    return line ? fill(line, params) : null;
  };
  const raidSeat = new Map<number, number>();
  const flak = new Map<number, number>();
  const raidBombers = new Map<number, number>();
  let hitSeat = null as { seat: number; site: number; dealt: number } | null;
  let bestLanded = null as { seat: number; site: number; victim: number; dealt: number } | null;
  const returned: number[] = [];
  events.forEach((r, index) => {
    switch (r.t) {
      case 'Launch':
        raidSeat.set(r.raid, r.seat);
        raidBombers.set(r.raid, r.bombers);
        break;
      case 'BarkPark':
        captions.push({ tick: r.tick, index, text: `${short(r.seat)} spends the night at Project Bark Park.`, radio: null, faction: factions[r.seat] });
        stories.push({ weight: 30, key: 'park', params: { attacker: short(r.seat) }, line: `${short(r.seat)} rested at Project Bark Park.` });
        break;
      case 'Intercept': {
        const down = lost(r.raid_lost);
        const words = r.wiped ? 'the whole raid goes down' : down === 0 ? 'no losses' : `${plural(down, 'plane')} down`;
        captions.push({
          tick: r.tick,
          index,
          text: `${short(r.patrol_seat)} jumps the ${short(r.seat)} raid: ${words}.`,
          radio: call(r.tick, r.patrol_seat, 'intercept', { rival: short(r.seat) }),
          faction: factions[r.patrol_seat],
        });
        if (r.wiped) stories.push({ weight: 40, key: 'wiped', params: { attacker: short(r.seat), victim: short(r.patrol_seat) }, line: `${short(r.patrol_seat)} shot down a whole ${short(r.seat)} raid.` });
        break;
      }
      case 'Flak': {
        const down = lost(r.raid_lost);
        const strike = events.slice(index + 1).some((x) => x.t === 'Strike' && x.raid === r.raid);
        if (down > 0 && !strike) {
          const site = content.sites[r.site];
          captions.push({ tick: r.tick, index, text: `Flak over ${site.name} downs ${plural(down, 'plane')}. No bombs land.`, radio: null, faction: factions[r.victim] });
        }
        if (down > 0) flak.set(r.raid, down);
        break;
      }
      case 'Strike': {
        const site = content.sites[r.site];
        const shot = flak.get(r.raid) ?? 0;
        const what = (r.hits > 0 ? `hit ${site.name} (−${r.hits})` : `miss ${site.name}`) + (shot ? `; flak got ${shot}` : '');
        captions.push({
          tick: r.tick,
          index,
          text: `${short(r.seat)} bombers ${what}.`,
          radio: call(r.tick, r.seat, r.hits > 0 ? 'hit' : 'miss', { site: site.name }),
          faction: factions[r.seat],
        });
        if (r.hits > 0) {
          const big = site.kind === 'HQ' || site.kind === 'Jewel';
          const key = site.kind === 'HQ' ? 'hq' : site.kind === 'Jewel' ? 'jewel' : 'strike';
          stories.push({
            weight: (big ? 30 : 15) + r.hits * 5 + (r.health === 0 ? 10 : 0),
            key,
            params: { attacker: short(r.seat), victim: short(r.victim), site: site.name },
            line: `${short(r.seat)} hit ${short(r.victim)}'s ${KIND_NAME[site.kind]}, ${site.name}${r.health === 0 ? ', and knocked it out' : ''}.`,
          });
          if (!bestLanded || r.hits > bestLanded.dealt) bestLanded = { seat: r.seat, site: r.site, victim: r.victim, dealt: r.hits };
          if (!hitSeat || r.hits > hitSeat.dealt) hitSeat = { seat: r.victim, site: r.site, dealt: r.hits };
        }
        break;
      }
      case 'Bounce':
        captions.push({ tick: r.tick, index, text: `Boing! ${short(r.seat)}'s bombs bounce off the Bark Park fence.`, radio: null, faction: factions[r.seat] });
        break;
      case 'Hold':
        captions.push({
          tick: r.tick,
          index,
          text: `${short(r.seat)} takes ${prizeName(r.prize)}.`,
          radio: call(r.tick, r.seat, 'prize', { prize: prizeName(r.prize) }),
          faction: factions[r.seat],
        });
        break;
      case 'PrizeFight': {
        const name = prizeName(r.prize);
        captions.push({
          tick: r.tick,
          index,
          text: r.taken ? `${short(r.seat)} takes ${name} from ${short(r.holder)}.` : `${short(r.holder)} holds ${name} against ${short(r.seat)}.`,
          radio: r.taken ? call(r.tick, r.seat, 'prize', { prize: name }) : null,
          faction: factions[r.taken ? r.seat : r.holder],
        });
        break;
      }
      case 'Fetch': {
        const site = content.sites[r.site];
        const shout = r.bounced ? undefined : voice.take(`leader.${factions[r.seat]}.fetch`, leaderLines(factions[r.seat]).fetch);
        captions.push({ tick: r.tick, index, text: r.bounced ? `The Big Fetch bounces off the Bark Park fence.` : `${short(r.seat)} throws the Big Fetch at ${site.name} (−${r.hits}).`, radio: shout ? fill(shout, { site: site.name }) : null, faction: factions[r.seat] });
        if (!r.bounced) stories.push({ weight: 60, key: 'fetch', params: { attacker: short(r.seat), victim: short(r.victim), site: site.name }, line: `${short(r.seat)} threw the Big Fetch at ${site.name}.` });
        break;
      }
      case 'Score':
        gained[r.seat] += r.delta;
        break;
      case 'Dawn':
        returned.push(...r.returned);
        r.prizes.forEach((owner, i) => {
          if (owner !== null && before.prizes[i]?.owner !== owner) {
            stories.push({ weight: 12, key: 'prize', params: { attacker: short(owner), prize: prizeName(i) }, line: `${short(owner)} now holds ${prizeName(i)}.` });
          }
        });
        break;
    }
  });
  stories.sort((a, b) => b.weight - a.weight);
  const top = stories[0];
  const pool = content.flavor.headlines[top?.key ?? 'quiet'] ?? content.flavor.headlines.quiet;
  const headline = fill(voice.take(`headline.${top?.key ?? 'quiet'}`, pool) ?? 'Another night over the West', top?.params ?? {});
  // The headline already tells the top story.
  const highlights = stories.slice(1, 4).map((s) => s.line);

  // Who speaks at dawn: whoever got hit hardest, else whoever landed the
  // biggest strike, else the leader or the one at the back, in turn.
  let speaker: NightSummary['speaker'] = null;
  const night = after.night;
  if (returned.length) speaker = { seat: returned[0], pool: 'back', params: {} };
  else if (hitSeat && night % 2 === 0) speaker = { seat: hitSeat.seat, pool: 'hit', params: { site: siteName(hitSeat.site) } };
  else if (bestLanded) speaker = { seat: bestLanded.seat, pool: 'landed', params: { site: siteName(bestLanded.site), rival: short(bestLanded.victim) } };
  else if (hitSeat) speaker = { seat: hitSeat.seat, pool: 'hit', params: { site: siteName(hitSeat.site) } };
  else {
    const scores = after.seats.map((s) => s.score);
    const best = scores.indexOf(Math.max(...scores));
    const worst = scores.indexOf(Math.min(...scores));
    speaker = night % 2 === 0 ? { seat: best, pool: 'ahead', params: {} } : { seat: worst, pool: 'behind', params: {} };
  }
  return { headline, highlights, captions, gained, speaker, returned };
}

export function raidLabel(fighters: number, bombers: number): string {
  const parts: string[] = [];
  if (bombers) parts.push(plural(bombers, 'bomber'));
  if (fighters) parts.push(plural(fighters, 'fighter'));
  return parts.join(' + ');
}
