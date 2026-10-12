// Plays a resolved night back one moment at a time. Raids fly in straight
// lines from their Airfield; whenever something happens (a patrol jumps a
// raid, bombs land, a prize changes hands) the clock stops, the camera moves
// in, and the result is stamped on the map with one line of caption. The
// outcome is already decided, so pause, speed up and skip are all safe.

import { Graphics } from 'pixi.js';
import type { MapView } from '../map/mapview';
import type { Content, EventRec, FactionId, GameState, Loss, P, UnitKind } from '../types';
import { HIT_RED, INK, PAPER, dashedCircle, dashedLine, drawPlane, factionColor, mix } from '../theme';
import type { Caption } from './summary';

export interface ShowHooks {
  caption(c: Caption | null): void;
  score(seat: number, delta: number): void;
  sound(kind: string): void;
  ended(): void;
  progress?(k: number): void;
}

interface RaidTrack {
  id: number;
  seat: number;
  color: number;
  from: P;
  to: P;
  launch: number;
  arrive: number;
  fighters: number;
  bombers: number;
  sqs: number[];
  /** Tick it stopped flying in: arrived, wiped out, or parked on a prize. */
  done: number | null;
  /** Parked on this prize (holding it). */
  holding: number | null;
  /** Tick it started heading home, fading as it goes. */
  home: number | null;
  /** Where it was when shot down. */
  lostAt: P | null;
}

interface PatrolTrack {
  sq: number;
  seat: number;
  color: number;
  at: P;
  r: number;
  planes: number;
  phase: number;
}

interface Fx {
  kind: 'ring' | 'chute' | 'boom' | 'puff' | 'smoke' | 'boing';
  x: number;
  y: number;
  vx: number;
  vy: number;
  color: number;
  born: number;
  dur: number;
  size: number;
}

/** Milliseconds per tick at 1x while raids are in the air. */
const TICK_MS = 85;
/** How long each kind of moment holds the screen at 1x. */
const HOLD: Record<string, number> = {
  BarkPark: 1500,
  Intercept: 1700,
  Flak: 1300,
  StrikeHit: 1800,
  StrikeMiss: 1100,
  Bounce: 1400,
  Hold: 1200,
  PrizeFight: 1700,
  Fetch: 2000,
};

const BEATS = new Set(['BarkPark', 'Intercept', 'Strike', 'Bounce', 'Hold', 'PrizeFight', 'Fetch']);

export class ShowPlayer {
  readonly map: MapView;
  readonly content: Content;
  readonly factions: FactionId[];
  readonly events: EventRec[];
  private hooks: ShowHooks;
  private captions = new Map<number, Caption>();
  private kinds = new Map<number, UnitKind>();
  private raids = new Map<number, RaidTrack>();
  private patrols: PatrolTrack[] = [];
  private park = new Set<number>();
  private view: GameState;
  private after: GameState;
  private fx: Fx[] = [];
  private pathG = new Graphics();
  private unitG = new Graphics();
  private fxG = new Graphics();
  private parkG = new Graphics();
  private ball: { from: P; to: P; t0: number; t1: number; heard: boolean } | null = null;
  private fetchAt = new Map<number, number>();
  private i = 0;
  private hold = 0;
  private intro = 900;
  private outro = -1;
  private stamps = 0;
  private now = 0;
  private maxTick: number;
  t = 0;
  speed = 1;
  paused = false;
  done = false;

  constructor(map: MapView, content: Content, factions: FactionId[], events: EventRec[], before: GameState, after: GameState, captions: Caption[], hooks: ShowHooks) {
    this.map = map;
    this.content = content;
    this.factions = factions;
    this.events = events;
    this.hooks = hooks;
    this.view = structuredClone(before);
    this.after = after;
    for (const c of captions) this.captions.set(c.index, c);
    for (const s of before.squadrons) this.kinds.set(s.id, s.kind);
    this.maxTick = Math.max(1, ...events.filter((e) => e.t !== 'Dawn').map((e) => (e.t === 'Launch' ? e.arrive : e.tick)));
    const fetchTick = content.rules.bark_park.fetch_tick;
    events.forEach((e, k) => {
      if (e.t === 'Fetch') this.fetchAt.set(k, fetchTick);
      if (e.t === 'BarkPark') this.park.add(e.seat);
    });
    map.layers.plan.addChild(this.parkG, this.pathG);
    map.layers.units.addChild(this.unitG);
    map.layers.fx.addChild(this.fxG);
    map.setMarks(null);
    map.setState(this.view);
    this.drawPark();
    this.unsubscribe = map.onTick((dt) => this.frame(dt));
  }
  private unsubscribe: () => void;

