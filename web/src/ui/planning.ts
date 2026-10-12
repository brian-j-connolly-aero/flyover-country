// The planning screen: private, short and readable at a glance. Drag a
// squadron card onto the map: bombers onto a target, fighters onto your land
// to patrol (the circle shows what they cover) or onto a target to escort.
// Build and repair on the side, then Commit.

import { Graphics } from 'pixi.js';
import { MapView, SITE_R } from '../map/mapview';
import type { Engine } from '../engine';
import type { Checked, FactionId, GameState, Mission, Orders, P, SeatKind, Squadron, Target, UnitKind } from '../types';
import { GOOD, HIT_RED, INK, dashedCircle, drawPlane, drawRaidPath, factionColor, FACTION_HEX } from '../theme';
import { faction, KIND_NAME, missionLabel, plural, squadronName, opensTonight } from '../text';
import { leaderSvg, emblemSvg } from '../art/dogs';
import { crateSvg, factionPlaneSvg, pipsHtml, siteSvg } from './glyph';
import { h, clear, svg, modal, toast } from './dom';
import { rulesModal } from './rules';
import { audio } from '../audio';

export interface Coach {
  /** Called after every change so a tutorial can move to its next step. */
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

type Drop = { mission: Mission } | { error: string };

export class PlanningScreen {
  readonly el = h('div.plan');
  readonly opts: PlanningOpts;
  readonly engine: Engine;
  readonly seat: number;
  readonly me: FactionId;
  state: GameState;
  orders: Orders;
  checked!: Checked;
  selected: number | null = null;
  map!: MapView;
  readonly mapWrap = h('div.plan-mapwrap');
  private top = h('header.plan-top');
  private side = h('aside.plan-side');
  private tray = h('div.plan-tray');
  private hint = h('div.plan-hint');
  private tip = h('div.plan-tip');
  private planG = new Graphics();
  private dragG = new Graphics();
  private drag: { sq: number; ghost: HTMLElement; x: number; y: number; moved: boolean } | null = null;
  private deadline: number | null = null;
  private timerEl = h('div.plan-timer');
  private committed = false;
  private destroyed = false;
  private savedMissions: Orders['missions'] | null = null;

  constructor(root: HTMLElement, opts: PlanningOpts) {
    this.opts = opts;
    this.engine = opts.engine;
    this.seat = opts.seat;
    this.me = opts.factions[opts.seat];
    this.state = this.engine.parse(opts.state);
    this.orders = structuredClone(opts.initial);
    this.orders.seat = this.seat;
    this.el.style.setProperty('--fc', faction(this.me).color);
    const main = h('div.plan-main', this.mapWrap, this.side);
    this.mapWrap.append(this.hint, this.tip);
    this.el.append(this.top, main, this.tray);
    root.appendChild(this.el);
    if (opts.timer > 0) this.deadline = Date.now() + opts.timer * 1000;
  }

  async init() {
    this.map = new MapView(this.mapWrap, this.engine.content);
    await this.map.init();
    if (this.destroyed) return;
    this.map.setSeats(this.opts.factions);
    this.map.setState(this.state);
    this.map.setMarks(this.state.last, this.seat);
    this.map.layers.plan.addChild(this.planG, this.dragG);
    this.map.fit();
    this.bindMap();
    this.refresh();
    this.map.onTick(() => this.tickTimer());
  }

  destroy() {
    this.destroyed = true;
    this.map?.destroy();
    this.el.remove();
  }

  // --- Orders ---------------------------------------------------------------------------------

  squads(): Squadron[] {
    return this.state.squadrons.filter((s) => s.seat === this.seat);
  }

  squad(id: number): Squadron | undefined {
    return this.state.squadrons.find((s) => s.id === id);
  }

  mission(id: number): Mission {
    return this.orders.missions.find((m) => m.sq === id)?.mission ?? { t: 'Rest' };
  }

