// Plays a resolved night back as a show: glowing tracks, dogfights, flak,
// strikes and parachutes, with a camera that holds on the biggest moment.
// The outcome is already decided, so pause, speed up and skip are safe.

import { Graphics } from 'pixi.js';
import type { MapView, NodeState } from '../map/mapview';
import type { Content, EventRec, FactionId, GameEvent, MissionTag, P, PublicView, SqSnap, UnitKind } from '../types';
import { drawGlyph, factionColor } from '../theme';
import { aceLine, chatter, eventSalt, faction, leader, nodeName, pick, unitName, zoneName } from '../text';
import { barkPark, moments, type Moment } from './summary';

interface GroupTrack {
  id: number;
  seat: number;
  faction: FactionId;
  kinds: UnitKind[];
  ace: boolean;
  mission: MissionTag;
  pts: [number, number, number][];
  first: number;
  last: number;
}

interface Fx {
  kind: 'ring' | 'flash' | 'chute' | 'tracer' | 'flak' | 'boom' | 'smoke' | 'static' | 'arc' | 'text';
  x: number;
  y: number;
  x2?: number;
  y2?: number;
  color: number;
  born: number;
  dur: number;
  size: number;
}

export interface ShowHooks {
  chatter(who: string, color: string, text: string, priority: number): void;
  banner(title: string, sub: string, kind: 'update' | 'jackpot' | 'static' | 'title' | 'park'): void;
  score(seat: number, delta: number): void;
  sound(kind: string): void;
  ended(): void;
  progress?(t: number): void;
}

const TICKS = 360;

export class ShowPlayer {
  readonly map: MapView;
  readonly content: Content;
  readonly factions: FactionId[];
  readonly events: EventRec[];
  private hooks: ShowHooks;
  private groups = new Map<number, GroupTrack>();
  private sqGroup = new Map<number, number>();
  private dwell: number[] = [];
  private moments: Moment[];
  private fired = 0;
  private fx: Fx[] = [];
  private unitG = new Graphics();
  private fxG = new Graphics();
  private trailG = new Graphics();
  private clock = 0; // real ms since start (scaled by speed)
  private now = 0; // real ms, unscaled, for effects
  private cumul: number[] = [];
  t = 0;
  speed = 1;
  paused = false;
  done = false;
  private endAt: number | null = null;
  private nodes: NodeState[];
  private finalNodes: NodeState[];
  private range: [number, number] | null = null;
  private lockFocus: Moment | null = null;
  private quietNarration = false;
  /** Project Bark Park: who's at the park tonight, their fence, and their radio. */
  private park = new Set<number>();
  private parkG = new Graphics();
  private parkLines: { tick: number; seat: number; n: number; done: boolean }[] = [];
  /** The Big Fetch: lifts off a few ticks before it lands. */
  private fetches: { seat: number; node: number; tick: number; launched: boolean }[] = [];
  private ballG = new Graphics();
  private boings = 0;

  constructor(map: MapView, content: Content, factions: FactionId[], events: EventRec[], before: PublicView, after: PublicView, hooks: ShowHooks) {
    this.map = map;
    this.content = content;
    this.factions = factions;
    this.events = events.filter((e) => e.public);
    this.hooks = hooks;
    this.nodes = before.nodes.map((n) => ({ owner: n.owner, health: n.health, active: n.active }));
    this.finalNodes = after.nodes.map((n) => ({ owner: n.owner, health: n.health, active: n.active }));
    for (const r of this.events) {
      const e = r.e;
      if (e.t === 'Launch') {
        this.groups.set(e.group, {
          id: e.group, seat: e.seat, faction: factions[e.seat], kinds: e.squadrons.map((s) => s.kind),
          ace: e.squadrons.some((s) => s.ace), mission: e.mission, pts: [], first: r.tick, last: r.tick,
        });
        for (const s of e.squadrons) this.sqGroup.set(s.sq, e.group);
      }
      if (e.t === 'Track') {
        const g = this.groups.get(e.group);
        if (g) {
          g.pts = e.pts;
          if (e.pts.length) {
            g.first = e.pts[0][0];
            g.last = e.pts[e.pts.length - 1][0];
          }
        }
      }
    }
    const fetchTick = content.rules.bark_park?.fetch_tick ?? 240;
    for (const r of this.events) {
      const e = r.e;
      if (e.t === 'BarkPark') {
        this.park.add(e.seat);
        [45, 135, 225, 305].forEach((t, n) => this.parkLines.push({ tick: t + e.seat * 7, seat: e.seat, n, done: false }));
      }
      if (e.t === 'Fetch' || (e.t === 'Bounce' && e.group === null && r.tick === fetchTick)) {
        this.fetches.push({ seat: e.seat, node: e.node, tick: r.tick, launched: false });
      }
    }
    this.moments = moments(content, this.events, factions);
    this.buildDwell();
    map.layers.links.addChildAt(this.parkG, 0);
    map.layers.fx.addChild(this.ballG);
    map.layers.units.addChild(this.trailG, this.unitG);
    map.layers.fx.addChild(this.fxG);
    this.fxG.blendMode = 'add';
    this.trailG.blendMode = 'add';
    map.setNodes(this.nodes.map((n) => ({ ...n })));
    this.unsubscribe = map.onTick((dt) => this.frame(dt));
  }
  private unsubscribe: () => void;
  private destroyed = false;