  destroy() {
    this.done = true;
    this.unsubscribe();
    for (const g of [this.pathG, this.unitG, this.fxG, this.parkG]) g.destroy();
    this.map.removeLabels('stamp-');
    this.map.removeLabels('pop-');
  }

  setSpeed(s: number) {
    this.speed = s;
  }

  togglePause() {
    this.paused = !this.paused;
  }

  /** Jump to the end: every score lands, the map shows the morning. */
  skip() {
    if (this.done) return;
    while (this.i < this.events.length) {
      const r = this.events[this.i++];
      if (r.t === 'Score') this.hooks.score(r.seat, r.delta);
    }
    this.finish();
  }

  private finish() {
    if (this.done) return;
    this.done = true;
    this.hold = 0;
    this.raids.clear();
    this.patrols = [];
    this.ball = null;
    this.fx = [];
    this.pathG.clear();
    this.unitG.clear();
    this.fxG.clear();
    this.parkG.clear();
    this.map.removeLabels('stamp-');
    this.map.removeLabels('pop-');
    this.map.setState(this.after);
    this.map.setNight(0);
    { const home = this.map.home(); this.map.focus(home.x, home.y, home.zoom); }
    this.hooks.caption(null);
    this.hooks.progress?.(1);
    this.hooks.ended();
  }

  // --- The clock -------------------------------------------------------------------------------

  private frame(dt: number) {
    if (this.done) return;
    this.now += dt;
    if (!this.paused) {
      const sdt = dt * this.speed;
      if (this.intro > 0) {
        this.intro -= sdt;
        this.map.setNight(Math.min(1, 1 - this.intro / 900));
      } else if (this.outro >= 0) {
        this.outro += sdt;
        this.map.setNight(Math.max(0, 1 - this.outro / 900));
        if (this.outro >= 900) this.finish();
      } else if (this.hold > 0) {
        this.hold -= sdt;
        if (this.hold <= 0) this.endBeat();
      } else {
        this.t += sdt / TICK_MS;
        this.pump();
      }
    }
    if (this.done) return;
    this.draw(dt);
    this.hooks.progress?.(Math.min(1, this.t / this.maxTick));
  }

  /** Apply every event up to the clock, stopping at the next moment worth showing. */
  private pump() {
    while (this.hold <= 0 && this.i < this.events.length) {
      const r = this.events[this.i];
      if (r.t === 'Dawn') break;
      // The Big Fetch takes off a little before it lands.
      const fetchTick = this.fetchAt.get(this.i);
      if (fetchTick !== undefined && r.t === 'Fetch' && !this.ball) {
        const from = this.content.landmarks.find((l) => l.id === 'bark_park')?.pos ?? [0, 0];
        this.ball = { from, to: this.content.sites[r.site].pos, t0: fetchTick - 14, t1: fetchTick, heard: false };
      }
      if (r.tick > this.t) break;
      this.i++;
      this.apply(r, this.i - 1);
    }
    if (this.hold > 0) return;
    // A raid with nothing left to do when it gets there (fighters on a sweep,
    // or a target already flattened) just turns for home.
    for (const r of this.raids.values()) if (r.done === null && this.t > r.arrive) this.goHome(r, r.arrive);
    const next = this.events[this.i];
    const moving = this.flying();
    if (!next || next.t === 'Dawn') {
      if (!moving) this.outro = 0;
      return;
    }
    // Nothing in the air: skip ahead to the next thing that happens.
    if (!moving && next.tick > this.t + 2) {
      const lead = this.fetchAt.has(this.i) ? 15 : 1;
      this.t = Math.max(this.t, next.tick - lead);
    }
  }

  private flying(): boolean {
    for (const r of this.raids.values()) {
      if (r.done === null) return true;
      if (r.home !== null && this.t < r.home + 16) return true;
    }
    return !!this.ball && this.t < this.ball.t1;
  }