  setMission(sq: number, m: Mission) {
    this.orders.missions = this.orders.missions.filter((x) => x.sq !== sq);
    if (m.t !== 'Rest') this.orders.missions.push({ sq, mission: m });
    audio.play('click');
    this.refresh();
  }

  refresh() {
    this.checked = this.engine.check(this.opts.state, this.orders);
    this.renderTop();
    this.renderSide();
    this.renderTray();
    this.drawPlan();
    this.opts.coach?.step(this);
  }

  get supplyLeft(): number {
    return this.checked.preview.supply_left;
  }

  // --- Top bar ----------------------------------------------------------------------------------

  private renderTop() {
    const fd = faction(this.me);
    const me = this.state.seats[this.seat];
    const r = this.engine.content.rules;
    clear(this.top);
    const opens = opensTonight(this.state.night);
    this.top.append(
      h('div.top-who', svg(emblemSvg(this.me, { size: 34 }), 'top-emblem'), h('div', h('div.top-name', fd.short), h('div.top-sub', fd.leader))),
      h('div.top-night', h('div.top-big', `Night ${this.state.night}`), h('div.top-sub', `of ${r.nights}`)),
      ...(opens ? [h('div.top-news', opens)] : []),
      h('div.top-stat', { title: 'Supply pays for new squadrons and repairs.' }, svg(crateSvg(22)), h('span.top-big', String(this.supplyLeft)), h('span.top-sub', 'Supply')),
      h('div.top-stat', h('span.top-big', String(me.score)), h('span.top-sub', 'Points')),
      this.timerEl,
      h(
        'div.top-btns',
        h('button.btn', { onclick: () => this.suggest(), title: 'Fill in a sensible plan you can change' }, 'Suggest Plan'),
        h('button.btn', { onclick: () => this.clearAll() }, 'Clear'),
        h('button.btn', { onclick: () => rulesModal() }, 'Rules'),
      ),
    );
    this.tickTimer();
  }

