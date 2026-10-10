// The planning window: private and fast. Drag a squadron onto a target, zone
// or friendly squadron; its route draws itself and gets a risk color. Queue
// builds and repairs, use faction actions, then Commit.

import { Graphics } from 'pixi.js';
import { MapView } from '../map/mapview';
import type { Engine } from '../engine';
import type {
  FactionId, Mission, NodeKind, Orders, P, Preview, SeatKind, SeatView, Squadron, TargetInfo, UnitKind, Warning, WeakSpot,
} from '../types';
import { drawGlyph, factionColor } from '../theme';
import {
  advisorLine, effectText, esc, faction, KIND_LABEL, leader, missionLabel, nodeName, patchText, stageBlurb, stageName,
  turns, unitDef, unitName, zoneName,
} from '../text';
import { leaderSvg, emblemSvg } from '../art/dogs';
import { glyphSvg } from './glyph';
import { h, clear, svg, modal, toast } from './dom';
import { rulesModal } from './rules';
import { audio } from '../audio';

export interface Coach {
  step(screen: PlanningScreen): void;
}

export interface PlanningOpts {
  engine: Engine;
  state: string;
  seat: number;
  factions: FactionId[];
  kinds: SeatKind[];
  initial: Orders;
  timer: number;
  onCommit(o: Orders): void;
  coach?: Coach | null;
}

interface Option {
  label: string;
  mission: Mission | null;
  reason?: string | null;
  sq: number;
}

const RISK_COLOR: Record<string, number> = { Green: 0x3ddc84, Yellow: 0xffd23f, Red: 0xff4d4d };
const RISK_CSS: Record<string, string> = { Green: '#3ddc84', Yellow: '#ffd23f', Red: '#ff4d4d' };

export class PlanningScreen {
  readonly el = h('div.plan');
  readonly opts: PlanningOpts;
  readonly engine: Engine;
  view: SeatView;
  orders: Orders;
  previews = new Map<number, Preview>();
  warnings: Warning[] = [];
  selected: number | null = null;
  map!: MapView;
  showWeak = false;
  showTargets = false;
  weak: WeakSpot | null = null;
  enemyWeak: WeakSpot | null = null;
  targets: TargetInfo[] = [];
  private top = h('header.plan-top');
  private mapWrap = h('div.plan-mapwrap');
  private side = h('aside.plan-side');
  private tray = h('div.plan-tray');
  private tip = h('div.plan-tip');
  private popup: HTMLElement | null = null;
  private hint = h('div.plan-hint');
  private routeG = new Graphics();
  private unitG = new Graphics();
  private markG = new Graphics();
  private dragWaypoint: { sq: number; p: P } | null = null;
  private down: { x: number; y: number; moved: boolean; pan: boolean } | null = null;
  private cardDrag: { sq: number; ghost: HTMLElement; x: number; y: number; moved: boolean } | null = null;
  private deadline: number | null = null;
  private timerEl = h('div.plan-timer');
  private committed = false;
  private sideTab: 'advisor' | 'build' | 'faction' | 'intel' = 'advisor';
  private glyphSpots = new Map<number, P>();
  private destroyed = false;
  /** Tonight's orders from before a trip to the park, in case it's called off. */
  private parkSaved: Orders['missions'] | null = null;

  constructor(root: HTMLElement, opts: PlanningOpts) {
    this.opts = opts;
    this.engine = opts.engine;
    this.view = this.engine.seatView(opts.state, opts.seat);
    this.orders = normalize(opts.initial, this.view);
    const fd = faction(this.view.faction);
    this.el.style.setProperty('--fc', fd.color);
    this.el.append(this.top, h('div.plan-main', this.mapWrap, this.side), this.tray);
    this.mapWrap.append(this.tip, this.hint);
    root.appendChild(this.el);
    if (opts.timer > 0) this.deadline = Date.now() + opts.timer * 1000;
  }

  async init() {
    this.map = new MapView(this.mapWrap, this.engine.content);
    await this.map.init();
    this.map.setSeats(this.opts.factions);
    this.map.layers.overlay.addChild(this.markG, this.routeG);
    this.map.layers.units.addChild(this.unitG);
    this.map.fit();
    this.focusHome();
    this.bindMap();
    this.refresh();
    this.map.onTick(() => this.tickTimer());
    this.opts.coach?.step(this);
  }

  destroy() {
    this.destroyed = true;
    this.popup?.remove();
    this.map?.destroy();
    this.el.remove();
  }

  private focusHome() {
    const cap = this.engine.content.nodes.find((n) => n.kind === 'Capital' && n.home === this.view.faction);
    if (cap) {
      this.map.cam = { x: (cap.pos[0] + this.engine.content.width / 2) / 2, y: (cap.pos[1] + this.engine.content.height / 2) / 2, zoom: 1.25 };
      this.map.markLabelsDirty();
    }
  }

  // --- Data ------------------------------------------------------------------------------------------

  squad(id: number): Squadron | undefined {
    return this.view.squadrons.find((s) => s.id === id);
  }

  label(s: Squadron): string {
    if (s.ace) return faction(this.view.faction).ace;
    return `${unitName(s.kind)} #${s.id}`;
  }

  mission(id: number): Mission {
    return this.orders.missions.find((m) => m.sq === id)?.mission ?? { t: 'Rest' };
  }

  setMission(sq: number, m: Mission, waypoint: P | null = null) {
    if (this.orders.retreat && m.t !== 'Rest') {
      toast("Not tonight: everyone's at Project Bark Park. Call off the trip in the Faction tab to fly.");
      return;
    }
    const i = this.orders.missions.findIndex((x) => x.sq === sq);
    const entry = { sq, mission: m, waypoint };
    if (i >= 0) this.orders.missions[i] = entry;
    else this.orders.missions.push(entry);
    // Escorts of a squadron that now stays home have nothing to fly with.
    if (m.t === 'Rest') {
      for (const o of this.orders.missions) {
        if ((o.mission.t === 'Escort' || o.mission.t === 'JamEscort') && o.mission.sq === sq) o.mission = { t: 'Rest' };
      }
    }
    audio.play('click');
    this.refresh();
    this.opts.coach?.step(this);
  }

  refresh() {
    if (this.destroyed) return;
    const st = this.opts.state;
    this.previews = new Map(this.engine.previews(st, this.orders).map((p) => [p.sq, p]));
    this.warnings = this.engine.advise(st, this.opts.seat, this.orders);
    if (this.showWeak) this.weak = this.engine.weakSpot(st, this.opts.seat, this.opts.seat);
    if (this.showTargets) this.targets = this.engine.bestTargets(st, this.opts.seat);
    this.renderTop();
    this.renderTray();
    this.renderSide();
    this.drawOverlay();
  }

  fuelPlanned(): number {
    let f = 0;
    for (const p of this.previews.values()) f += p.fuel;
    return f;
  }

  supplyPlanned(): number {
    const c = this.engine.content;
    let s = this.orders.builds.reduce((a, k) => a + (c.units.find((u) => u.kind === k)?.cost ?? 0), 0);
    s += this.orders.repairs.length * (c.rules.economy.repair_cost ?? 3);
    s += Math.max(0, this.orders.harvest_bid);
    s += this.orders.bet?.stake ?? 0;
    return s;
  }

  // --- Header -------------------------------------------------------------------------------------------