  destroy() {
    this.done = true;
    this.destroyed = true;
    this.unsubscribe();
    this.unitG.destroy();
    this.fxG.destroy();
    this.trailG.destroy();
    this.parkG.destroy();
    this.ballG.destroy();
    this.map.removeLabels('boing-');
  }

  private buildDwell() {
    const big = new Array(TICKS + 2).fill(0);
    for (const m of this.moments) {
      for (let t = Math.max(0, m.tick - 3); t <= Math.min(TICKS, m.tick + Math.max(6, m.hold)); t++) big[t] = Math.max(big[t], m.importance);
    }
    const airborne = new Array(TICKS + 2).fill(0);
    for (const g of this.groups.values()) for (let t = g.first; t <= Math.min(TICKS, g.last); t++) airborne[t]++;
    this.dwell = [];
    let acc = 0;
    // The Big Fetch is in the air for the three ticks before it lands.
    for (const f of this.fetches) for (let t = Math.max(0, f.tick - 3); t < f.tick; t++) airborne[t]++;
    for (let t = 0; t <= TICKS; t++) {
      let d = airborne[t] > 0 ? 90 : 22;
      if (big[t] >= 30) d = 380;
      if (big[t] >= 80) d = 700; // slow motion on ace duels and crown jewels
      this.dwell.push(d);
      this.cumul.push(acc);
      acc += d;
    }
    this.cumul.push(acc);
  }

  /** Total show length at 1x, in seconds. */
  get length(): number {
    return this.cumul[this.cumul.length - 1] / 1000;
  }