  private apply(r: EventRec, index: number) {
    switch (r.t) {
      case 'Patrol':
        this.patrols.push({ sq: r.sq, seat: r.seat, color: this.color(r.seat), at: r.at, r: r.r, planes: r.planes, phase: Math.random() * Math.PI * 2 });
        break;
      case 'Launch': {
        this.raids.set(r.raid, {
          id: r.raid, seat: r.seat, color: this.color(r.seat), from: r.from, to: r.to, launch: r.tick, arrive: r.arrive,
          fighters: r.fighters, bombers: r.bombers, sqs: r.sqs, done: null, holding: null, home: null, lostAt: null,
        });
        if (this.raids.size === 1) this.hooks.sound('launch');
        break;
      }
      case 'BarkPark': {
        const pos = this.content.landmarks.find((l) => l.id === 'bark_park')?.pos ?? [0, 0];
        this.beat(index, pos, 'AT THE PARK', 'park', HOLD.BarkPark);
        this.hooks.sound('park');
        break;
      }
      case 'Intercept': {
        const raid = this.raids.get(r.raid);
        const down = this.lose(raid, r.raid_lost);
        const p = this.patrols.find((x) => x.sq === r.patrol);
        if (p) p.planes = Math.max(0, p.planes - r.patrol_lost);
        this.burst(r.at, this.color(r.patrol_seat), 'ring', 1);
        this.chutes(r.at, down, this.color(r.seat));
        this.chutes(r.at, r.patrol_lost, this.color(r.patrol_seat));
        if (raid && r.wiped) {
          raid.done = r.tick;
          raid.lostAt = r.at;
        }
        const stamp = r.wiped ? 'WIPED OUT' : down > 0 ? `${down} DOWN` : 'NO HITS';
        this.beat(index, r.at, stamp, down > 0 ? 'hit' : 'miss', HOLD.Intercept);
        this.hooks.sound('guns');
        break;
      }
      case 'Flak': {
        const raid = this.raids.get(r.raid);
        const pos = this.content.sites[r.site].pos;
        const down = this.lose(raid, r.raid_lost);
        for (let k = 0; k < 5; k++) this.burst([pos[0] + (Math.random() - 0.5) * 30, pos[1] - 8 - Math.random() * 18], INK, 'puff', 0.6);
        this.chutes(pos, down, this.color(r.seat));
        this.hooks.sound('flak');
        if (this.captions.has(index)) {
          if (raid && raid.fighters + raid.bombers === 0) raid.done = r.tick;
          this.beat(index, pos, `FLAK: ${down} DOWN`, 'hit', HOLD.Flak);
        }
        break;
      }
      case 'Strike': {
        const raid = this.raids.get(r.raid);
        const site = this.content.sites[r.site];
        if (raid) this.goHome(raid, r.tick);
        this.view.sites[r.site].health = r.health;
        this.map.setState(this.view);
        if (r.hits > 0) {
          const big = site.kind === 'HQ' || site.kind === 'Jewel';
          this.burst(site.pos, HIT_RED, 'boom', big ? 1.5 : 1.1);
          this.map.shake = big ? 1 : 0.6;
          this.hooks.sound(big ? 'bigboom' : 'boom');
          this.beat(index, site.pos, r.health === 0 ? 'KNOCKED OUT' : `HIT −${r.hits}`, 'hit', HOLD.StrikeHit);
        } else {
          for (let k = 0; k < 3; k++) this.burst([site.pos[0] + (Math.random() - 0.5) * 34, site.pos[1] + (Math.random() - 0.5) * 34], INK, 'smoke', 0.7);
          this.hooks.sound('thud');
          this.beat(index, site.pos, 'MISSED', 'miss', HOLD.StrikeMiss);
        }
        break;
      }
      case 'Bounce': {
        const raid = this.raids.get(r.raid);
        const pos = this.content.sites[r.site].pos;
        if (raid) this.goHome(raid, r.tick);
        this.burst(pos, 0x8fbf6a, 'boing', 1);
        this.hooks.sound('boing');
        this.beat(index, pos, 'BOING!', 'park', HOLD.Bounce);
        break;
      }
      case 'Hold': {
        const raid = this.raids.get(r.raid);
        const pos = this.content.prizes[r.prize].pos;
        if (raid) {
          raid.done = r.tick;
          raid.holding = r.prize;
        }
        this.view.prizes[r.prize].owner = r.seat;
        this.map.setState(this.view);
        this.burst(pos, this.color(r.seat), 'ring', 1);
        this.hooks.sound('coin');
        this.beat(index, pos, 'TAKEN', 'take', HOLD.Hold);
        break;
      }
      case 'PrizeFight': {
        const raid = this.raids.get(r.raid);
        const pos = this.content.prizes[r.prize].pos;
        const down = this.lose(raid, r.raid_lost);
        let held = 0;
        for (const l of r.holder_lost) {
          held += l.planes;
          for (const x of this.raids.values()) if (x.holding === r.prize && x.sqs.includes(l.sq)) this.lose(x, [l]);
        }
        this.chutes(pos, down, this.color(r.seat));
        this.chutes(pos, held, this.color(r.holder));
        this.burst(pos, this.color(r.taken ? r.seat : r.holder), 'ring', 1.2);
        if (raid) {
          raid.done = r.tick;
          if (r.taken) {
            for (const x of this.raids.values()) if (x.holding === r.prize) this.goHome(x, r.tick);
            raid.holding = r.prize;
            this.view.prizes[r.prize].owner = r.seat;
          } else {
            this.goHome(raid, r.tick);
            let left = 0;
            for (const x of this.raids.values()) if (x.holding === r.prize) left += x.fighters + x.bombers;
            if (left === 0) this.view.prizes[r.prize].owner = null;
          }
        }
        this.map.setState(this.view);
        this.hooks.sound('guns');
        this.beat(index, pos, r.taken ? 'TAKEN' : 'HELD', r.taken ? 'take' : 'miss', HOLD.PrizeFight);
        break;
      }
      case 'Fetch': {
        const pos = this.content.sites[r.site].pos;
        this.ball = null;
        if (r.bounced) {
          this.burst(pos, 0x8fbf6a, 'boing', 1.3);
          this.hooks.sound('boing');
          this.beat(index, pos, 'BOING!', 'park', HOLD.Fetch);
        } else {
          this.view.sites[r.site].health = r.health;
          this.map.setState(this.view);
          this.burst(pos, 0xd7e63e, 'boom', 1.7);
          this.map.shake = 1.2;
          this.hooks.sound('bigboom');
          this.beat(index, pos, `FETCH −${r.hits}`, 'hit', HOLD.Fetch);
        }
        break;
      }
      case 'Score':
        this.hooks.score(r.seat, r.delta);
        break;
      default:
        break;
    }
  }