  private renderTop() {
    const v = this.view;
    const fd = faction(v.faction);
    clear(this.top);
    const fuelNeed = this.fuelPlanned();
    const supplyNeed = this.supplyPlanned();
    const btn = (label: string, on: () => void, active = false, title = '') => h(`button.pbtn${active ? '.on' : ''}`, { onclick: on, title }, label);
    this.top.append(
      svg(emblemSvg(v.faction, { size: 40 }), 'plan-emblem'),
      h('div.plan-who', h('div.plan-fname', fd.name), h('div.plan-leader', `${fd.leader} · ${this.opts.kinds.filter((k) => k === 'human').length > 1 ? 'Planning in secret' : 'Your move'}`)),
      h('div.plan-night', h('div.big', `Night ${v.night}/${v.nights}`), h('div.small', stageName(v.stage))),
      h(
        'div.plan-res',
        { title: `Supply builds and repairs. Income ${v.income_supply}/night, stockpile cap ${v.cap_supply}.` },
        h('div.small', 'SUPPLY'),
        h('div.big', `${v.supply - supplyNeed}`, h('span.dim', ` / ${v.supply}`)),
        h('div.small', `+${v.income_supply} at dawn`),
      ),
      h(
        'div.plan-res' + (fuelNeed > v.fuel ? '.bad' : ''),
        { title: `Fuel flies missions. Income ${v.income_fuel}/night, stockpile cap ${v.cap_fuel}.` },
        h('div.small', 'FUEL'),
        h('div.big', `${v.fuel - fuelNeed}`, h('span.dim', ` / ${v.fuel}`)),
        h('div.small', `+${v.income_fuel} at dawn`),
      ),
      h('div.plan-res', h('div.small', 'BRAGGING RIGHTS'), h('div.big', String(v.seats[v.seat].score))),
      this.parkBadge() ?? "",
      this.timerEl,
      h(
        'div.plan-btns',
        btn('Suggest Plan', () => this.suggest(), false, 'Fill in a complete, decent plan to tweak'),
        btn('Weak Spot', () => this.toggleWeak(), this.showWeak, 'What an enemy should hit to hurt you most'),
        btn('Best Targets', () => this.toggleTargets(), this.showTargets, 'The most valuable targets you can see'),
        btn('Rules', () => rulesModal(this.view.faction)),
        btn('Reset', () => this.reset(), false, "Go back to last night's orders"),
      ),
    );
  }

  /** A nudge in the header: the doghouse, a night at the park, or the Big Fetch. */
  private parkBadge(): HTMLElement | null {
    const me = this.view.seats[this.view.seat];
    const go = () => { this.sideTab = 'faction'; this.renderSide(); };
    if (this.orders.retreat) return h('button.park-badge.on', { onclick: go, title: 'Your squadrons rest tonight and your land cannot be touched.' }, '🎾 At the Bark Park tonight');
    if (me.doghouse) return h('button.park-badge', { onclick: go, title: 'Once a game: a night off, untouchable, then back with the Big Fetch.' }, '🐾 In the doghouse: Bark Park open');
    if (me.fetch) return h('button.park-badge', { onclick: go, title: 'One stadium-sized tennis ball, any target you could attack tonight.' }, this.orders.fetch === null ? '🎾 Big Fetch loaded' : `🎾 Big Fetch → ${nodeName(this.orders.fetch)}`);
    return null;
  }