  private tickTimer() {
    if (!this.deadline || this.committed) {
      this.timerEl.style.display = 'none';
      return;
    }
    const left = Math.max(0, Math.ceil((this.deadline - Date.now()) / 1000));
    this.timerEl.style.display = '';
    this.timerEl.textContent = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`;
    this.timerEl.classList.toggle('urgent', left <= 20);
    if (left <= 0) {
      toast('Time! Orders locked as they stand.');
      this.commit(true);
    }
  }

  // --- Squadron tray ------------------------------------------------------------------------------

  private renderTray() {
    clear(this.tray);
    const list = h('div.tray-list');
    for (const s of this.squads()) list.appendChild(this.card(s));
    for (const k of this.orders.builds) list.appendChild(h('div.sq-card.new', svg(factionPlaneSvg(this.me, k, 30)), h('div.sq-name', k === 'Fighter' ? 'New fighters' : 'New bombers'), h('div.sq-order', 'Ready next night')));
    const commit = h('button.commit', { onclick: () => this.commit(false) }, 'Commit orders');
    this.tray.append(list, commit);
  }

  private card(s: Squadron): HTMLElement {
    const m = this.mission(s.id);
    const atPark = this.orders.retreat;
    const chip =
      atPark ? h('span.chip.park', 'At the dog park') :
      m.t === 'Rest' ? h('span.chip.rest', s.planes < s.max ? 'Rest (+1 plane)' : 'Rest') :
      h(`span.chip.${m.t === 'Patrol' ? 'patrol' : m.target.t === 'Prize' ? 'prize' : 'strike'}`, missionLabel(m));
    const card = h(
      `div.sq-card${this.selected === s.id ? '.sel' : ''}${m.t === 'Rest' && !atPark ? '.idle' : ''}`,
      { 'data-sq': String(s.id), title: s.kind === 'Fighter' ? 'Drag onto your land to patrol, or onto a target to escort' : 'Drag onto a target to strike' },
      h('div.sq-icon', svg(factionPlaneSvg(this.me, s.kind, 34))),
      h('div.sq-body', h('div.sq-name', s.ace ? h('span.ace', '★ ') : null, squadronName(s, this.me)), h('div.sq-planes', { html: pipsHtml(s.planes, s.max) })),
      h('div.sq-order', chip, m.t !== 'Rest' && !atPark ? h('button.sq-x', { title: 'Rest instead', onclick: (e: Event) => { e.stopPropagation(); this.setMission(s.id, { t: 'Rest' }); } }, '×') : null),
    );
    card.addEventListener('pointerdown', (e) => this.cardDown(s.id, e));
    return card;
  }

  private cardDown(sq: number, e: PointerEvent) {
    if ((e.target as HTMLElement).closest('button')) return;
    if (this.orders.retreat) {
      toast('Everyone is at Project Bark Park tonight.');
      return;
    }
    e.preventDefault();
    const s = this.squad(sq)!;
    const ghost = h('div.drag-ghost', svg(factionPlaneSvg(this.me, s.kind, 40)));
    this.drag = { sq, ghost, x: e.clientX, y: e.clientY, moved: false };
    const move = (ev: PointerEvent) => {
      const d = this.drag;
      if (!d) return;
      if (!d.moved && Math.hypot(ev.clientX - d.x, ev.clientY - d.y) > 6) {
        d.moved = true;
        document.body.appendChild(d.ghost);
        this.select(sq);
      }
      if (d.moved) {
        d.ghost.style.transform = `translate(${ev.clientX - 20}px, ${ev.clientY - 20}px)`;
        const p = this.mapPoint(ev.clientX, ev.clientY);
        this.preview(sq, p);
      }
    };
    const up = (ev: PointerEvent) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      const d = this.drag;
      this.drag = null;
      if (!d) return;
      d.ghost.remove();
      this.dragG.clear();
      if (!d.moved) {
        this.select(this.selected === sq ? null : sq);
        return;
      }
      const p = this.mapPoint(ev.clientX, ev.clientY);
      if (p) this.dropAt(sq, p.sx, p.sy);
      this.select(null);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  select(sq: number | null) {
    this.selected = sq;
    this.renderTray();
    this.map.highlights.clear();
    if (sq !== null) {
      const s = this.squad(sq)!;
      this.highlightTargets(s);
      this.setHint(s.kind === 'Fighter' ? 'Drop on your land to patrol, or on a target to escort. Prizes are fair game.' : 'Drop on a red-ringed target to strike, or on a prize to claim it.');
    } else {
      this.setHint('');
    }
    this.map.drawHighlights();
  }

  private highlightTargets(s: Squadron) {
    const c = this.engine.content;
    c.sites.forEach((site, i) => {
      if (!this.state.sites[i].active) return;
      if (site.faction === this.me) {
        if (s.kind === 'Fighter') this.map.highlights.set(`s${i}`, GOOD);
      } else if (this.engine.siteError(this.opts.state, this.seat, i) === '') {
        this.map.highlights.set(`s${i}`, HIT_RED);
      }
    });
    c.prizes.forEach((_, i) => this.map.highlights.set(`p${i}`, INK));
  }

  private setHint(t: string) {
    this.hint.textContent = t;
    this.hint.style.display = t ? 'block' : 'none';
  }

  // --- Map ----------------------------------------------------------------------------------------

  private mapPoint(cx: number, cy: number): { sx: number; sy: number; p: P } | null {
    const r = this.mapWrap.getBoundingClientRect();
    if (cx < r.left || cx > r.right || cy < r.top || cy > r.bottom) return null;
    const sx = cx - r.left;
    const sy = cy - r.top;
    return { sx, sy, p: this.map.toMap(sx, sy) };
  }

  /** What dropping this squadron here would mean. */
  dropMission(sq: number, sx: number, sy: number): Drop {
    const s = this.squad(sq)!;
    const hit = this.map.pickAt(sx, sy);
    const p = this.map.toMap(sx, sy);
    if (hit && 'prize' in hit) return { mission: { t: 'Fly', target: { t: 'Prize', prize: hit.prize } } };
    if (hit && 'site' in hit) {
      const site = this.engine.content.sites[hit.site];
      if (site.faction === this.me) {
        if (s.kind === 'Bomber') return { error: "Bombers don't guard. Send fighters to patrol here." };
        return this.patrolAt(site.pos);
      }
      const err = this.engine.siteError(this.opts.state, this.seat, hit.site);
      if (err) return { error: err };
      return { mission: { t: 'Fly', target: { t: 'Site', site: hit.site } } };
    }
    if (s.kind === 'Bomber') return { error: 'Drop bombers right on a target.' };
    return this.patrolAt([Math.round(p[0]), Math.round(p[1])]);
  }

  private patrolAt(at: P): Drop {
    const err = this.engine.patrolError(this.opts.state, this.seat, at[0], at[1]);
    return err ? { error: err } : { mission: { t: 'Patrol', at: [Math.round(at[0]), Math.round(at[1])] } };
  }

  dropAt(sq: number, sx: number, sy: number) {
    const d = this.dropMission(sq, sx, sy);
    if ('error' in d) {
      toast(d.error);
      return;
    }
    this.setMission(sq, d.mission);
  }

  /** While dragging: the patrol circle or raid line this drop would make. */
  private preview(sq: number, at: { sx: number; sy: number; p: P } | null) {
    const g = this.dragG;
    g.clear();
    if (!at) return;
    const d = this.dropMission(sq, at.sx, at.sy);
    const color = factionColor(this.me);
    if ('error' in d) {
      dashedCircle(g, at.p[0], at.p[1], 10, 3, 2, HIT_RED, 0.9);
      return;
    }
    const m = d.mission;
    if (m.t === 'Patrol') {
      const r = this.radius();
      g.circle(m.at[0], m.at[1], r).fill({ color, alpha: 0.16 }).stroke({ width: 2.5, color, alpha: 0.95 });
    } else if (m.t === 'Fly') {
      drawRaidPath(g, this.airfield(), this.targetPos(m.target), color, 3, 0.7, SITE_R + 3);
    }
  }

  radius(): number {
    const r = this.engine.content.rules.patrol;
    const jewel = this.engine.content.sites.findIndex((s) => s.faction === this.me && s.kind === 'Jewel');
    const perk = this.me === 'FrontRange' && this.state.sites[jewel].health > 0;
    return perk ? r.mountain_radius : r.radius;
  }

  airfield(): P {
    return this.engine.content.sites.find((s) => s.faction === this.me && s.kind === 'Airfield')!.pos;
  }

  targetPos(t: Target): P {
    return t.t === 'Site' ? this.engine.content.sites[t.site].pos : this.engine.content.prizes[t.prize].pos;
  }

  /** Tonight's plan on the map: raid arrows, patrol circles, and the patrols each raid flies through. */
  drawPlan() {
    const g = this.planG;
    g.clear();
    this.map.removeLabels('plan-');
    const color = factionColor(this.me);
    const pv = this.checked.preview;
    for (const m of pv.orders.missions) {
      if (m.mission.t !== 'Patrol') continue;
      const at = m.mission.at;
      const r = this.radius();
      g.circle(at[0], at[1], r).fill({ color, alpha: 0.14 }).stroke({ width: 2.5, color, alpha: 0.95 });
      drawPlane(g, 'Fighter', at[0], at[1] - r, 6, 0, color);
    }
    pv.raids.forEach((raid, i) => {
      for (const c of raid.crosses) {
        const col = factionColor(this.map.ownerFaction(c.seat));
        dashedCircle(g, c.at[0], c.at[1], c.r, 5, 2.6, col, 0.95);
      }
      drawRaidPath(g, raid.from, raid.to, color, raid.bombers > 0 ? 4 : 2.5, 1, SITE_R + 4);
      const mid: P = [raid.from[0] + (raid.to[0] - raid.from[0]) * 0.55, raid.from[1] + (raid.to[1] - raid.from[1]) * 0.55];
      const parts: string[] = [];
      if (raid.bombers) parts.push(plural(raid.bombers, 'bomber'));
      if (raid.fighters) parts.push(plural(raid.fighters, 'fighter'));
      this.map.label(`plan-raid-${i}`, parts.join(' + '), mid, 'label-raid');
    });
  }

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
    let down: { x: number; y: number; moved: boolean } | null = null;
    canvas.addEventListener('pointerdown', (e) => {
      const r = canvas.getBoundingClientRect();
      pointers.set(e.pointerId, [e.clientX - r.left, e.clientY - r.top]);
      if (pointers.size === 2) {
        const [a, b] = [...pointers.values()];
        pinch = Math.hypot(a[0] - b[0], a[1] - b[1]);
        down = null;
        return;
      }
      canvas.setPointerCapture(e.pointerId);
      down = { x: e.clientX, y: e.clientY, moved: false };
    });
    canvas.addEventListener('pointermove', (e) => {
      const r = canvas.getBoundingClientRect();
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      if (pointers.has(e.pointerId)) pointers.set(e.pointerId, [x, y]);
      if (pinch !== null && pointers.size === 2) {
        const [a, b] = [...pointers.values()];
        const d = Math.hypot(a[0] - b[0], a[1] - b[1]);
        this.map.zoomAt((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, d / pinch);
        pinch = d;
        return;
      }
      if (down) {
        if (!down.moved && Math.hypot(e.clientX - down.x, e.clientY - down.y) > 5) down.moved = true;
        if (down.moved && this.selected === null) {
          this.map.panBy(e.movementX, e.movementY);
          return;
        }
      }
      if (this.selected !== null) this.preview(this.selected, { sx: x, sy: y, p: this.map.toMap(x, y) });
      this.hover(x, y);
    });
    const end = (e: PointerEvent) => {
      pointers.delete(e.pointerId);
      if (pointers.size < 2) pinch = null;
      const d = down;
      down = null;
      if (!d || d.moved) return;
      const r = canvas.getBoundingClientRect();
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      if (this.selected !== null) {
        this.dropAt(this.selected, x, y);
        this.dragG.clear();
        this.select(null);
      }
    };
    canvas.addEventListener('pointerup', end);
    canvas.addEventListener('pointercancel', (e) => {
      pointers.delete(e.pointerId);
      down = null;
    });
    canvas.addEventListener('pointerleave', () => (this.tip.style.display = 'none'));
    window.addEventListener('keydown', this.onKey);
  }

  private onKey = (e: KeyboardEvent) => {
    if (this.destroyed || !this.el.isConnected) {
      window.removeEventListener('keydown', this.onKey);
      return;
    }
    if (e.key === 'Escape' && this.selected !== null) {
      this.dragG.clear();
      this.select(null);
    }
  };

  /** A small card about the site or prize under the cursor. */
  private hover(x: number, y: number) {
    const hit = this.map.pickAt(x, y);
    if (!hit) {
      this.tip.style.display = 'none';
      return;
    }
    const c = this.engine.content;
    clear(this.tip);
    if ('site' in hit) {
      const s = c.sites[hit.site];
      const st = this.state.sites[hit.site];
      const mine = s.faction === this.me;
      const err = mine ? '' : this.engine.siteError(this.opts.state, this.seat, hit.site);
      this.tip.append(
        h('div.tip-head', svg(siteSvg(s.kind, FACTION_HEX[s.faction], 24, st.health <= 0)), h('div', h('b', s.name), h('div.tip-sub', `${faction(s.faction).short} ${KIND_NAME[s.kind]}`))),
        h('div.tip-pips', { html: pipsHtml(st.health, st.max) }),
        ...(err ? [h('div.tip-note', err)] : []),
      );
    } else {
      const p = c.prizes[hit.prize];
      const owner = this.state.prizes[hit.prize].owner;
      this.tip.append(h('b', p.name), h('div.tip-sub', owner === null ? 'Nobody holds it yet' : `Held by ${faction(this.opts.factions[owner]).short}`));
    }
    this.tip.style.display = 'block';
    const r = this.mapWrap.getBoundingClientRect();
    this.tip.style.left = `${Math.min(x + 16, r.width - 230)}px`;
    this.tip.style.top = `${Math.min(y + 16, r.height - 120)}px`;
  }

  // --- Side panel ------------------------------------------------------------------------------------

  private renderSide() {
    clear(this.side);
    const fd = faction(this.me);
    const tips = this.checked.tips.slice(0, 2);
    const worst = tips[0]?.level ?? 0;
    const mood = worst >= 3 ? 'alarmed' : tips.length === 0 ? 'happy' : 'neutral';
    this.side.append(
      h(
        'section.side-advisor',
        svg(leaderSvg(this.me, mood, { size: 76, bg: true }), 'adv-dog'),
        h('div.adv-tips', tips.length ? tips.map((t) => h(`div.adv-tip.l${t.level}`, t.text)) : h('div.adv-tip.l0', 'Looks good to me.')),
      ),
    );
    this.side.append(this.renderShop());
    const me = this.state.seats[this.seat];
    if (me.doghouse || this.orders.retreat) this.side.append(this.renderPark());
    if (me.fetch) this.side.append(this.renderFetch());
    this.side.append(h('section.side-perk', h('div.perk-name', fd.perk), h('div.perk-text', fd.perk_text)));
  }

  private renderShop(): HTMLElement {
    const c = this.engine.content;
    const rules = c.rules;
    const sec = h('section.side-shop', h('h3', 'Spend Supply'));
    const count = this.squads().length + this.orders.builds.length;
    const airfield = c.sites.findIndex((s) => s.faction === this.me && s.kind === 'Airfield');
    const airUp = this.state.sites[airfield].health > 0 || this.orders.repairs.includes(airfield);
    const cost = (k: UnitKind) => {
      const base = k === 'Fighter' ? rules.costs.fighter : rules.costs.bomber;
      const jewel = c.sites.findIndex((s) => s.faction === this.me && s.kind === 'Jewel');
      return this.me === 'Pacific' && this.state.sites[jewel].health > 0 ? Math.min(base, rules.costs.move_fast) : base;
    };
    const buy = (k: UnitKind) => {
      const n = cost(k);
      const why = !airUp ? 'Repair your Airfield first.' : count >= rules.max_squadrons ? `${rules.max_squadrons} squadrons is the most you can have.` : this.supplyLeft < n ? 'Not enough Supply.' : '';
      return h(
        'button.buy',
        { disabled: !!why, title: why || `Ready next night`, onclick: () => { this.orders.builds.push(k); audio.play('click'); this.refresh(); } },
        svg(factionPlaneSvg(this.me, k, 26)),
        h('span', k === 'Fighter' ? 'Fighters' : 'Bombers'),
        h('span.cost', String(n)),
      );
    };
    sec.append(h('div.buy-row', buy('Fighter'), buy('Bomber')));
    if (this.orders.builds.length) {
      sec.append(h('button.link', { onclick: () => { this.orders.builds.pop(); this.refresh(); } }, `Undo last build`));
    }
    // Repairs: one button per damaged site.
    const damaged = c.sites.map((s, i) => ({ s, i })).filter(({ s, i }) => s.faction === this.me && this.state.sites[i].health < this.state.sites[i].max);
    for (const { s, i } of damaged) {
      const st = this.state.sites[i];
      const queued = this.orders.repairs.filter((x) => x === i).length;
      const after = st.health + queued;
      sec.append(
        h(
          'div.repair-row',
          svg(siteSvg(s.kind, FACTION_HEX[this.me], 22, after <= 0)),
          h('span.repair-name', s.name),
          h('span', { html: pipsHtml(after, st.max) }),
          queued ? h('button.link', { onclick: () => { const k = this.orders.repairs.lastIndexOf(i); this.orders.repairs.splice(k, 1); this.refresh(); } }, 'Undo') : null,
          h('button.buy.small', { disabled: after >= st.max || this.supplyLeft < rules.costs.repair, onclick: () => { this.orders.repairs.push(i); audio.play('click'); this.refresh(); } }, 'Repair', h('span.cost', String(rules.costs.repair))),
        ),
      );
    }
    return sec;
  }

  private renderPark(): HTMLElement {
    const on = this.orders.retreat;
    return h(
      `section.side-park${on ? '.on' : ''}`,
      h('h3', 'Project Bark Park'),
      h('p', on ? 'Your squadrons rest at the secret dog park tonight. Nothing can touch your sites.' : 'Once a game: rest tonight where nothing can touch you, and come back refitted with the Big Fetch.'),
      h('button.btn.park-go', { onclick: () => this.setRetreat(!on) }, on ? 'Stay home instead' : 'Go to Project Bark Park'),
    );
  }

  private setRetreat(on: boolean) {
    if (on) {
      this.savedMissions = this.orders.missions;
      this.orders.missions = [];
      this.orders.retreat = true;
      audio.play('park');
    } else {
      this.orders.retreat = false;
      this.orders.missions = this.savedMissions ?? [];
    }
    this.refresh();
  }

  private renderFetch(): HTMLElement {
    const c = this.engine.content;
    const sel = h('select.fetch-pick') as HTMLSelectElement;
    sel.appendChild(h('option', { value: '' }, 'Keep it for later'));
    c.sites.forEach((s, i) => {
      if (!this.state.sites[i].active || s.faction === this.me) return;
      if (this.engine.siteError(this.opts.state, this.seat, i) !== '') return;
      const o = h('option', { value: String(i) }, `${s.name} (${faction(s.faction).short})`) as HTMLOptionElement;
      if (this.orders.fetch === i) o.selected = true;
      sel.appendChild(o);
    });
    sel.addEventListener('change', () => {
      this.orders.fetch = sel.value === '' ? null : Number(sel.value);
      this.refresh();
    });
    return h('section.side-fetch', h('h3', 'The Big Fetch'), h('p', `Knocks ${c.rules.bark_park.fetch_pips} pips off any open target. Nothing can stop it.`), sel);
  }

  // --- Buttons ------------------------------------------------------------------------------------

  suggest() {
    const o = this.engine.suggest(this.opts.state, this.seat, 'normal');
    this.orders = { ...o, seat: this.seat };
    toast('Suggested plan filled in. Change anything you like.');
    this.refresh();
  }

  private clearAll() {
    this.orders = { seat: this.seat, missions: [], builds: [], repairs: [], retreat: false, fetch: null };
    this.refresh();
  }

  commit(force: boolean) {
    if (this.committed) return;
    const idle = this.squads().filter((s) => this.mission(s.id).t === 'Rest' && s.planes === s.max).length;
    if (!force && !this.orders.retreat && idle === this.squads().length && this.squads().length > 0) {
      const close = modal(
        h(
          'div.confirm',
          h('h2', 'Everyone rests tonight?'),
          h('p', 'None of your squadrons have orders.'),
          h('div.confirm-btns', h('button.btn', { onclick: () => close() }, 'Keep planning'), h('button.commit', { onclick: () => { close(); this.commit(true); } }, 'Commit')),
        ),
      );
      return;
    }
    this.committed = true;
    audio.play('commit');
    this.opts.onCommit(structuredClone(this.checked.preview.orders));
  }
}