  /** Stop the clock on a moment: camera in, stamp on the map, one caption. */
  private beat(index: number, pos: P, stamp: string, kind: string, ms: number) {
    const r = this.events[index];
    this.t = Math.max(this.t, r.tick);
    // Points scored in this moment land with it.
    while (this.i < this.events.length) {
      const n = this.events[this.i];
      if (n.tick !== r.tick || (n.t !== 'Score' && n.t !== 'SquadronDown')) break;
      this.i++;
      if (n.t === 'Score') {
        this.hooks.score(n.seat, n.delta);
        const pop = this.map.label(`pop-${this.stamps}-${n.seat}`, `+${n.delta}`, [pos[0] + 16, pos[1] - 18], 'label-pop');
        pop.style.setProperty('--fc', `#${this.color(n.seat).toString(16).padStart(6, '0')}`);
      }
    }
    if (!BEATS.has(r.t) && !this.captions.has(index)) return;
    this.map.removeLabels('stamp-');
    this.map.label(`stamp-${this.stamps++}`, stamp, [pos[0], pos[1] - 26], `label-stamp ${kind}`);
    const zoom = this.map.home().zoom * (r.t === 'BarkPark' ? 1.6 : 2.1);
    this.map.focus(pos[0], pos[1] + 10, zoom);
    this.hooks.caption(this.captions.get(index) ?? null);
    this.hold = ms;
  }

  private endBeat() {
    this.hold = 0;
    this.map.removeLabels('stamp-');
    this.map.removeLabels('pop-');
    // Pull back out unless another moment follows right away.
    let next: EventRec | null = null;
    for (let k = this.i; k < this.events.length && !next; k++) if (BEATS.has(this.events[k].t) || this.captions.has(k)) next = this.events[k];
    if (!next || next.tick > this.t + 6) {
      const home = this.map.home();
      this.map.focus(home.x, home.y, home.zoom);
    }
  }

  private goHome(raid: RaidTrack, tick: number) {
    raid.done = tick;
    raid.holding = null;
    raid.home = tick;
  }

  private lose(raid: RaidTrack | undefined, losses: Loss[]): number {
    let n = 0;
    for (const l of losses) {
      n += l.planes;
      if (!raid) continue;
      if (this.kinds.get(l.sq) === 'Bomber') raid.bombers = Math.max(0, raid.bombers - l.planes);
      else raid.fighters = Math.max(0, raid.fighters - l.planes);
    }
    return n;
  }

  private color(seat: number): number {
    return factionColor(this.factions[seat]);
  }

  // --- Drawing ---------------------------------------------------------------------------------