  private tickTimer() {
    if (!this.deadline || this.committed) return;
    const left = Math.max(0, Math.ceil((this.deadline - Date.now()) / 1000));
    this.timerEl.textContent = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`;
    this.timerEl.classList.toggle('urgent', left <= 15);
    if (left <= 0) {
      toast("Time's up: orders locked as they stand.");
      this.commit(true);
    }
  }

  // --- Squadron tray ----------------------------------------------------------------------------------

  private renderTray() {
    clear(this.tray);
    this.tray.classList.toggle('park', this.orders.retreat);
    const list = h('div.tray-list');
    if (this.orders.retreat) {
      list.appendChild(
        h(
          'div.park-note',
          h('span', h('b', '🎾 Everyone is at Project Bark Park tonight. '), 'Squadrons rest at home and nothing can touch your land.'),
          h('button.pbtn', { onclick: () => this.setRetreat(false) }, 'Call off the trip'),
        ),
      );
    }
    const sqs = [...this.view.squadrons].sort((a, b) => Number(b.ace) - Number(a.ace) || a.kind.localeCompare(b.kind) || a.id - b.id);
    for (const s of sqs) list.appendChild(this.card(s));
    const commitBtn = h('button.commit', { onclick: () => this.commit(false) }, 'Commit orders');
    this.tray.append(list, commitBtn);
  }

  private card(s: Squadron): HTMLElement {
    const v = this.view;
    const down = s.down_until > 0;
    const m = this.mission(s.id);
    const p = this.previews.get(s.id);
    const pips = h('span.pips', ...[0, 1, 2, 3].map((i) => h(i < s.strength ? 'i.on' : 'i')));
    const stars = s.stars ? h('span.stars', '★'.repeat(s.stars)) : null;
    const risk = p ? h('span.risk', { style: { background: RISK_CSS[p.color] }, title: `${p.color}: ${p.reason}` }) : null;
    const squads = v.squadrons.map((q) => ({ id: q.id, label: this.label(q) }));
    const atPark = this.orders.retreat && !down;
    const orderText = down ? `Back on Night ${s.down_until}` : atPark ? 'At the Bark Park' : missionLabel(m, squads);
    const el = h(
      `div.sq-card${this.selected === s.id ? '.sel' : ''}${down ? '.down' : ''}${p ? '.' + p.color.toLowerCase() : ''}`,
      { 'data-sq': String(s.id), title: unitDef(s.kind).desc },
      svg(glyphSvg(v.faction, s.kind, 30), 'sq-glyph'),
      h(
        'div.sq-body',
        h('div.sq-name', s.ace ? h('b', `${this.label(s)} `) : this.label(s), s.ace ? h('span.ace', 'ACE') : null, stars),
        h('div.sq-meta', pips, h('span.base', `@ ${nodeName(s.base)}`)),
        h('div.sq-order', risk, orderText, p ? h('span.fuel', ` · ${p.fuel} fuel`) : null),
        p && this.selected === s.id ? h('div.sq-reason', `${p.color}: ${p.reason}.`) : null,
      ),
      !down && !atPark && m.t !== 'Rest' ? h('button.sq-x', { title: 'Stay home and rest', onclick: (e: Event) => { e.stopPropagation(); this.setMission(s.id, { t: 'Rest' }); } }, '⌂') : null,
      !down && !atPark ? h('button.sq-more', { title: 'More orders', onclick: (e: Event) => { e.stopPropagation(); this.cardMenu(s, e as MouseEvent); } }, '⋯') : null,
    );
    if (!down) {
      el.addEventListener('pointerdown', (e) => this.cardPointerDown(s.id, e));
    }
    return el;
  }

  private cardMenu(s: Squadron, e: MouseEvent) {
    const opts: Option[] = [{ label: 'Rest at base (repairs faster)', mission: { t: 'Rest' }, sq: s.id }];
    const u = unitDef(s.kind);
    for (const o of this.view.squadrons) {
      if (o.id === s.id) continue;
      const om = this.mission(o.id);
      if (!['Attack', 'Scout', 'Rebase'].includes(om.t)) continue;
      if (u.orders.includes('Escort')) opts.push({ label: `Escort ${this.label(o)} (${missionLabel(om)})`, mission: { t: 'Escort', sq: o.id }, sq: s.id });
      if (u.orders.includes('Jam')) opts.push({ label: `Jam for ${this.label(o)} (${missionLabel(om)})`, mission: { t: 'JamEscort', sq: o.id }, sq: s.id });
    }
    if (opts.length === 1 && !u.orders.includes('Escort') && !u.orders.includes('Jam')) {
      opts.push({ label: 'Drag this card onto the map to give an order', mission: null, reason: 'Tip', sq: s.id });
    } else if (opts.length === 1) {
      opts.push({ label: 'Give a bomber an Attack order first, then fly with it', mission: null, reason: 'Tip', sq: s.id });
    }
    this.showPopup(opts, e.clientX, e.clientY);
  }

  private cardPointerDown(sq: number, e: PointerEvent) {
    if ((e.target as HTMLElement).closest('button')) return;
    if (this.orders.retreat) {
      toast("Not tonight: everyone's at Project Bark Park. Call off the trip to fly.");
      return;
    }
    e.preventDefault();
    const ghost = h('div.drag-ghost', svg(glyphSvg(this.view.faction, this.squad(sq)!.kind, 34)));
    this.cardDrag = { sq, ghost, x: e.clientX, y: e.clientY, moved: false };
    const move = (ev: PointerEvent) => {
      const d = this.cardDrag;
      if (!d) return;
      if (!d.moved && Math.hypot(ev.clientX - d.x, ev.clientY - d.y) > 6) {
        d.moved = true;
        document.body.appendChild(d.ghost);
        this.select(sq);
      }
      if (d.moved) {
        d.ghost.style.transform = `translate(${ev.clientX - 17}px, ${ev.clientY - 17}px)`;
        const r = this.mapWrap.getBoundingClientRect();
        if (ev.clientX >= r.left && ev.clientX <= r.right && ev.clientY >= r.top && ev.clientY <= r.bottom) {
          this.hover(ev.clientX - r.left, ev.clientY - r.top);
        }
      }
    };
    const up = (ev: PointerEvent) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      const d = this.cardDrag;
      this.cardDrag = null;
      if (!d) return;
      d.ghost.remove();
      if (!d.moved) {
        this.select(this.selected === sq ? null : sq);
        return;
      }
      const r = this.mapWrap.getBoundingClientRect();
      if (ev.clientX >= r.left && ev.clientX <= r.right && ev.clientY >= r.top && ev.clientY <= r.bottom) {
        this.mapAction(sq, ev.clientX - r.left, ev.clientY - r.top, ev.clientX, ev.clientY);
      } else {
        const over = document.elementFromPoint(ev.clientX, ev.clientY)?.closest('.sq-card') as HTMLElement | null;
        const target = over ? Number(over.dataset.sq) : NaN;
        if (Number.isFinite(target) && target !== sq) this.escortOptions(sq, target, ev.clientX, ev.clientY);
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  select(sq: number | null) {
    this.selected = sq;
    this.popup?.remove();
    this.renderTray();
    this.drawOverlay();
    if (sq !== null) {
      const s = this.squad(sq)!;
      const u = unitDef(s.kind);
      const verbs = u.orders.filter((o) => o !== 'Rest' && o !== 'Rebase').map((o) => o.toLowerCase());
      this.setHint(`${this.label(s)}: drop on a target to ${verbs.join(', ')}. Red rings can be attacked, green guarded, blue rebased.`);
    } else {
      this.setHint('');
    }
  }

  private setHint(t: string) {
    this.hint.textContent = t;
    this.hint.style.display = t ? 'block' : 'none';
  }

  // --- Map interaction -------------------------------------------------------------------------------

  private bindMap() {
    const canvas = this.map.app.canvas;
    canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const r = canvas.getBoundingClientRect();
      this.map.zoomAt(e.clientX - r.left, e.clientY - r.top, e.deltaY < 0 ? 1.15 : 1 / 1.15);
    }, { passive: false });
    const pointers = new Map<number, P>();
    let pinch: number | null = null;
    canvas.addEventListener('pointerdown', (e) => {
      const r = canvas.getBoundingClientRect();
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      pointers.set(e.pointerId, [x, y]);
      if (pointers.size === 2) {
        const [a, b] = [...pointers.values()];
        pinch = Math.hypot(a[0] - b[0], a[1] - b[1]);
        this.down = null;
        return;
      }
      canvas.setPointerCapture(e.pointerId);
      this.popup?.remove();
      // Grab the waypoint handle of the selected route?
      if (this.selected !== null) {
        const hp = this.handlePos(this.selected);
        if (hp) {
          const [hx, hy] = this.map.toScreen(hp);
          if (Math.hypot(hx - x, hy - y) < 14) {
            this.dragWaypoint = { sq: this.selected, p: hp };
            return;
          }
        }
      }
      this.down = { x, y, moved: false, pan: e.button !== 0 };
    });
    canvas.addEventListener('pointermove', (e) => {
      const r = canvas.getBoundingClientRect();
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      if (pointers.has(e.pointerId)) pointers.set(e.pointerId, [x, y]);
      if (pointers.size === 2 && pinch) {
        const [a, b] = [...pointers.values()];
        const d = Math.hypot(a[0] - b[0], a[1] - b[1]);
        this.map.zoomAt((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, d / pinch);
        pinch = d;
        return;
      }
      if (this.dragWaypoint) {
        this.dragWaypoint.p = this.map.toMap(x, y);
        this.drawOverlay();
        return;
      }
      if (this.down) {
        const dx = x - this.down.x;
        const dy = y - this.down.y;
        if (this.down.moved || Math.hypot(dx, dy) > 5) {
          this.down.moved = true;
          this.map.panBy(e.movementX, e.movementY);
          return;
        }
      }
      this.hover(x, y);
    });
    const end = (e: PointerEvent) => {
      pointers.delete(e.pointerId);
      if (pointers.size < 2) pinch = null;
      const r = canvas.getBoundingClientRect();
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      if (this.dragWaypoint) {
        const { sq, p } = this.dragWaypoint;
        this.dragWaypoint = null;
        const m = this.mission(sq);
        const wp: P = [Math.round(p[0]), Math.round(p[1])];
        this.setMission(sq, m, wp);
        if (this.engine.content.mississippi.length && this.previews.get(sq)?.in_static) {
          toast('That route crosses the Mississippi into the Static. Pilots who fly in there don\'t come back.');
        }
        return;
      }
      const d = this.down;
      this.down = null;
      if (!d || d.moved || e.type === 'pointercancel') return;
      if (this.selected !== null) this.mapAction(this.selected, x, y, e.clientX, e.clientY);
      else this.mapClick(x, y, e.clientX, e.clientY);
    };
    canvas.addEventListener('pointerup', end);
    canvas.addEventListener('pointercancel', end);
    canvas.addEventListener('pointerleave', () => (this.tip.style.display = 'none'));
    canvas.addEventListener('dblclick', (e) => {
      if (this.selected === null) return;
      const r = canvas.getBoundingClientRect();
      const hp = this.handlePos(this.selected);
      if (!hp) return;
      const [hx, hy] = this.map.toScreen(hp);
      if (Math.hypot(hx - (e.clientX - r.left), hy - (e.clientY - r.top)) < 14) {
        const sq = this.selected;
        this.setMission(sq, this.mission(sq), null);
        toast('Waypoint cleared: the route plans itself again.');
      }
    });
  }

  /** Own squadron glyph under the cursor. */
  private squadAt(x: number, y: number): number | null {
    let best: number | null = null;
    let bd = 12 * 12;
    for (const [sq, p] of this.glyphSpots) {
      const [sx, sy] = this.map.toScreen(p);
      const d = (sx - x) ** 2 + (sy - y) ** 2;
      if (d < bd) {
        bd = d;
        best = sq;
      }
    }
    return best;
  }

  private mapClick(x: number, y: number, cx: number, cy: number) {
    const sq = this.squadAt(x, y);
    if (sq !== null && this.squad(sq) && this.squad(sq)!.down_until === 0) {
      this.select(sq);
      return;
    }
    const node = this.map.nodeAt(x, y);
    if (node !== null) this.nodeInfoPopup(node, cx, cy);
  }

  private mapAction(sq: number, x: number, y: number, cx: number, cy: number) {
    const other = this.squadAt(x, y);
    if (other !== null && other !== sq) {
      this.escortOptions(sq, other, cx, cy);
      return;
    }
    const node = this.map.nodeAt(x, y);
    const point = this.map.toMap(x, y);
    const opts = this.optionsAt(sq, node, point);
    const live = opts.filter((o) => o.mission);
    if (live.length === 1 && opts.length === 1) {
      this.setMission(sq, live[0].mission!);
      return;
    }
    if (opts.length === 0) {
      if (node === null && other === null) this.select(null);
      else toast("That squadron can't do anything there.");
      return;
    }
    this.showPopup(opts, cx, cy);
  }

  private escortOptions(sq: number, target: number, cx: number, cy: number) {
    const s = this.squad(sq)!;
    const u = unitDef(s.kind);
    const t = this.squad(target);
    if (!t) return;
    const opts: Option[] = [];
    if (u.orders.includes('Escort')) opts.push({ label: `Escort ${this.label(t)}`, mission: { t: 'Escort', sq: target }, sq });
    if (u.orders.includes('Jam')) opts.push({ label: `Jam along ${this.label(t)}'s route`, mission: { t: 'JamEscort', sq: target }, sq });
    if (!opts.length) {
      this.select(target);
      return;
    }
    if (this.mission(target).t === 'Rest') {
      opts.forEach((o) => (o.reason = `${this.label(t)} isn't flying anywhere yet.`));
      opts.forEach((o) => (o.mission = null));
    }
    this.showPopup(opts, cx, cy);
  }