  private tickAt(ms: number): number {
    // Binary search the cumulative dwell table.
    let lo = 0;
    let hi = this.cumul.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (this.cumul[mid] <= ms) lo = mid;
      else hi = mid - 1;
    }
    const base = this.cumul[lo];
    const span = this.dwell[lo] ?? 1;
    return Math.min(TICKS + 1, lo + Math.min(1, (ms - base) / span));
  }

  setSpeed(s: number) {
    this.speed = s;
  }

  togglePause() {
    this.paused = !this.paused;
  }

  skip() {
    // Fire everything that's left so the scoreboard ends right, then stop.
    this.range = null;
    this.clock = this.cumul[this.cumul.length - 1];
    this.t = TICKS + 1;
    while (this.fired < this.events.length) this.fire(this.events[this.fired++], true);
    this.map.setNodes(this.finalNodes.map((n) => ({ ...n })));
    this.fx = [];
    this.finish();
  }

  /** Replay a slice (the Play of the Night) with the camera locked on it. */
  playRange(m: Moment, onDone: () => void) {
    const t0 = Math.max(0, m.tick - 14);
    const t1 = Math.min(TICKS, m.tick + 14);
    this.range = [t0, t1];
    this.lockFocus = m;
    this.done = false;
    this.endAt = null;
    this.clock = this.cumul[t0];
    this.t = t0;
    this.fired = this.events.findIndex((r) => r.tick >= t0);
    if (this.fired < 0) this.fired = this.events.length;
    for (const f of this.fetches) if (f.tick >= t0) f.launched = false;
    this.quietNarration = true;
    this.fx = [];
    this.replayDone = onDone;
  }
  private replayDone: (() => void) | null = null;

  private finish() {
    if (this.done) return;
    this.done = true;
    this.map.setNodes(this.finalNodes.map((n) => ({ ...n })));
    this.unitG.clear();
    this.trailG.clear();
    this.hooks.ended();
  }

  private frame(dt: number) {
    if (this.destroyed) return;
    this.now += dt;
    if (this.done && !this.range) {
      this.drawFx();
      this.drawPark();
      this.ballG.clear();
      return;
    }
    if (!this.paused) {
      const slowRange = this.range ? 0.6 : 1;
      this.clock += dt * this.speed * slowRange;
      this.t = this.tickAt(this.clock);
    }
    this.launchFetches();
    while (this.fired < this.events.length && this.events[this.fired].tick <= this.t) {
      if (this.range && this.events[this.fired].tick > this.range[1]) break;
      this.fire(this.events[this.fired++], false);
    }
    this.parkRadio();
    this.drawUnits();
    this.drawFx();
    this.drawPark();
    this.drawBalls();
    this.direct();
    this.hooks.progress?.(this.t);
    if (this.range && this.t >= this.range[1]) {
      this.range = null;
      this.lockFocus = null;
      this.quietNarration = false;
      this.done = true;
      this.unitG.clear();
      this.trailG.clear();
      const cb = this.replayDone;
      this.replayDone = null;
      this.map.setNodes(this.finalNodes.map((n) => ({ ...n })));
      cb?.();
      return;
    }
    if (!this.range && this.t >= TICKS && this.endAt === null) this.endAt = this.now + 1200;
    if (this.endAt !== null && this.now >= this.endAt) this.finish();
  }

  // --- Camera director ---------------------------------------------------------------------

  private direct() {
    if (this.lockFocus) {
      this.map.focus(this.lockFocus.pos[0], this.lockFocus.pos[1], 2.4);
      return;
    }
    const t = this.t;
    const flying = this.fetches.find((f) => t >= f.tick - 3 && t < f.tick);
    if (flying) {
      const a = barkPark(this.content);
      const b = this.content.nodes[flying.node].pos;
      const d = Math.hypot(b[0] - a[0], b[1] - a[1]);
      this.map.focus((a[0] + b[0]) / 2, (a[1] + b[1]) / 2 - d * 0.18, Math.max(1, Math.min(2.2, (this.content.width * 0.5) / Math.max(d, 120))));
      return;
    }
    let best: Moment | null = null;
    for (const m of this.moments) {
      if (t >= m.tick - 6 && t <= m.tick + m.hold) {
        // Never cut away from an ace duel in progress.
        if (!best || m.importance > best.importance || (best.kind !== 'duel' && m.kind === 'duel')) best = m;
      }
    }
    if (best) {
      const z = best.importance >= 80 ? 2.5 : best.importance >= 50 ? 2.0 : 1.6;
      this.map.focus(best.pos[0], best.pos[1], z);
      return;
    }
    // Otherwise follow the visible traffic, or pull back to the whole West.
    let n = 0;
    let sx = 0;
    let sy = 0;
    for (const g of this.groups.values()) {
      if (t < g.first || t > g.last) continue;
      const p = this.posAt(g, t);
      if (!p) continue;
      sx += p[0];
      sy += p[1];
      n++;
    }
    if (n > 0) this.map.focus(sx / n, sy / n, n === 1 ? 1.5 : 1.2);
    else this.map.focus(this.content.width / 2 + 10, this.content.height / 2, 1);
  }

  // --- Drawing ---------------------------------------------------------------------------------

  private posAt(g: GroupTrack, t: number): P | null {
    const pts = g.pts;
    if (!pts.length || t < pts[0][0] || t > pts[pts.length - 1][0] + 0.001) return null;
    let i = 0;
    while (i < pts.length - 1 && pts[i + 1][0] < t) i++;
    const a = pts[i];
    const b = pts[Math.min(i + 1, pts.length - 1)];
    const span = b[0] - a[0];
    const k = span > 0 ? (t - a[0]) / span : 0;
    return [a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k];
  }

  private drawUnits() {
    const g = this.unitG;
    const tr = this.trailG;
    g.clear();
    tr.clear();
    const t = this.t;
    const zoom = this.map.cam.zoom;
    for (const grp of this.groups.values()) {
      if (t < grp.first || t > grp.last) continue;
      const p = this.posAt(grp, t);
      if (!p) continue;
      const color = factionColor(grp.faction);
      // Contrail: the last 30 ticks, fading.
      const trail: P[] = [];
      for (const q of grp.pts) if (q[0] >= t - 30 && q[0] <= t) trail.push([q[1], q[2]]);
      trail.push(p);
      for (let i = 1; i < trail.length; i++) {
        const a = i / trail.length;
        tr.moveTo(trail[i - 1][0], trail[i - 1][1]).lineTo(trail[i][0], trail[i][1]).stroke({ width: 2.2 / zoom + 0.6, color, alpha: a * 0.55 });
      }
      const prev = this.posAt(grp, Math.max(grp.first, t - 1.5)) ?? p;
      let ang = Math.atan2(p[1] - prev[1], p[0] - prev[0]);
      if (!Number.isFinite(ang) || (p[0] === prev[0] && p[1] === prev[1])) ang = -Math.PI / 2;
      const size = 6.5 / Math.sqrt(zoom);
      const lead = grp.kinds.includes('Longhorn') ? 'Longhorn' : grp.kinds[0];
      g.circle(p[0], p[1], size * 1.8).fill({ color, alpha: 0.12 });
      drawGlyph(g, grp.faction, lead, p[0], p[1], size, ang);
      if (grp.kinds.length > 1) {
        for (let k = 1; k < Math.min(grp.kinds.length, 4); k++) {
          const off = k * size * 1.2;
          drawGlyph(g, grp.faction, grp.kinds[k], p[0] - Math.cos(ang) * off + Math.sin(ang) * off * 0.6 * (k % 2 ? 1 : -1), p[1] - Math.sin(ang) * off - Math.cos(ang) * off * 0.6 * (k % 2 ? 1 : -1), size * 0.75, ang);
        }
      }
      if (grp.ace) g.circle(p[0], p[1], size * 1.6).stroke({ width: 1, color: 0xffffff, alpha: 0.7 });
    }
  }

  private addFx(f: Omit<Fx, 'born'>) {
    this.fx.push({ ...f, born: this.now });
  }

  private drawFx() {
    const g = this.fxG;
    g.clear();
    const keep: Fx[] = [];
    for (const f of this.fx) {
      const k = (this.now - f.born) / f.dur;
      if (k >= 1) continue;
      keep.push(f);
      const a = 1 - k;
      switch (f.kind) {
        case 'ring':
          g.circle(f.x, f.y, f.size * (0.3 + k)).stroke({ width: 2.5 * a + 0.5, color: f.color, alpha: a });
          break;
        case 'boom':
          g.circle(f.x, f.y, f.size * (0.2 + k * 1.4)).fill({ color: f.color, alpha: a * 0.35 });
          g.circle(f.x, f.y, f.size * (0.1 + k * 0.6)).fill({ color: 0xffffff, alpha: a * 0.6 });
          g.circle(f.x, f.y, f.size * (0.4 + k * 2.2)).stroke({ width: 3 * a, color: f.color, alpha: a });
          break;
        case 'flash':
          g.circle(f.x, f.y, f.size * (1 - k * 0.5)).fill({ color: f.color, alpha: a * 0.8 });
          break;
        case 'tracer':
          g.moveTo(f.x, f.y).lineTo(f.x2!, f.y2!).stroke({ width: 1.2, color: f.color, alpha: a * 0.9 });
          break;
        case 'flak':
          for (let i = 0; i < 5; i++) {
            const ang = i * 1.3 + f.size;
            g.circle(f.x + Math.cos(ang) * 6 * (0.5 + k), f.y + Math.sin(ang) * 6 * (0.5 + k), 1.6 + k * 2).fill({ color: f.color, alpha: a * 0.7 });
          }
          if (f.x2 !== undefined) g.moveTo(f.x2, f.y2!).lineTo(f.x, f.y).stroke({ width: 0.8, color: f.color, alpha: a * 0.35 });
          break;
        case 'chute': {
          // A little parachute drifting down: everyone ejects.
          const y = f.y + k * 18;
          const x = f.x + Math.sin(k * 6) * 3;
          g.moveTo(x - 5, y - 6).arc(x, y - 6, 5, Math.PI, 0).closePath().fill({ color: 0xffffff, alpha: a * 0.9 });
          g.moveTo(x - 5, y - 6).lineTo(x, y).moveTo(x + 5, y - 6).lineTo(x, y).stroke({ width: 0.7, color: 0xffffff, alpha: a * 0.9 });
          g.circle(x, y + 1, 1.2).fill({ color: f.color, alpha: a });
          break;
        }
        case 'smoke':
          g.circle(f.x + k * 6, f.y - k * 12, f.size * (0.5 + k)).fill({ color: 0x9aa4b5, alpha: a * 0.3 });
          break;
        case 'static':
          for (let i = 0; i < 12; i++) {
            g.rect(f.x + (Math.random() - 0.5) * 30, f.y + (Math.random() - 0.5) * 30, 3, 1.5).fill({ color: 0xdde6f5, alpha: a * Math.random() });
          }
          break;
        case 'arc': {
          const mx = (f.x + f.x2!) / 2;
          const my = Math.min(f.y, f.y2!) - 120;
          const p = Math.min(1, k * 1.6);
          let px = f.x;
          let py = f.y;
          g.moveTo(px, py);
          for (let i = 1; i <= 24 * p; i++) {
            const s = i / 24;
            px = (1 - s) * (1 - s) * f.x + 2 * (1 - s) * s * mx + s * s * f.x2!;
            py = (1 - s) * (1 - s) * f.y + 2 * (1 - s) * s * my + s * s * f.y2!;
            g.lineTo(px, py);
          }
          g.stroke({ width: 2.5, color: f.color, alpha: a });
          break;
        }
        case 'text':
          break;
      }
    }
    this.fx = keep;
  }

  // --- Project Bark Park and the Big Fetch -------------------------------------------------------

  /** A fence around each faction at the park, and dogs at the park itself. */
  private drawPark() {
    const g = this.parkG;
    g.clear();
    if (!this.park.size) return;
    const pulse = 0.5 + 0.5 * Math.sin(this.now / 650);
    for (const seat of this.park) {
      const f = this.factions[seat];
      const zones = this.content.zones.filter((z) => z.region === f);
      for (const z of zones) g.poly(z.poly.flat()).fill({ color: 0x7fe08f, alpha: 0.05 + 0.06 * pulse });
      // Only the outer edges get a fence: count each edge across the region's zones.
      const edges = new Map<string, [P, P, number]>();
      for (const z of zones) {
        for (let i = 0; i < z.poly.length; i++) {
          const a = z.poly[i];
          const b = z.poly[(i + 1) % z.poly.length];
          const k = a[0] < b[0] || (a[0] === b[0] && a[1] < b[1]) ? `${a}|${b}` : `${b}|${a}`;
          const e = edges.get(k);
          edges.set(k, e ? [e[0], e[1], e[2] + 1] : [a, b, 1]);
        }
      }
      for (const [a, b, count] of edges.values()) {
        if (count > 1) continue;
        g.moveTo(a[0], a[1]).lineTo(b[0], b[1]).stroke({ width: 5, color: 0x7fe08f, alpha: 0.16 + 0.1 * pulse });
        g.moveTo(a[0], a[1]).lineTo(b[0], b[1]).stroke({ width: 1.4, color: 0xe9f5e1, alpha: 0.85 });
        const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
        for (let s = 0; s <= len; s += 9) {
          const k = len > 0 ? s / len : 0;
          g.circle(a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, 1.3).fill({ color: 0xffffff, alpha: 0.9 });
        }
      }
    }
    const bp = barkPark(this.content);
    g.circle(bp[0], bp[1], 16 + 3 * pulse).stroke({ width: 2, color: 0x7fe08f, alpha: 0.7 });
    let i = 0;
    for (const seat of this.park) {
      const ang = this.now / 900 + (i++ * Math.PI * 2) / this.park.size;
      g.circle(bp[0] + Math.cos(ang) * 11, bp[1] + Math.sin(ang) * 7, 3).fill({ color: factionColor(this.factions[seat]) });
    }
  }

  /** Radio from the park: the crews relaxing, and what the dog in charge is up to. */
  private parkRadio() {
    for (const l of this.parkLines) {
      if (l.done || l.tick > this.t) continue;
      l.done = true;
      if (this.quietNarration) continue;
      const f = this.factions[l.seat];
      const fd = faction(f);
      if (l.n % 2 === 0) {
        const line = chatter(f, 'park', {}, l.tick * 13 + l.seat);
        if (line) this.hooks.chatter(`${fd.short.toUpperCase()} AT THE PARK`, fd.color, line, 1);
      } else {
        const line = pick(this.content.flavor.park_curtain?.[f], l.tick * 7 + l.seat);
        if (line) this.hooks.chatter('PROJECT BARK PARK', '#7fe08f', line, 1);
      }
    }
  }

  /** The ball lifts off from the park three ticks before it lands. */
  private launchFetches() {
    for (const f of this.fetches) {
      if (f.launched || this.t < f.tick - 3 || this.t >= f.tick + 1) continue;
      f.launched = true;
      const short = faction(this.factions[f.seat]).short.toUpperCase();
      this.hooks.banner('THE BIG FETCH', `${short} LAUNCHES FROM PROJECT BARK PARK`, 'park');
      this.hooks.sound('fetch');
      if (!this.quietNarration) this.say(f.seat, `${short} LAUNCH CREW`, 'fetch', { node: nodeName(f.node) }, f.tick + f.seat, 4);
      const bp = barkPark(this.content);
      this.addFx({ kind: 'ring', x: bp[0], y: bp[1], color: 0xd7f542, dur: 1400, size: 26 });
    }
  }

  /** A stadium-sized tennis ball on an arc, with its shadow racing along the ground. */
  private drawBalls() {
    const g = this.ballG;
    g.clear();
    const t = this.t;
    for (const f of this.fetches) {
      const a = barkPark(this.content);
      const b = this.content.nodes[f.node].pos;
      if (t >= f.tick - 3 && t < f.tick) {
        const k = (t - (f.tick - 3)) / 3;
        const d = Math.hypot(b[0] - a[0], b[1] - a[1]);
        const c: P = [(a[0] + b[0]) / 2, Math.min(a[1], b[1]) - d * 0.45 - 40];
        const x = (1 - k) * (1 - k) * a[0] + 2 * (1 - k) * k * c[0] + k * k * b[0];
        const y = (1 - k) * (1 - k) * a[1] + 2 * (1 - k) * k * c[1] + k * k * b[1];
        const gx = a[0] + (b[0] - a[0]) * k;
        const gy = a[1] + (b[1] - a[1]) * k;
        const lift = Math.sin(Math.PI * k);
        g.ellipse(gx, gy, 9 - 4 * lift, 4 - 2 * lift).fill({ color: 0x000000, alpha: 0.35 });
        this.ball(g, x, y, 7 + 9 * lift, k * 9);
      } else if (t >= f.tick && t < f.tick + 8) {
        // It bounces away, as balls do.
        const s = t - f.tick;
        const fade = 1 - s / 8;
        const x = b[0] + s * 5;
        const y = b[1] - Math.abs(Math.sin(s * 1.7)) * 10 * fade;
        g.ellipse(x, b[1] + 2, 6, 2.5).fill({ color: 0x000000, alpha: 0.25 * fade });
        this.ball(g, x, y, 7, s * 3, fade);
      }
    }
  }

  private ball(g: Graphics, x: number, y: number, r: number, spin: number, alpha = 1) {
    g.circle(x, y, r).fill({ color: 0xd7f542, alpha });
    g.circle(x, y, r).stroke({ width: Math.max(0.8, r * 0.08), color: 0x9fb526, alpha });
    g.moveTo(x + Math.cos(spin) * r, y + Math.sin(spin) * r)
      .quadraticCurveTo(x + Math.cos(spin + Math.PI / 2) * r * 0.2, y + Math.sin(spin + Math.PI / 2) * r * 0.2, x + Math.cos(spin + Math.PI) * r, y + Math.sin(spin + Math.PI) * r)
      .stroke({ width: Math.max(0.8, r * 0.12), color: 0xffffff, alpha: 0.9 * alpha });
  }

  /** A cartoon "BOING!" over a node. */
  private boing(p: P) {
    const id = `boing-${this.boings++}`;
    this.map.label(id, 'BOING!', [p[0], p[1] - 16], 'label-boing');
    setTimeout(() => this.map.removeLabel(id), 1500);
  }

  // --- Events ---------------------------------------------------------------------------------

  private callsign(s: SqSnap): string {
    if (s.ace) return this.content.factions.find((f) => f.id === this.factions[s.seat])!.ace;
    const words: Record<string, string> = {
      Fighter: 'FLIGHT', Bomber: 'HEAVY', Scout: 'EYEBALL', Jammer: 'HUSH', Drone: 'UNICORN', Floatplane: 'PONTOON',
      Watchtower: 'WATCHTOWER', Longhorn: 'LONGHORN', Prototype: 'SAUCER', CropDuster: 'DUSTER', Gull: 'GULL',
    };
    return `${words[s.kind] ?? 'FLIGHT'} ${(s.sq % 9) + 1}`;
  }

  private say(seat: number, who: string, key: string, params: Record<string, string>, salt: number, priority: number) {
    if (this.quietNarration) return;
    const f = this.factions[seat];
    const line = chatter(f, key, params, salt);
    if (line) this.hooks.chatter(who, faction(f).color, line, priority);
  }

  private sayAce(seat: number, key: 'launch' | 'kill' | 'duel' | 'downed' | 'returns', salt: number, priority: number) {
    if (this.quietNarration) return;
    const f = this.factions[seat];
    const line = aceLine(f, key, salt);
    if (line) this.hooks.chatter(this.content.factions.find((x) => x.id === f)!.ace, faction(f).color, line, priority);
  }

  private snapFromGroup(gid: number): SqSnap | null {
    const g = this.groups.get(gid);
    if (!g) return null;
    return { sq: gid, seat: g.seat, kind: g.kinds[0], strength: 4, ace: g.ace, stars: 0 };
  }

  private fire(r: EventRec, silent: boolean) {
    const e: GameEvent = r.e;
    const salt = eventSalt(e, r.tick);
    const enemyName = (seat: number) => faction(this.factions[seat]).short;
    if (silent) {
      if (e.t === 'Score') this.hooks.score(e.seat, e.delta);
      return;
    }
    switch (e.t) {
      case 'NightStart':
        break;
      case 'Launch': {
        const lead = e.squadrons[0];
        if (lead) {
          if (lead.ace && salt % 2 === 0) this.sayAce(e.seat, 'launch', salt, 1);
          else if (salt % 3 === 0) this.say(e.seat, this.callsign(lead), 'launch', {}, salt, 0);
        }
        const base = this.content.nodes[e.base].pos;
        this.addFx({ kind: 'ring', x: base[0], y: base[1], color: factionColor(this.factions[e.seat]), dur: 900, size: 10 });
        this.hooks.sound('launch');
        break;
      }
      case 'Engage': {
        const colorA = factionColor(this.factions[e.a_seat]);
        const colorB = factionColor(this.factions[e.b_seat]);
        this.addFx({ kind: 'ring', x: e.x, y: e.y, color: 0xffffff, dur: 1200, size: 22 });
        const shots = e.rounds.flat();
        let i = 0;
        for (const s of shots.slice(0, 18)) {
          const fromA = e.a.some((q) => q.sq === s.from);
          const ang = (i++ * 2.4) % (Math.PI * 2);
          const r1 = 10;
          this.addFx({
            kind: 'tracer', x: e.x + Math.cos(ang) * r1, y: e.y + Math.sin(ang) * r1,
            x2: e.x - Math.cos(ang) * r1 * 0.4, y2: e.y - Math.sin(ang) * r1 * 0.4,
            color: fromA ? colorA : colorB, dur: 500 + i * 60, size: 1,
          });
          if (s.hit) this.addFx({ kind: 'flash', x: e.x + Math.cos(ang) * 4, y: e.y + Math.sin(ang) * 4, color: 0xffd36b, dur: 400 + i * 50, size: 3 });
        }
        this.hooks.sound(e.ace_duel ? 'duel' : 'guns');
        const a0 = e.a[0];
        const b0 = e.b[0];
        if (e.ace_duel) {
          this.hooks.banner('ACE DUEL', `${this.callsign(e.a.find((s) => s.ace)!)} vs ${this.callsign(e.b.find((s) => s.ace)!)}`, 'update');
          this.sayAce(e.a_seat, 'duel', salt, 3);
        } else if (a0 && b0) {
          const zone = this.zoneOf([e.x, e.y]);
          if (a0.ace && salt % 2 === 0) this.sayAce(e.a_seat, 'duel', salt, 2);
          else this.say(e.a_seat, this.callsign(a0), 'engage', { enemy: enemyName(e.b_seat), zone }, salt, 2);
          // Gulls, watchtowers and other uniques get their moment.
          const unique = this.content.factions.find((f) => f.id === this.factions[e.a_seat])!.unique;
          if (e.a.some((s) => s.kind === unique) && salt % 2 === 1) this.say(e.a_seat, this.callsign(e.a.find((s) => s.kind === unique)!), 'unique', {}, salt + 1, 1);
        }
        if (e.first_shot !== null && e.first_shot !== undefined) {
          const fs = e.first_shot;
          this.addFx({ kind: 'flash', x: e.x, y: e.y, color: factionColor(this.factions[fs]), dur: 500, size: 8 });
        }
        break;
      }
      case 'Flak': {
        const site = this.content.nodes[e.site].pos;
        const color = factionColor(this.factions[e.site_seat]);
        e.shots.forEach((s, i) => {
          this.addFx({ kind: 'flak', x: e.x + (i - 1) * 4, y: e.y - 3 + i * 2, x2: site[0], y2: site[1], color: s.hit ? 0xffb347 : color, dur: 800 + i * 150, size: i });
        });
        const g = this.snapFromGroup(e.group);
        if (g && salt % 2 === 0) this.say(e.seat, this.callsign(g), 'flak', {}, salt, 1);
        this.hooks.sound('flak');
        break;
      }
      case 'Splash': {
        const color = factionColor(this.factions[e.seat]);
        if (e.ground) {
          this.addFx({ kind: 'boom', x: e.x, y: e.y, color, dur: 1200, size: 8 });
        } else {
          this.addFx({ kind: 'flash', x: e.x, y: e.y, color: 0xffffff, dur: 600, size: 6 });
          this.addFx({ kind: 'ring', x: e.x, y: e.y, color, dur: 1000, size: 12 });
        }
        this.addFx({ kind: 'chute', x: e.x + 4, y: e.y - 4, color, dur: 3200, size: 1 });
        if (e.ace) {
          const ace = this.content.factions.find((f) => f.id === this.factions[e.seat])!.ace;
          this.hooks.banner('MISSION UPDATE', `${ace} IS DOWN. GOOD CHUTE.`, 'update');
          if (e.by_seat !== null) this.hooks.sound(`motif:${this.factions[e.by_seat]}`);
          this.sayAce(e.seat, 'downed', salt, 4);
          this.hooks.sound('sting');
        } else {
          const me: SqSnap = { sq: e.sq, seat: e.seat, kind: e.kind, strength: 0, ace: false, stars: 0 };
          if (e.by_seat !== null && e.by_sq !== null && salt % 2 === 0) {
            const shooterAce = this.isAce(e.by_sq);
            if (shooterAce) this.sayAce(e.by_seat, 'kill', salt, 3);
            else this.say(e.by_seat, `${faction(this.factions[e.by_seat]).short.toUpperCase()}`, 'splash', { enemy: enemyName(e.seat) }, salt, 2);
          } else {
            this.say(e.seat, this.callsign(me), 'ejected', {}, salt, 2);
          }
        }
        this.hooks.sound('splash');
        break;
      }
      case 'Strike': {
        const n = this.content.nodes[e.node];
        const color = factionColor(this.factions[e.seat]);
        this.addFx({ kind: 'boom', x: n.pos[0], y: n.pos[1], color: e.steps > 0 ? 0xff8a3d : color, dur: e.jewel ? 2600 : 1600, size: e.jewel ? 26 : 12 + e.steps * 6 });
        if (e.steps > 0) {
          for (let i = 0; i < 4; i++) this.addFx({ kind: 'smoke', x: n.pos[0] + (i - 2) * 3, y: n.pos[1], color: 0x999999, dur: 4000 + i * 600, size: 6 });
          this.nodes[e.node] = { ...this.nodes[e.node], health: e.health };
          this.map.setNodes(this.nodes.map((x) => ({ ...x })));
        }
        if (e.jewel && e.steps > 0) {
          this.map.shake = 1.6;
          this.hooks.banner('MISSION UPDATE', `CROWN JEWEL HIT: ${n.name.toUpperCase()}`, 'update');
          this.hooks.sound(`motif:${this.factions[e.seat]}`);
          this.say(e.seat, faction(this.factions[e.seat]).short.toUpperCase(), 'jewel_strike', { node: n.name }, salt, 5);
          this.hooks.sound('sting');
          this.hooks.sound('bigboom');
        } else if (e.steps > 0) {
          const unique = this.content.factions.find((f) => f.id === this.factions[e.seat])!.unique;
          if (e.kinds.includes(unique) && salt % 2 === 0) this.say(e.seat, unitName(unique).toUpperCase(), 'unique', {}, salt, 2);
          else this.say(e.seat, faction(this.factions[e.seat]).short.toUpperCase(), 'strike', { node: n.name }, salt, 2);
          this.map.shake = Math.max(this.map.shake, 0.35);
          this.hooks.sound('boom');
        } else {
          this.say(e.seat, faction(this.factions[e.seat]).short.toUpperCase(), 'miss', {}, salt, 1);
          this.hooks.sound('thud');
        }
        break;
      }
      case 'MassDriver':
      case 'Revenge': {
        const n = this.content.nodes[e.node];
        const from = e.t === 'MassDriver' ? this.content.nodes.find((x) => x.kind === 'MassDriver')!.pos : this.content.nodes.find((x) => x.kind === 'CrownJewel' && x.home === 'GrainBelt')!.pos;
        this.addFx({ kind: 'arc', x: from[0], y: from[1], x2: n.pos[0], y2: n.pos[1], color: 0xfff1a8, dur: 1800, size: 1 });
        this.addFx({ kind: 'boom', x: n.pos[0], y: n.pos[1], color: 0xffd36b, dur: 2400, size: 24 });
        this.map.shake = 1.2;
        this.hooks.banner(e.t === 'MassDriver' ? 'MASS DRIVER FIRED' : 'REVENGE STRIKE', `${faction(this.factions[e.seat]).short.toUpperCase()} HITS ${n.name.toUpperCase()}`, 'update');
        this.nodes[e.node] = { ...this.nodes[e.node], health: e.health };
        this.map.setNodes(this.nodes.map((x) => ({ ...x })));
        this.hooks.sound('bigboom');
        break;
      }
      case 'Smoke': {
        const n = this.content.nodes[e.node];
        for (let i = 0; i < 6; i++) this.addFx({ kind: 'smoke', x: n.pos[0] + (i - 3) * 4, y: n.pos[1] + 4, color: 0xaaaaaa, dur: 5000, size: 8 });
        this.say(e.seat, 'DUSTER', 'unique', {}, salt, 1);
        break;
      }
      case 'Pivot':
        this.sayAce(e.seat, 'launch', salt, 3);
        this.hooks.banner('PIVOT', `PIVOTS TO STRIKE ${nodeName(e.node).toUpperCase()}`, 'update');
        break;
      case 'Vanished':
        this.addFx({ kind: 'static', x: e.x, y: e.y, color: 0xffffff, dur: 1600, size: 1 });
        this.hooks.banner('SIGNAL LOST', "THE PROTOTYPE DIDN'T COME BACK. NOBODY ASKS.", 'static');
        break;
      case 'LostInStatic':
        this.addFx({ kind: 'static', x: e.x, y: e.y, color: 0xffffff, dur: 2000, size: 1 });
        this.hooks.banner('LOST IN THE STATIC', 'EAST OF THE MISSISSIPPI, NOBODY HEARS YOU', 'static');
        this.hooks.sound('static');
        break;
      case 'BetWin':
        this.hooks.banner('JACKPOT', `DREAMLAND'S BET PAYS ${e.payout} SUPPLY`, 'jackpot');
        this.hooks.sound('coin');
        break;
      case 'Jiggle':
        this.hooks.banner('LIME JELLO', 'WIGGLES BUT DOES NOT BREAK', 'update');
        break;
      case 'Score':
        this.hooks.score(e.seat, e.delta);
        break;
      case 'Capture': {
        const n = this.content.nodes[e.node];
        this.nodes[e.node] = { ...this.nodes[e.node], owner: e.seat };
        this.map.setNodes(this.nodes.map((x) => ({ ...x })));
        this.addFx({ kind: 'ring', x: n.pos[0], y: n.pos[1], color: factionColor(this.factions[e.seat]), dur: 1600, size: 18 });
        this.say(e.seat, faction(this.factions[e.seat]).short.toUpperCase(), 'capture', { node: n.name }, salt, 1);
        break;
      }
      case 'AceReturns':
        this.sayAce(e.seat, 'returns', salt, 2);
        break;
      case 'BarkPark': {
        const f = this.factions[e.seat];
        this.hooks.banner('PROJECT BARK PARK', `${faction(f).short.toUpperCase()} SITS THIS ONE OUT. UNTOUCHABLE TILL DAWN.`, 'park');
        const line = pick(leader(f).retreat, salt);
        if (line && !this.quietNarration) this.hooks.chatter(leader(f).name, faction(f).color, line, 4);
        this.hooks.sound('park');
        break;
      }
      case 'Bounce': {
        const n = this.content.nodes[e.node];
        this.addFx({ kind: 'ring', x: n.pos[0], y: n.pos[1], color: 0x7fe08f, dur: 1300, size: 26 });
        this.addFx({ kind: 'flash', x: n.pos[0], y: n.pos[1] - 6, color: 0xffffff, dur: 500, size: 7 });
        this.boing(n.pos);
        if (e.group !== null) this.say(e.seat, faction(this.factions[e.seat]).short.toUpperCase(), 'bounce', { enemy: enemyName(e.victim), node: n.name }, salt, 3);
        this.hooks.sound('boing');
        break;
      }
      case 'Fetch': {
        const n = this.content.nodes[e.node];
        const jewel = n.kind === 'CrownJewel';
        this.addFx({ kind: 'boom', x: n.pos[0], y: n.pos[1], color: 0xd7f542, dur: 2600, size: jewel ? 30 : 24 });
        this.addFx({ kind: 'ring', x: n.pos[0], y: n.pos[1], color: 0xffffff, dur: 1500, size: 34 });
        for (let i = 0; i < 5; i++) this.addFx({ kind: 'smoke', x: n.pos[0] + (i - 2) * 4, y: n.pos[1], color: 0x999999, dur: 4200 + i * 500, size: 7 });
        this.map.shake = jewel ? 1.8 : 1.3;
        this.nodes[e.node] = { ...this.nodes[e.node], health: e.health };
        this.map.setNodes(this.nodes.map((x) => ({ ...x })));
        const f = this.factions[e.seat];
        const victim = e.victim !== null ? faction(this.factions[e.victim]).short.toUpperCase() : '';
        this.hooks.banner('SQUEAK-BOOM!', `THE BIG FETCH HITS ${n.name.toUpperCase()}${victim ? ` (${victim})` : ''}`, 'park');
        const line = pick(leader(f).fetch, salt);
        if (line && !this.quietNarration) this.hooks.chatter(leader(f).name, faction(f).color, line, 5);
        this.hooks.sound('squeak');
        this.hooks.sound('bigboom');
        this.hooks.sound(`motif:${f}`);
        break;
      }
      case 'ParkReturn':
        break;
      case 'JewelEffect':
      case 'Abort':
      case 'Casserole':
      case 'Dawn':
        break;
      default:
        break;
    }
  }

  private isAce(sq: number): boolean {
    for (const r of this.events) {
      if (r.e.t === 'Launch' && r.e.squadrons.some((s) => s.sq === sq && s.ace)) return true;
      if (r.e.t === 'Engage' && [...r.e.a, ...r.e.b].some((s) => s.sq === sq && s.ace)) return true;
    }
    return false;
  }

  private zoneOf(p: P): string {
    let best = 0;
    let bd = Infinity;
    for (const z of this.content.zones) {
      const d = (z.center[0] - p[0]) ** 2 + (z.center[1] - p[1]) ** 2;
      if (d < bd) {
        bd = d;
        best = z.idx;
      }
    }
    return zoneName(best);
  }
}