  private drawPark() {
    const g = this.parkG;
    g.clear();
    for (const seat of this.park) {
      const f = this.factions[seat];
      this.content.sites.forEach((s, i) => {
        if (s.faction !== f || !this.view.sites[i].active) return;
        dashedCircle(g, s.pos[0], s.pos[1], 17, 2.5, 2.4, 0x8fbf6a, 0.95);
      });
    }
  }

  private pos(r: RaidTrack): P {
    const k = Math.max(0, Math.min(1, (this.t - r.launch) / Math.max(1, r.arrive - r.launch)));
    return [r.from[0] + (r.to[0] - r.from[0]) * k, r.from[1] + (r.to[1] - r.from[1]) * k];
  }

  private draw(dt: number) {
    const paths = this.pathG;
    const g = this.unitG;
    paths.clear();
    g.clear();
    const time = this.now / 1000;
    // Patrols: a ring of the area they guard, their fighters circling it.
    for (const p of this.patrols) {
      if (p.planes <= 0) continue;
      g.circle(p.at[0], p.at[1], p.r).fill({ color: p.color, alpha: 0.1 });
      dashedCircle(g, p.at[0], p.at[1], p.r, 6, 2.2, mix(p.color, PAPER, 0.25), 0.9);
      for (let k = 0; k < p.planes; k++) {
        const a = p.phase + time * 0.6 + (k / p.planes) * Math.PI * 2;
        drawPlane(g, 'Fighter', p.at[0] + Math.cos(a) * p.r, p.at[1] + Math.sin(a) * p.r, 5.5, a + Math.PI / 2, p.color, 1, PAPER);
      }
    }
    for (const r of this.raids.values()) {
      if (this.t < r.launch) continue;
      const n = r.fighters + r.bombers;
      // The route, faint ahead and solid behind, so you can see where it's going.
      if (r.done === null) {
        const at = this.pos(r);
        dashedLine(paths, at, r.to, 5, 1.6, mix(r.color, PAPER, 0.3), 0.75);
        paths.moveTo(r.from[0], r.from[1]).lineTo(at[0], at[1]).stroke({ width: 2.4, color: r.color, alpha: 0.8 });
        if (n > 0) this.formation(g, r, at, Math.atan2(r.to[1] - r.from[1], r.to[0] - r.from[0]), 1);
      } else if (r.holding !== null && n > 0) {
        const c = this.content.prizes[r.holding].pos;
        const a = time * 0.9 + r.id;
        this.formation(g, r, [c[0] + Math.cos(a) * 18, c[1] + Math.sin(a) * 18], a + Math.PI / 2, 1);
      } else if (r.home !== null && n > 0) {
        const k = (this.t - r.home) / 16;
        if (k >= 1) continue;
        const dx = r.from[0] - r.to[0];
        const dy = r.from[1] - r.to[1];
        const len = Math.hypot(dx, dy) || 1;
        const d = k * 90;
        this.formation(g, r, [r.to[0] + (dx / len) * d, r.to[1] + (dy / len) * d], Math.atan2(dy, dx), 1 - k);
      }
    }
    // The Big Fetch: a tennis ball the size of a stadium, lobbed from the park.
    if (this.ball && this.t >= this.ball.t0) {
      const b = this.ball;
      if (!b.heard) {
        b.heard = true;
        this.hooks.sound('fetch');
      }
      const k = Math.min(1, (this.t - b.t0) / (b.t1 - b.t0));
      const x = b.from[0] + (b.to[0] - b.from[0]) * k;
      const y = b.from[1] + (b.to[1] - b.from[1]) * k;
      const hgt = Math.sin(k * Math.PI) * 90;
      g.ellipse(x, y, 8, 3.5).fill({ color: INK, alpha: 0.35 });
      g.circle(x, y - hgt, 9 + Math.sin(k * Math.PI) * 4).fill({ color: 0xd7e63e }).stroke({ width: 1.6, color: INK });
      g.moveTo(x - 6, y - hgt - 4).quadraticCurveTo(x, y - hgt + 2, x + 6, y - hgt - 4).stroke({ width: 1.2, color: PAPER });
    }
    this.drawFx(dt);
  }