  optionsAt(sq: number, node: number | null, point: P): Option[] {
    const s = this.squad(sq)!;
    const u = unitDef(s.kind);
    const can = (o: string) => u.orders.includes(o as never);
    const c = this.engine.content;
    const out: Option[] = [];
    const check = (m: Mission) => this.engine.checkMission(this.opts.state, sq, m);
    if (node !== null) {
      const n = c.nodes[node];
      const nv = this.view.nodes[node];
      const enemy = nv.owner !== null && nv.owner !== this.opts.seat;
      if (can('Attack') && (enemy || (nv.owner === null && n.kind !== 'Lake'))) {
        const m: Mission = { t: 'Attack', node };
        const r = check(m);
        const weak = s.kind === 'Fighter' && !s.ace;
        out.push({ label: `Attack ${n.name}${weak ? ' (fighters hit weakly)' : ''}`, mission: r ? null : m, reason: r, sq });
      }
      if (can('Guard') && n.kind !== 'Lake') {
        const m: Mission = { t: 'Guard', node };
        const r = check(m);
        const verb = s.kind === 'CropDuster' ? 'Lay smoke over' : s.kind === 'Watchtower' ? 'Watch over' : 'Guard';
        if (!r || nv.owner === this.opts.seat || nv.capturable) {
          out.push({ label: `${verb} ${n.name}${nv.capturable && nv.owner !== this.opts.seat ? ' (capture it)' : ''}`, mission: r ? null : m, reason: r, sq });
        }
      }
      if (can('Rebase') && (n.kind === 'Airbase' || n.kind === 'Boneyard' || n.kind === 'Capital' || (n.kind === 'Lake' && s.kind === 'Floatplane'))) {
        const m: Mission = { t: 'Rebase', node };
        const r = check(m);
        if (!r || nv.owner === this.opts.seat || n.kind === 'Lake') out.push({ label: `Rebase to ${n.name}`, mission: r ? null : m, reason: r, sq });
      }
    }
    const zone = node !== null ? c.nodes[node].zone : this.map.zoneAt(point);
    if (zone !== null) {
      const zn = zoneName(zone);
      if (can('Scout')) out.push({ label: `Scout ${zn}`, mission: { t: 'Scout', zone }, sq });
      if (can('Sweep')) out.push({ label: `Sweep ${zn} (hunt enemy aircraft)`, mission: { t: 'Sweep', zone }, sq });
      if (can('Jam')) out.push({ label: `Jam radar and air defense over ${zn}`, mission: { t: 'JamZone', zone }, sq });
      if (can('Guard') && node === null && s.kind !== 'CropDuster') {
        const m: Mission = { t: 'GuardZone', zone };
        const r = check(m);
        if (!r) out.push({ label: `Guard the skies over ${zn}`, mission: m, sq });
      }
    }
    return out;
  }

  private showPopup(opts: Option[], cx: number, cy: number) {
    this.popup?.remove();
    const box = h('div.popup');
    for (const o of opts) {
      let dot: HTMLElement | null = null;
      let fuel = '';
      if (o.mission && o.mission.t !== 'Rest') {
        const draft = structuredClone(this.orders);
        const i = draft.missions.findIndex((m) => m.sq === o.sq);
        const entry = { sq: o.sq, mission: o.mission, waypoint: null };
        if (i >= 0) draft.missions[i] = entry;
        else draft.missions.push(entry);
        const pv = this.engine.preview(this.opts.state, draft, o.sq, o.mission, null).preview;
        if (pv) {
          dot = h('span.risk', { style: { background: RISK_CSS[pv.color] }, title: pv.reason });
          fuel = ` · ${pv.fuel} fuel`;
          o.reason = o.reason ?? `${pv.color}: ${pv.reason}.`;
        }
      }
      const item = h(
        `button.popup-item${o.mission ? '' : '.disabled'}`,
        {
          onclick: () => {
            if (!o.mission) return;
            box.remove();
            this.setMission(o.sq, o.mission);
          },
        },
        dot,
        h('span.popup-label', o.label + fuel),
        o.reason ? h('span.popup-reason', o.reason) : null,
      );
      box.appendChild(item);
    }
    document.body.appendChild(box);
    const r = box.getBoundingClientRect();
    box.style.left = `${Math.min(cx + 8, window.innerWidth - r.width - 8)}px`;
    box.style.top = `${Math.min(cy + 8, window.innerHeight - r.height - 8)}px`;
    this.popup = box;
  }

  private nodeInfoPopup(node: number, cx: number, cy: number) {
    const c = this.engine.content;
    const n = c.nodes[node];
    const nv = this.view.nodes[node];
    const owner = nv.owner === null ? (n.kind === 'Lake' ? 'Nobody' : 'Neutral') : faction(this.opts.factions[nv.owner]).short;
    const opts: Option[] = [
      { label: `${n.name}`, mission: null, reason: `${KIND_LABEL[n.kind]} · ${owner} · ${healthText(nv.health, nv.exact)}`, sq: -1 },
      { label: n.tagline, mission: null, sq: -1 },
    ];
    const mine = nv.owner === this.opts.seat;
    if (mine && nv.health < 2 && n.kind !== 'Lake') {
      const queued = this.orders.repairs.includes(node);
      const box = h('div.popup');
      for (const o of opts) box.appendChild(h('div.popup-item.info', h('span.popup-label', o.label), o.reason ? h('span.popup-reason', o.reason) : null));
      box.appendChild(h('button.popup-item', { onclick: () => { box.remove(); this.toggleRepair(node); } }, queued ? 'Cancel repair' : `Repair now (${c.rules.economy.repair_cost} Supply)`));
      document.body.appendChild(box);
      box.style.left = `${Math.min(cx + 8, window.innerWidth - 300)}px`;
      box.style.top = `${Math.min(cy + 8, window.innerHeight - 160)}px`;
      this.popup?.remove();
      this.popup = box;
      return;
    }
    this.showPopup(opts, cx, cy);
    this.popup?.querySelectorAll('.popup-item').forEach((el) => el.classList.add('info'));
  }

  private hover(x: number, y: number) {
    const node = this.map.nodeAt(x, y);
    const sq = this.squadAt(x, y);
    const c = this.engine.content;
    let text = '';
    if (sq !== null) {
      const s = this.squad(sq);
      const enemy = this.view.enemies.find((e) => e.sq === sq);
      if (s) text = `<b>${esc(this.label(s))}</b><br>${esc(missionLabel(this.mission(sq)))}`;
      else if (enemy) text = enemyText(enemy, this.opts.factions, this.view.night);
    } else if (node !== null) {
      const n = c.nodes[node];
      const nv = this.view.nodes[node];
      const owner = nv.owner === null ? (n.kind === 'Lake' ? '' : 'Neutral') : faction(this.opts.factions[nv.owner]).short;
      text = `<b>${esc(n.name)}</b><br>${esc(KIND_LABEL[n.kind])}${owner ? ' · ' + esc(owner) : ''} · ${healthText(nv.health, nv.exact)}`;
      if (n.objective > 0) text += `<br>Hold it at dawn: +${n.objective} Bragging Rights a night`;
      if (this.selected !== null && nv.attack && nv.owner !== this.opts.seat && nv.owner !== null) text += `<br><i>${esc(nv.attack)}</i>`;
      const enemies = this.view.enemies.filter((e) => e.base === node);
      if (enemies.length) text += `<br>${enemies.map((e) => esc(enemyText(e, this.opts.factions, this.view.night, true))).join('<br>')}`;
      if (n.tagline) text += `<br><span class="tip-tag">${esc(n.tagline)}</span>`;
    }
    if (!text) {
      this.tip.style.display = 'none';
      return;
    }
    this.tip.innerHTML = text;
    this.tip.style.display = 'block';
    const w = this.mapWrap.clientWidth;
    this.tip.style.left = `${Math.min(x + 16, w - 260)}px`;
    this.tip.style.top = `${y + 14}px`;
  }

  // --- Overlay drawing ---------------------------------------------------------------------------------

  private handlePos(sq: number): P | null {
    const m = this.orders.missions.find((x) => x.sq === sq);
    const p = this.previews.get(sq);
    if (!m || !p || m.mission.t === 'Escort' || m.mission.t === 'JamEscort') return null;
    if (m.waypoint) return m.waypoint;
    const path = p.path;
    if (path.length < 2) return null;
    const a = path[0];
    const b = path[path.length - 1];
    return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  }

  drawOverlay() {
    const v = this.view;
    const c = this.engine.content;
    const map = this.map;
    if (!map) return;
    map.setNodes(v.nodes.map((n) => ({ owner: n.owner, health: n.health, active: n.active, smoke: n.smoke, exact: n.exact })));
    map.setFlows(this.opts.seat, v.flows.links);
    // Highlights for the selected squadron's options.
    map.highlights.clear();
    if (this.selected !== null) {
      const s = this.squad(this.selected);
      const u = s ? unitDef(s.kind) : null;
      if (s && u) {
        for (const n of c.nodes) {
          const nv = v.nodes[n.idx];
          if (u.orders.includes('Attack') && nv.attack === null && nv.owner !== null && nv.owner !== this.opts.seat) map.highlights.set(n.idx, 0xff4d4d);
          else if (u.orders.includes('Guard') && nv.guard === null && n.kind !== 'Lake' && (nv.owner === this.opts.seat || nv.capturable)) map.highlights.set(n.idx, 0x3ddc84);
          else if (u.orders.includes('Rebase') && n.idx !== s.base && ((n.kind === 'Lake' && s.kind === 'Floatplane') || (['Airbase', 'Boneyard'].includes(n.kind) && nv.owner === this.opts.seat))) map.highlights.set(n.idx, 0x4c9bff);
        }
      }
    }
    map.drawHighlights();
    // Weak spot and best targets.
    const mg = this.markG;
    mg.clear();
    map.removeLabels('mark-');
    if (this.showWeak && this.weak) {
      for (const n of this.weak.nodes) {
        const p = c.nodes[n].pos;
        mg.circle(p[0], p[1], 15).stroke({ width: 2.5, color: 0xff9f1c, alpha: 0.95 });
        mg.circle(p[0], p[1], 19).stroke({ width: 1, color: 0xff9f1c, alpha: 0.6 });
        map.label(`mark-w${n}`, 'WEAK SPOT', [p[0], p[1] - 22], 'label-mark weak');
      }
    }
    if (this.showWeak && this.enemyWeak) {
      for (const n of this.enemyWeak.nodes) {
        const p = c.nodes[n].pos;
        mg.circle(p[0], p[1], 15).stroke({ width: 2.5, color: 0xff4d4d, alpha: 0.95 });
        map.label(`mark-e${n}`, 'THEIR WEAK SPOT', [p[0], p[1] - 22], 'label-mark weak');
      }
    }
    if (this.showTargets) {
      this.targets.forEach((t, i) => {
        const p = c.nodes[t.node].pos;
        mg.circle(p[0], p[1], 14).stroke({ width: 2, color: 0xffd23f, alpha: 0.95 });
        map.label(`mark-t${t.node}`, `#${i + 1}`, [p[0] + 14, p[1] - 14], 'label-mark target');
      });
    }
    // The Big Fetch target: a tennis ball, and its arc from the park.
    if (this.orders.fetch !== null) {
      const p = c.nodes[this.orders.fetch].pos;
      const park = c.landmarks?.find((l) => l.id === 'bark_park')?.pos;
      if (park) {
        const mid: P = [(park[0] + p[0]) / 2, Math.min(park[1], p[1]) - 60];
        mg.moveTo(park[0], park[1]).quadraticCurveTo(mid[0], mid[1], p[0], p[1]).stroke({ width: 1.5, color: 0xd7f542, alpha: 0.7 });
      }
      mg.circle(p[0], p[1], 16).stroke({ width: 2.5, color: 0xd7f542, alpha: 0.95 });
      mg.circle(p[0], p[1], 5).fill({ color: 0xd7f542 });
      map.label('mark-fetch', 'BIG FETCH', [p[0], p[1] - 24], 'label-mark fetch');
    }
    // Revealed zones glow faintly.
    for (const r of v.revealed) {
      const z = c.zones[r.zone];
      mg.poly(z.poly.flat()).stroke({ width: 2, color: 0xffffff, alpha: 0.25 });
    }
    this.drawRoutes();
    this.drawUnits();
  }

  private drawRoutes() {
    const g = this.routeG;
    g.clear();
    const c = this.engine.content;
    for (const o of this.orders.missions) {
      const p = this.previews.get(o.sq);
      if (!p) continue;
      if (o.mission.t === 'Escort' || o.mission.t === 'JamEscort') continue;
      const sel = this.selected === o.sq;
      const color = RISK_COLOR[p.color];
      let path = p.path;
      if (sel && this.dragWaypoint && this.dragWaypoint.sq === o.sq) path = [path[0], this.dragWaypoint.p, path[path.length - 1]];
      const width = sel ? 3 : 1.8;
      const flat = path.flat();
      g.poly(flat, false).stroke({ width: width + 4, color, alpha: sel ? 0.25 : 0.12 });
      g.poly(flat, false).stroke({ width, color, alpha: sel ? 1 : 0.75 });
      const end = path[path.length - 1];
      const prev = path[path.length - 2] ?? path[0];
      const ang = Math.atan2(end[1] - prev[1], end[0] - prev[0]);
      if (o.mission.t === 'Attack' || o.mission.t === 'Rebase') {
        g.poly([end[0], end[1], end[0] - Math.cos(ang - 0.4) * 9, end[1] - Math.sin(ang - 0.4) * 9, end[0] - Math.cos(ang + 0.4) * 9, end[1] - Math.sin(ang + 0.4) * 9]).fill({ color });
      }
      if (o.mission.t === 'Sweep' || o.mission.t === 'Scout' || o.mission.t === 'JamZone' || o.mission.t === 'GuardZone') {
        const z = c.zones[o.mission.zone];
        g.circle(z.center[0], z.center[1], o.mission.t === 'Sweep' ? z.radius * 0.66 : 22).stroke({ width: 1.5, color, alpha: 0.8 });
      }
      if (o.mission.t === 'Guard') {
        const n = c.nodes[o.mission.node];
        g.circle(n.pos[0], n.pos[1], 16).stroke({ width: 2, color: 0x3ddc84, alpha: 0.9 });
      }
      // Escort and jammer badges along the lead's route.
      const flying = this.orders.missions.filter((x) => (x.mission.t === 'Escort' || x.mission.t === 'JamEscort') && x.mission.sq === o.sq);
      if (flying.length) {
        const mid = path[Math.floor(path.length / 2)];
        g.circle(mid[0], mid[1], 6).fill({ color: 0x071022 }).stroke({ width: 1.5, color: 0xffffff });
      }
      if (sel) {
        const hp = this.dragWaypoint?.sq === o.sq ? this.dragWaypoint.p : this.handlePos(o.sq);
        if (hp) {
          g.circle(hp[0], hp[1], 6).fill({ color: 0x071022 }).stroke({ width: 2, color: 0xffffff });
          g.circle(hp[0], hp[1], 2).fill({ color: 0xffffff });
        }
      }
    }
  }

  private drawUnits() {
    const g = this.unitG;
    g.clear();
    this.glyphSpots.clear();
    const c = this.engine.content;
    const byBase = new Map<number, { sq: number; kind: UnitKind; f: FactionId; ghost: boolean; mine: boolean; strength: number; ace: boolean }[]>();
    for (const s of this.view.squadrons) {
      if (s.down_until > 0) continue;
      const list = byBase.get(s.base) ?? [];
      list.push({ sq: s.id, kind: s.kind, f: this.view.faction, ghost: false, mine: true, strength: s.strength, ace: s.ace });
      byBase.set(s.base, list);
    }
    for (const e of this.view.enemies) {
      const list = byBase.get(e.base) ?? [];
      list.push({ sq: e.sq, kind: e.kind, f: this.opts.factions[e.seat], ghost: e.night < this.view.night, mine: false, strength: e.strength, ace: e.ace });
      byBase.set(e.base, list);
    }
    const zoom = this.map.cam.zoom;
    const size = 5.2 / Math.sqrt(zoom);
    for (const [base, list] of byBase) {
      const n = c.nodes[base];
      list.forEach((u, i) => {
        const ang = -Math.PI / 2 + (i - (list.length - 1) / 2) * 0.55;
        const r = 15 + Math.floor(i / 6) * 8;
        const x = n.pos[0] + Math.cos(ang) * r;
        const y = n.pos[1] + Math.sin(ang) * r;
        const sel = this.selected === u.sq;
        if (sel) g.circle(x, y, size * 2).fill({ color: 0xffffff, alpha: 0.25 });
        const ordered = u.mine && this.mission(u.sq).t !== 'Rest';
        drawGlyph(g, u.f, u.kind, x, y, size, -Math.PI / 2, u.ghost ? 0.35 : u.mine && !ordered ? 0.75 : 1);
        if (u.ace) g.circle(x, y, size * 1.5).stroke({ width: 0.8, color: 0xffffff, alpha: u.ghost ? 0.3 : 0.8 });
        if (ordered) g.circle(x + size, y - size, 1.6).fill({ color: factionColor(u.f) });
        this.glyphSpots.set(u.sq, [x, y]);
      });
    }
  }