  /** A raid as you'd see it from above: bombers in a V, fighters on the wings. */
  private formation(g: Graphics, r: RaidTrack, at: P, angle: number, alpha: number) {
    const c = Math.cos(angle);
    const s = Math.sin(angle);
    const put = (along: number, side: number): P => [at[0] + along * c - side * s, at[1] + along * s + side * c];
    for (let i = 0; i < r.bombers; i++) {
      const row = Math.ceil(i / 2);
      const [x, y] = put(-row * 9, (i % 2 ? 1 : -1) * row * 9);
      drawPlane(g, 'Bomber', x, y, 7, angle, r.color, alpha, PAPER);
    }
    for (let i = 0; i < r.fighters; i++) {
      const row = Math.floor(i / 2);
      const [x, y] = put(5 - row * 9, (i % 2 ? 1 : -1) * (r.bombers ? 17 : 6 + row * 4));
      drawPlane(g, 'Fighter', x, y, 5.5, angle, r.color, alpha, PAPER);
    }
  }

  private burst(at: P, color: number, kind: Fx['kind'], size: number) {
    const base = { x: at[0], y: at[1], vx: 0, vy: 0, color, born: this.now, size };
    if (kind === 'boom') {
      this.fx.push({ ...base, kind: 'boom', dur: 900 });
      for (let k = 0; k < 6; k++) {
        const a = Math.random() * Math.PI * 2;
        this.fx.push({ ...base, kind: 'smoke', vx: Math.cos(a) * 9, vy: Math.sin(a) * 9 - 6, dur: 1800 + Math.random() * 600, size: size * (0.6 + Math.random() * 0.5) });
      }
    } else {
      this.fx.push({ ...base, kind, dur: kind === 'puff' ? 700 + Math.random() * 400 : kind === 'smoke' ? 1600 : 1000 });
    }
  }

  /** One little parachute per plane lost. The pups are fine. */
  private chutes(at: P, n: number, color: number) {
    for (let k = 0; k < n; k++) {
      this.fx.push({ kind: 'chute', x: at[0] + (k - (n - 1) / 2) * 11, y: at[1] - 6, vx: (Math.random() - 0.5) * 3, vy: 7, color, born: this.now + k * 120, dur: 2600, size: 1 });
    }
  }

  private drawFx(dt: number) {
    const g = this.fxG;
    g.clear();
    const now = this.now;
    this.fx = this.fx.filter((f) => now - f.born < f.dur);
    for (const f of this.fx) {
      const age = now - f.born;
      if (age < 0) continue;
      const k = age / f.dur;
      f.x += (f.vx * dt) / 1000;
      f.y += (f.vy * dt) / 1000;
      switch (f.kind) {
        case 'ring':
          g.circle(f.x, f.y, (10 + k * 34) * f.size).stroke({ width: 3 * (1 - k) + 0.5, color: f.color, alpha: 1 - k });
          break;
        case 'boing':
          for (let j = 0; j < 3; j++) {
            const kk = Math.max(0, k - j * 0.15);
            if (kk > 0) g.circle(f.x, f.y, (14 + kk * 30) * f.size).stroke({ width: 2.5, color: f.color, alpha: 1 - kk });
          }
          break;
        case 'boom': {
          const r = (8 + Math.sqrt(k) * 24) * f.size;
          g.circle(f.x, f.y, r).fill({ color: 0xf2b134, alpha: (1 - k) * 0.85 });
          g.circle(f.x, f.y, r * 0.6).fill({ color: f.color, alpha: 1 - k });
          g.circle(f.x, f.y, r).stroke({ width: 2, color: INK, alpha: (1 - k) * 0.8 });
          break;
        }
        case 'puff':
          g.circle(f.x, f.y, (3 + k * 6) * f.size * 2).fill({ color: mix(INK, PAPER, 0.35), alpha: (1 - k) * 0.8 });
          break;
        case 'smoke':
          g.circle(f.x, f.y, (5 + k * 12) * f.size).fill({ color: mix(INK, PAPER, 0.25), alpha: (1 - k) * 0.55 });
          break;
        case 'chute': {
          const sway = Math.sin(age / 260 + f.x) * 3;
          const x = f.x + sway;
          const y = f.y;
          const a = k > 0.75 ? (1 - k) / 0.25 : 1;
          g.moveTo(x - 6, y).arc(x, y, 6, Math.PI, 0).closePath().fill({ color: PAPER, alpha: a }).stroke({ width: 1.2, color: f.color, alpha: a });
          g.moveTo(x - 6, y).lineTo(x, y + 8).lineTo(x + 6, y).stroke({ width: 0.8, color: PAPER, alpha: a * 0.8 });
          g.circle(x, y + 9, 2.2).fill({ color: f.color, alpha: a }).stroke({ width: 0.8, color: PAPER, alpha: a });
          break;
        }
      }
    }
  }
}