  // --- Side panel --------------------------------------------------------------------------------------

  private renderSide() {
    clear(this.side);
    const tabs: [typeof this.sideTab, string][] = [
      ['advisor', 'Advisor'],
      ['build', 'Build & Repair'],
      ['faction', 'Faction'],
      ['intel', 'Intel'],
    ];
    this.side.appendChild(
      h('div.side-tabs', ...tabs.map(([k, label]) => h(`button.side-tab${this.sideTab === k ? '.on' : ''}`, { onclick: () => { this.sideTab = k; this.renderSide(); } }, label, k === 'faction' && this.factionNeedsYou() ? h('span.tab-dot') : null))),
    );
    const body = h('div.side-body');
    this.side.appendChild(body);
    if (this.sideTab === 'advisor') this.renderAdvisor(body);
    if (this.sideTab === 'build') this.renderBuild(body);
    if (this.sideTab === 'faction') this.renderFaction(body);
    if (this.sideTab === 'intel') this.renderIntel(body);
  }

  /** The Faction tab has something time-sensitive: the doghouse or a loaded Big Fetch. */
  private factionNeedsYou(): boolean {
    const me = this.view.seats[this.view.seat];
    return (me.doghouse && !this.orders.retreat) || (me.fetch && this.orders.fetch === null);
  }

  private renderAdvisor(body: HTMLElement) {
    const f = this.view.faction;
    const worst = this.warnings[0]?.severity ?? 0;
    const mood = worst >= 3 ? 'alarmed' : worst === 0 ? 'happy' : 'neutral';
    const L = leader(f);
    body.appendChild(h('div.adv-head', svg(leaderSvg(f, mood, { size: 84, bg: true }), 'adv-dog'), h('div', h('div.adv-name', L.name), h('div.adv-title', L.title))));
    const shown = this.warnings.filter((w) => w.severity > 0).slice(0, 3);
    const info = this.warnings.filter((w) => w.severity === 0);
    const list = [...shown, ...(shown.length < 3 ? info.slice(0, 3 - shown.length) : [])];
    for (const w of list) {
      const params = { ...w.params };
      if (params.reason) params.reason = params.reason;
      const line = advisorLine(f, w.key, params, this.view.night * 31 + w.key.length * 7 + (w.sq ?? 0));
      if (!line) continue;
      const box = h(
        `div.adv-line.sev${w.severity}`,
        {
          onclick: () => {
            if (w.sq !== null) this.select(w.sq);
            if (['doghouse', 'fetch_ready', 'retreating'].includes(w.key)) { this.sideTab = 'faction'; this.renderSide(); }
          },
        },
        ...turns(line).map((t) => h('div', t.who ? h('b', t.who + ': ') : null, t.text)),
      );
      body.appendChild(box);
    }
    // Tonight's situation.
    const notes: string[] = [];
    notes.push(`${stageName(this.view.stage)}: ${stageBlurb(this.view.stage)}`);
    const patch = patchText(this.view.patch);
    if (patch && f === 'Pacific') notes.push(`Patch Notes ${patch.version}: ${patch.text}`);
    for (const e of this.view.effects.filter((e) => e.from <= this.view.night && this.view.night <= e.until)) {
      notes.push(`${faction(this.opts.factions[e.seat]).short}: ${effectText(e.kind)}`);
    }
    if (f === 'Texas' && this.view.texas_grudge.length) {
      notes.push(`Remember the Alamo: strikes on ${this.view.texas_grudge.map((s) => faction(this.opts.factions[s]).short).join(', ')} fly free tonight.`);
    }
    if (f === 'Beehive' && this.view.stage < 2) {
      const g = this.view.beehive_grudge.map((s) => faction(this.opts.factions[s]).short);
      notes.push(g.length ? `Oh My Heck: you may hit back at ${g.join(', ')}.` : 'Oh My Heck: nobody has hit Utah yet, so no attacks on anyone (neutral targets are fine).');
    }
    if (this.view.pending.length) notes.push(`Arriving: ${this.view.pending.map((p) => `${unitName(p.kind)} (Night ${p.night})`).join(', ')}.`);
    body.appendChild(h('ul.adv-notes', ...notes.map((n) => h('li', n))));
  }

  private renderBuild(body: HTMLElement) {
    const v = this.view;
    const c = this.engine.content;
    const left = v.supply - this.supplyPlanned();
    const used = this.orders.builds.length;
    body.appendChild(h('div.side-h', `Build (factories: ${Math.max(0, v.build_capacity - used)} of ${v.build_capacity} slots left)`));
    body.appendChild(h('div.side-note', v.faction === 'Cascadia' ? 'Process: new squadrons arrive a night late, as veterans.' : 'New squadrons appear at an airbase next dawn.'));
    for (const b of v.builds) {
      const u = c.units.find((x) => x.kind === b.kind)!;
      const n = this.orders.builds.filter((k) => k === b.kind).length;
      const can = b.ok && left >= b.cost && used < v.build_capacity;
      body.appendChild(
        h(
          'div.build-row',
          svg(glyphSvg(v.faction, b.kind, 26), 'build-glyph'),
          h('div.build-name', h('div', u.name, n ? h('b', ` ×${n}`) : null), h('div.side-note', b.reason ?? u.desc)),
          h('button.pbtn', { disabled: !can, onclick: () => { this.orders.builds.push(b.kind); this.refresh(); } }, `+ ${b.cost}`),
          n ? h('button.pbtn', { onclick: () => { const i = this.orders.builds.lastIndexOf(b.kind); this.orders.builds.splice(i, 1); this.refresh(); } }, '−') : null,
        ),
      );
    }
    const damaged = v.nodes.map((n, i) => ({ n, i })).filter(({ n, i }) => n.owner === this.opts.seat && n.health < 2 && c.nodes[i].kind !== 'Lake');
    body.appendChild(h('div.side-h', 'Repairs'));
    body.appendChild(h('div.side-note', `Damaged nodes repair one step a night for free, starting the night after a hit. Pay ${c.rules.economy.repair_cost} Supply to repair a step right now.`));
    if (!damaged.length) body.appendChild(h('div.side-note', 'Nothing damaged. Nice.'));
    for (const { n, i } of damaged) {
      const queued = this.orders.repairs.includes(i);
      body.appendChild(
        h(
          'div.build-row',
          h('div.build-name', h('div', c.nodes[i].name), h('div.side-note', `${KIND_LABEL[c.nodes[i].kind]} · ${healthText(n.health, true)}`)),
          h('button.pbtn' + (queued ? '.on' : ''), { disabled: !queued && left < c.rules.economy.repair_cost, onclick: () => this.toggleRepair(i) }, queued ? 'Queued ✓' : `Repair ${c.rules.economy.repair_cost}`),
        ),
      );
    }
  }

  toggleRepair(node: number) {
    const i = this.orders.repairs.indexOf(node);
    if (i >= 0) this.orders.repairs.splice(i, 1);
    else this.orders.repairs.push(node);
    this.refresh();
  }

  private renderFaction(body: HTMLElement) {
    const v = this.view;
    const c = this.engine.content;
    const f = v.faction;
    const fd = faction(f);
    const me = this.opts.seat;
    const others = this.opts.factions.map((x, i) => ({ f: x, i })).filter((x) => x.i !== me);
    const card = fd.card;
    this.renderPark(body);
    body.appendChild(h('div.side-h', `${card.signature[0]}`));
    body.appendChild(h('div.side-note', card.signature[1]));
    const left = v.supply - this.supplyPlanned();
    const num = (value: number, min: number, max: number, on: (n: number) => void) =>
      h('input.num', { type: 'number', min: String(min), max: String(max), value: String(value), onchange: (e: Event) => on(Math.max(min, Math.min(max, Number((e.target as HTMLInputElement).value) || 0))) });
    const select = (items: [string, string][], value: string, on: (v: string) => void) => {
      const s = h('select', { onchange: (e: Event) => on((e.target as HTMLSelectElement).value) }, ...items.map(([k, label]) => h('option', { value: k, selected: k === value }, label))) as HTMLSelectElement;
      return s;
    };
    // Dreamland: the secret bet.
    if (f === 'Dreamland') {
      const bet = this.orders.bet;
      const on = bet?.on ?? others[0].i;
      const victimNodes = c.nodes.filter((n) => v.nodes[n.idx].owner !== null && v.nodes[n.idx].owner !== on && n.kind !== 'Lake');
      const tgt = bet ? (bet.node !== null ? `n${bet.node}` : `s${bet.victim}`) : `s${others.find((o) => o.i !== on)?.i ?? me}`;
      body.appendChild(
        h(
          'div.fa-box',
          h('div.fa-row', 'Bet that ', select(others.map((o) => [String(o.i), faction(o.f).short]), String(on), (x) => { this.orders.bet = { on: Number(x), node: null, victim: others.find((o) => o.i !== Number(x))?.i ?? me, stake: bet?.stake ?? 2 }; this.refresh(); })),
          h(
            'div.fa-row',
            'hits ',
            select(
              [
                ...this.opts.factions.map((x, i) => [`s${i}`, `anything of ${faction(x).short} (pays ×2)`] as [string, string]).filter(([k]) => k !== `s${on}`),
                ...victimNodes.map((n) => [`n${n.idx}`, `${n.name} (pays ×4)`] as [string, string]),
              ],
              tgt,
              (x) => {
                const b = this.orders.bet ?? { on, node: null, victim: null, stake: 2 };
                if (x.startsWith('n')) this.orders.bet = { ...b, on, node: Number(x.slice(1)), victim: null };
                else this.orders.bet = { ...b, on, node: null, victim: Number(x.slice(1)) };
                this.refresh();
              },
            ),
          ),
          h('div.fa-row', 'Stake ', num(bet?.stake ?? 0, 0, Math.min(c.rules.factions.bet_max ?? 6, left + (bet?.stake ?? 0)), (n) => { this.orders.bet = n > 0 ? { ...(this.orders.bet ?? { on, node: null, victim: others.find((o) => o.i !== on)?.i ?? me, stake: 0 }), stake: n } : null; this.refresh(); }), ' Supply (0 = no bet)'),
          h('div.side-note', 'Payouts flash on the big screen during the show. Losing bets stay secret.'),
        ),
      );
    }
    // Colorado: sell a radar picture.
    if (f === 'FrontRange') {
      const sale = this.orders.intel_sale;
      body.appendChild(
        h(
          'div.fa-box',
          h('div.fa-row', 'Sell one radar picture tonight for ', num(sale?.price ?? 0, 0, 20, (n) => { this.orders.intel_sale = n > 0 ? { price: n, only: sale?.only ?? null } : null; this.refresh(); }), ' Supply (0 = not selling)'),
          h('div.fa-row', 'To ', select([['any', 'whoever bids highest'], ...others.map((o) => [String(o.i), `${faction(o.f).short} only`] as [string, string])], sale?.only === null || sale?.only === undefined ? 'any' : String(sale.only), (x) => { if (this.orders.intel_sale) this.orders.intel_sale.only = x === 'any' ? null : Number(x); this.refresh(); })),
          h('div.side-note', 'Buyers name a region and a top price in their own planning. The deal delivers at dawn. Make your pitch at the table.'),
        ),
      );
    } else if (this.opts.factions.includes('FrontRange')) {
      const req = this.orders.intel_request;
      body.appendChild(h('div.side-h', 'Buy a radar picture from Colorado'));
      body.appendChild(
        h(
          'div.fa-box',
          h('div.fa-row', 'Region ', select([['none', 'not buying'], ...c.zones.map((z) => [String(z.idx), z.name] as [string, string])], req ? String(req.zone) : 'none', (x) => { this.orders.intel_request = x === 'none' ? null : { zone: Number(x), max_price: req?.max_price ?? 3 }; this.refresh(); })),
          req ? h('div.fa-row', 'Pay up to ', num(req.max_price, 1, 20, (n) => { this.orders.intel_request = { ...req, max_price: n }; this.refresh(); }), ' Supply') : null,
          h('div.side-note', "Delivered at dawn if Colorado is selling at or under your price. Colorado's radar reaches far, but not everywhere."),
        ),
      );
    }
    // Harvest Shipment.
    if (v.harvest > 0 && f !== 'GrainBelt') {
      body.appendChild(h('div.side-h', `Harvest Shipment: worth ${v.harvest} Supply`));
      body.appendChild(
        h(
          'div.fa-box',
          h('div.fa-row', 'Secret bid ', num(this.orders.harvest_bid, 0, left + this.orders.harvest_bid, (n) => { this.orders.harvest_bid = n; this.refresh(); }), ' Supply'),
          h('div.side-note', 'Highest bid wins the shipment at dawn; the Grain Belt keeps the bid. Losers get their Supply back.'),
        ),
      );
    }
    if (f === 'GrainBelt') {
      body.appendChild(h('div.side-note', v.harvest > 0 ? `Tonight's shipment is worth ${v.harvest}. Everyone else bids in secret; you keep the winning bid.` : ''));
    }
    // Grain Belt revenge strike.
    if (f === 'GrainBelt' && v.revenge_against !== null) {
      const enemyNodes = c.nodes.filter((n) => v.nodes[n.idx].owner === v.revenge_against && v.nodes[n.idx].attack === null);
      body.appendChild(h('div.side-h', `Revenge strike on ${faction(this.opts.factions[v.revenge_against]).short}`));
      body.appendChild(h('div.fa-box', select([['none', 'pick a target'], ...enemyNodes.map((n) => [String(n.idx), n.name] as [string, string])], this.orders.revenge === null ? 'none' : String(this.orders.revenge), (x) => { this.orders.revenge = x === 'none' ? null : Number(x); this.refresh(); }), h('div.side-note', 'They hit the Ball of Twine. It cannot be intercepted.')));
    }
    // The Mass Driver.
    if (v.mass_driver_holder === me && v.stage === 2 && !v.mass_driver_fired) {
      const targets = c.nodes.filter((n) => v.nodes[n.idx].attack === null && v.nodes[n.idx].owner !== null && v.nodes[n.idx].owner !== me);
      body.appendChild(h('div.side-h', 'White Sands Mass Driver: one shot per game'));
      body.appendChild(h('div.fa-box', select([['none', 'hold fire'], ...targets.map((n) => [String(n.idx), `${n.name} (${faction(this.opts.factions[v.nodes[n.idx].owner!]).short})`] as [string, string])], this.orders.mass_driver === null ? 'none' : String(this.orders.mass_driver), (x) => { this.orders.mass_driver = x === 'none' ? null : Number(x); this.refresh(); }), h('div.side-note', 'Knocks any node down a step. Nothing can stop it.')));
    } else if (v.mass_driver_holder === me) {
      body.appendChild(h('div.side-note', v.mass_driver_fired ? 'You hold the Mass Driver. Its one shot has been fired.' : 'You hold the Mass Driver. Its one shot opens at Gloves Off (Night 6).'));
    }
    body.appendChild(h('div.side-h', 'Your five rules'));
    for (const [k, [t, d]] of Object.entries(card) as [string, [string, string]][]) body.appendChild(h('div.rule-mini', h('b', `${cap(k)}: ${t}. `), d));
  }

  /** Project Bark Park and the Big Fetch, when they apply. */
  private renderPark(body: HTMLElement) {
    const v = this.view;
    const c = this.engine.content;
    const me = v.seats[v.seat];
    const bp = c.rules.bark_park;
    if (me.doghouse || this.orders.retreat) {
      body.appendChild(
        h(
          `div.park-box${this.orders.retreat ? '.on' : ''}`,
          h('div.park-title', '🐾 Project Bark Park'),
          h('div.side-note', `You're in the doghouse. Once a game you can spend tonight at a secret dog park at Four Corners. Your squadrons stay home and rest, and nothing can touch your land tonight: bombs bounce off the fence. At dawn you're back with everything repaired, at least ${bp.floor} squadrons, ${bp.supply} more Supply, and one shot of the Big Fetch.`),
          h(
            `button.park-go${this.orders.retreat ? '.on' : ''}`,
            { onclick: () => this.setRetreat(!this.orders.retreat) },
            this.orders.retreat ? '✓ Going to the Bark Park tonight (tap to call it off)' : 'Retreat to the Bark Park tonight',
          ),
        ),
      );
    } else if (me.been_to_park) {
      body.appendChild(h('div.side-note', 'You have been to Project Bark Park this game.'));
    }
    if (me.fetch) {
      const targets = c.nodes.filter((n) => v.nodes[n.idx].attack === null && v.nodes[n.idx].owner !== null && v.nodes[n.idx].owner !== v.seat);
      const label = (n: (typeof targets)[number]) => `${n.kind === 'CrownJewel' ? '★ ' : ''}${n.name} (${faction(this.opts.factions[v.nodes[n.idx].owner!]).short})`;
      const sel = h(
        'select',
        { onchange: (e: Event) => { const x = (e.target as HTMLSelectElement).value; this.orders.fetch = x === 'none' ? null : Number(x); this.refresh(); } },
        h('option', { value: 'none', selected: this.orders.fetch === null }, 'hold the ball'),
        ...targets
          .sort((a, b) => Number(b.kind === 'CrownJewel') - Number(a.kind === 'CrownJewel') || a.name.localeCompare(b.name))
          .map((n) => h('option', { value: String(n.idx), selected: this.orders.fetch === n.idx }, label(n))),
      );
      body.appendChild(
        h(
          'div.park-box.fetch',
          h('div.park-title', '🎾 The Big Fetch: one shot'),
          h('div.fa-row', 'Target ', sel),
          h('div.side-note', `A stadium-sized tennis ball from the Bark Park, late in the night. It can't be intercepted and knocks the target down ${bp.fetch_steps} steps; a crown jewel counts as a full hit.${v.stage < 2 ? ' Crown jewels open up at Gloves Off (Night 6).' : ''} It bounces off anyone at the park.`),
        ),
      );
    }
  }

  /** Go to the park (everyone rests) or call it off (tonight's orders come back). */
  private setRetreat(on: boolean) {
    if (on === this.orders.retreat) return;
    if (on) {
      this.parkSaved = structuredClone(this.orders.missions);
      this.orders.retreat = true;
      for (const m of this.orders.missions) {
        m.mission = { t: 'Rest' };
        m.waypoint = null;
      }
      this.orders.mass_driver = null;
      this.orders.revenge = null;
      this.selected = null;
      audio.play('commit');
    } else {
      this.orders.retreat = false;
      if (this.parkSaved) this.orders.missions = this.parkSaved;
      this.parkSaved = null;
      audio.play('click');
    }
    this.refresh();
  }

  private renderIntel(body: HTMLElement) {
    const v = this.view;
    const me = this.opts.seat;
    body.appendChild(h('div.side-h', 'Scoreboard'));
    for (const s of v.seats) body.appendChild(h('div.intel-row', { style: { color: faction(s.faction).color } }, `${faction(s.faction).short}${s.seat === me ? ' (you)' : ''}`, h('b', String(s.score)), s.scrappy ? h('span.tag', 'SCRAPPY') : null));
    body.appendChild(h('div.side-h', 'Enemy weak spots'));
    body.appendChild(h('div.side-note', 'Pick a rival to see where their network is thinnest, as far as you know.'));
    for (const s of v.seats.filter((x) => x.seat !== me)) {
      body.appendChild(
        h('button.pbtn.wide', {
          onclick: () => {
            this.enemyWeak = this.engine.weakSpot(this.opts.state, me, s.seat);
            this.showWeak = true;
            this.refresh();
            const w = this.enemyWeak;
            toast(w.nodes.length ? `${w.nodes.map(nodeName).join(' and ')}: ${w.pct}% of ${faction(s.faction).short}'s ${w.resource}.` : 'No single weak spot found.', 5000);
          },
        }, `${faction(s.faction).short}`),
      );
    }
    body.appendChild(h('div.side-h', 'What you can see'));
    const seen = v.enemies.filter((e) => e.night === v.night).length;
    const ghosts = v.enemies.length - seen;
    body.appendChild(h('div.side-note', `${seen} enemy squadrons on radar right now; ${ghosts} remembered from earlier nights (faded).`));
    if (v.revealed.length) body.appendChild(h('div.side-note', `Revealed tonight: ${v.revealed.map((r) => zoneName(r.zone) + (r.half ? ' (half, overcast)' : '') + (r.radar_of !== null ? ' (bought)' : '')).join(', ')}.`));
  }

  // --- Buttons ------------------------------------------------------------------------------------------

  private suggest() {
    const s = this.engine.suggest(this.opts.state, this.opts.seat, 'normal');
    this.orders = normalize(s, this.view);
    toast('Suggested plan filled in. Tweak anything you like.');
    this.refresh();
    this.opts.coach?.step(this);
  }

  private reset() {
    this.orders = normalize(this.view.carry_over, this.view);
    this.refresh();
  }

  private toggleWeak() {
    this.showWeak = !this.showWeak;
    if (!this.showWeak) this.enemyWeak = null;
    this.refresh();
    if (this.showWeak && this.weak && this.weak.nodes.length) {
      toast(`${this.weak.nodes.map(nodeName).join(' and ')} carry ${this.weak.pct}% of your ${this.weak.resource}.`, 5000);
    }
  }

  private toggleTargets() {
    this.showTargets = !this.showTargets;
    this.refresh();
  }

  commit(force: boolean) {
    if (this.committed) return;
    const worst = this.warnings[0]?.severity ?? 0;
    if (!force && worst >= 3) {
      const close = modal(
        h(
          'div.confirm',
          h('h2', 'Commit anyway?'),
          h('p', `${leader(this.view.faction).name} has concerns:`),
          h('ul', ...this.warnings.filter((w) => w.severity >= 3).map((w) => h('li', advisorLine(this.view.faction, w.key, w.params, 1)))),
          h('div.confirm-btns', h('button.pbtn', { onclick: () => close() }, 'Keep planning'), h('button.commit', { onclick: () => { close(); this.commit(true); } }, 'Commit')),
        ),
      );
      return;
    }
    this.committed = true;
    audio.play('commit');
    this.opts.onCommit(structuredClone(this.orders));
  }
}

function cap(s: string) {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function healthText(h: number, exact: boolean): string {
  const t = h === 2 ? 'full strength' : h === 1 ? 'damaged (half output)' : 'knocked out';
  return exact ? t : `${t}, last we saw`;
}

function enemyText(e: { kind: UnitKind; seat: number; strength: number; ace: boolean; night: number }, factions: FactionId[], night: number, plain = false): string {
  const who = faction(factions[e.seat]).short;
  const name = e.ace ? faction(factions[e.seat]).ace : unitName(e.kind);
  const age = night - e.night;
  const when = age <= 0 ? 'on radar now' : `last seen ${age === 1 ? 'last night' : `${age} nights ago`}`;
  const s = `${who} ${name} (${e.strength}/4), ${when}`;
  return plain ? s : `<b>${esc(who)} ${esc(name)}</b><br>${e.strength}/4 · ${esc(when)}`;
}

/** Make sure every active squadron has an entry, in a stable order. */
function normalize(o: Orders, v: SeatView): Orders {
  const out: Orders = structuredClone(o);
  out.seat = v.seat;
  const active = new Set(v.squadrons.filter((s) => s.down_until === 0).map((s) => s.id));
  out.missions = out.missions.filter((m) => active.has(m.sq));
  for (const s of v.squadrons) {
    if (s.down_until === 0 && !out.missions.some((m) => m.sq === s.id)) out.missions.push({ sq: s.id, mission: { t: 'Rest' }, waypoint: null });
  }
  out.builds = out.builds ?? [];
  out.repairs = (out.repairs ?? []).filter((n) => v.nodes[n]?.owner === v.seat && v.nodes[n].health < 2);
  out.harvest_bid = v.harvest > 0 ? out.harvest_bid ?? 0 : 0;
  if (out.mass_driver !== null && v.mass_driver_holder !== v.seat) out.mass_driver = null;
  if (out.revenge !== null && v.revenge_against === null) out.revenge = null;
  const me = v.seats[v.seat];
  out.retreat = !!out.retreat && me.doghouse;
  out.fetch = me.fetch && out.fetch !== null && out.fetch !== undefined && v.nodes[out.fetch]?.attack === null ? out.fetch : null;
  return out;
}

export type { NodeKind };
