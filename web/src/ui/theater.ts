// The Theater: the screen everyone watches. Big type, few words. Used full
// screen in its own window (laptop + TV) or as an overlay in the planning
// window. It never sees tonight's orders, only what's public.

import { MapView } from '../map/mapview';
import { ShowPlayer } from '../show/player';
import type { Caption } from '../show/summary';
import type { FinalData, FromTheater, MorningData, ShowData, ToTheater } from '../bus';
import type { Content, FactionId, GameState, SeatKind } from '../types';
import { faction, turns } from '../text';
import { leaderSvg, emblemSvg } from '../art/dogs';
import { h, clear, svg } from './dom';
import { audio } from '../audio';

const fcStyle = (f: FactionId) => ({ '--fc': faction(f).color }) as unknown as CSSStyleDeclaration;

/** A dog line, with Griffin & Pan's two-dog lines split into turns. */
export function spoken(line: string): HTMLElement[] {
  return turns(line).map((t) => h('div.say', t.who ? h('b', `${t.who}: `) : null, t.text));
}

export class Theater {
  readonly root: HTMLElement;
  readonly content: Content;
  private map: MapView | null = null;
  private send: (m: FromTheater) => void;
  private board = h('div.th-board');
  private center = h('div.th-center');
  private caption = h('div.th-caption');
  private hud = h('div.th-hud');
  private mapEl = h('div.th-map');
  private player: ShowPlayer | null = null;
  private factions: FactionId[] = [];
  private kinds: SeatKind[] = [];
  private scores: number[] = [];
  private state: GameState | null = null;
  private phase = 'idle';
  private showData: ShowData | null = null;
  private captionTimer: number | null = null;
  private ready: Promise<void>;
  private keyHandler = (e: KeyboardEvent) => this.onKey(e);

  constructor(root: HTMLElement, content: Content, send: (m: FromTheater) => void) {
    this.root = root;
    this.content = content;
    this.send = send;
    root.classList.add('theater');
    root.append(this.mapEl, this.board, this.center, this.caption, this.hud);
    this.map = new MapView(this.mapEl, content);
    this.map.insets = { top: 66, bottom: 6 };
    this.ready = this.map.init().then(() => this.map!.fit());
    window.addEventListener('keydown', this.keyHandler);
  }

  /** Stop drawing while hidden behind the planning window. */
  setVisible(v: boolean) {
    void this.ready.then(() => {
      if (!this.map) return;
      if (v) this.map.app.ticker.start();
      else this.map.app.ticker.stop();
    });
  }

  destroy() {
    window.removeEventListener('keydown', this.keyHandler);
    this.player?.destroy();
    this.map?.destroy();
  }

  private onKey(e: KeyboardEvent) {
    if (!this.root.isConnected || this.root.getClientRects().length === 0) return;
    if ((e.target as HTMLElement)?.tagName === 'INPUT') return;
    if (e.code === 'Space') {
      e.preventDefault();
      if (this.phase === 'show') this.control('pause');
      else if (this.phase === 'morning') this.send({ type: 'continue' });
    } else if (this.phase === 'show') {
      if (e.key === '1') this.control('speed', 1);
      if (e.key === '2') this.control('speed', 2);
      if (e.key === 's' || e.key === 'S') this.control('skip');
    }
  }

  async handle(m: ToTheater) {
    await this.ready;
    switch (m.type) {
      case 'idle':
        this.setPhase('idle');
        this.seat(m.factions, m.kinds, m.state);
        clear(this.center);
        if (m.text) this.center.appendChild(h('div.card.th-idle', h('div.th-title', m.text)));
        audio.music('theme');
        break;
      case 'morning':
        this.morning(m.data);
        break;
      case 'curtain':
        this.curtain(m);
        break;
      case 'show':
        this.show(m.data);
        break;
      case 'final':
        this.final(m.data);
        break;
      case 'control':
        this.control(m.action, m.value);
        break;
      case 'sound':
        audio.setEnabled(m.on);
        if (m.music !== undefined) audio.setMusic(m.music);
        break;
    }
  }

  private setPhase(p: string) {
    if (p !== 'show') {
      this.player?.destroy();
      this.player = null;
      this.setCaption(null);
      clear(this.hud);
    }
    this.phase = p;
    this.root.dataset.phase = p;
    this.status();
  }

  private status() {
    this.send({ type: 'status', phase: this.phase, paused: this.player?.paused ?? false, speed: this.player?.speed ?? 1, t: this.player?.t ?? 0 });
  }

  control(action: string, value?: number) {
    if (action === 'replay' && this.showData && this.phase !== 'show') {
      this.show(this.showData);
      return;
    }
    const p = this.player;
    if (!p || this.phase !== 'show') return;
    if (action === 'pause') p.togglePause();
    if (action === 'speed' && value) p.setSpeed(value);
    if (action === 'skip') p.skip();
    this.renderHud();
    this.status();
  }

  // --- Shared pieces ------------------------------------------------------------------------------

  private seat(factions: FactionId[], kinds: SeatKind[], state: GameState | null) {
    this.factions = factions;
    this.kinds = kinds;
    const map = this.map!;
    if (map.seatFactions.join() !== factions.join()) map.setSeats(factions);
    if (state) {
      this.state = state;
      map.setState(state);
      map.setMarks(state.last);
      this.scores = state.seats.map((s) => s.score);
    }
    this.renderBoard();
  }

  private renderBoard() {
    clear(this.board);
    this.board.classList.toggle('many', this.factions.length > 5);
    const best = Math.max(...this.scores);
    this.factions.forEach((f, i) => {
      const s = this.state?.seats[i];
      const tag = s?.fetch ? h('span.tag', 'BIG FETCH') : s?.doghouse && this.phase !== 'show' ? h('span.tag', 'CAN GO TO THE PARK') : null;
      this.board.appendChild(
        h(
          `div.score${this.scores[i] === best && best > 0 ? '.lead' : ''}`,
          { style: fcStyle(f), 'data-seat': String(i) },
          svg(emblemSvg(f, { size: 30 }), 'score-emblem'),
          h('div.score-name', faction(f).short, this.kinds[i] && this.kinds[i] !== 'human' ? h('span.bot', 'bot') : null),
          h('div.score-num', String(this.scores[i] ?? 0)),
          tag,
        ),
      );
    });
  }

  private bumpScore(seat: number, delta: number) {
    this.scores[seat] = (this.scores[seat] ?? 0) + delta;
    const el = this.board.querySelector(`[data-seat="${seat}"]`);
    if (!el) return;
    const num = el.querySelector('.score-num');
    if (num) num.textContent = String(this.scores[seat]);
    el.classList.remove('bump');
    void (el as HTMLElement).offsetWidth;
    el.classList.add('bump');
    const best = Math.max(...this.scores);
    this.board.querySelectorAll('.score').forEach((s, i) => s.classList.toggle('lead', this.scores[i] === best && best > 0));
  }

  private setCaption(c: Caption | null) {
    if (this.captionTimer !== null) clearTimeout(this.captionTimer);
    this.captionTimer = null;
    clear(this.caption);
    if (!c) return;
    const box = h(
      'div.cap',
      { style: fcStyle(c.faction) },
      h('div.cap-text', c.text),
      c.radio ? h('div.cap-radio', `“${c.radio}”`) : null,
    );
    this.caption.appendChild(box);
    if (c.radio) audio.play('squelch');
    this.captionTimer = window.setTimeout(() => box.classList.add('out'), 4200);
  }

  private renderHud() {
    clear(this.hud);
    if (this.phase !== 'show' || !this.player) return;
    const p = this.player;
    const btn = (label: string, on: () => void, active = false, title = '') => h(`button.th-btn${active ? '.on' : ''}`, { onclick: on, title }, label);
    this.hud.append(
      btn(p.paused ? '▶' : '❚❚', () => this.control('pause'), false, 'Pause (Space)'),
      btn('1×', () => this.control('speed', 1), p.speed === 1, 'Normal speed (1)'),
      btn('2×', () => this.control('speed', 2), p.speed === 2, 'Double speed (2)'),
      btn('Skip ⏭', () => this.control('skip'), false, 'Skip to the morning (S)'),
    );
  }

  // --- Morning -------------------------------------------------------------------------------------

  private morning(d: MorningData) {
    this.setPhase('morning');
    this.seat(d.factions, d.kinds, d.state);
    const home = this.map!.home();
    this.map!.focus(home.x, home.y, home.zoom);
    clear(this.center);
    const rows: (HTMLElement | null)[] = [];
    if (d.intro) {
      rows.push(
        h(
          'ol.steps',
          h('li', h('b', 'Plan'), h('span', 'Drag your squadrons onto the map, in secret.')),
          h('li', h('b', 'Watch'), h('span', 'Every plan plays out at once, on this screen.')),
          h('li', h('b', 'Score'), h('span', `Hits, shoot-downs and prizes. Most points after Night ${d.nights} wins.`)),
        ),
      );
    }
    if (d.report) {
      const r = d.report;
      if (r.highlights.length) rows.push(h('ul.highlights', ...r.highlights.slice(0, 3).map((x) => h('li', x))));
      const order = d.factions.map((_, i) => i).sort((a, b) => r.gained[b] - r.gained[a]);
      rows.push(
        h(
          'div.gains',
          ...order.map((i) => h(`div.gain${r.gained[i] > 0 ? '' : '.zero'}`, { style: fcStyle(d.factions[i]) }, h('span.gain-n', `+${r.gained[i]}`), h('span', faction(d.factions[i]).short))),
        ),
      );
    }
    if (d.news.length) rows.push(h('div.news', ...d.news.map((n) => h('div.news-item', n))));
    if (d.speaker) {
      const f = d.factions[d.speaker.seat];
      rows.push(
        h(
          'div.speaker',
          { style: fcStyle(f) },
          svg(leaderSvg(f, d.speaker.mood, { size: 104, bg: true }), 'speaker-dog'),
          h('div.bubble', h('div.bubble-who', faction(f).leader), ...spoken(d.speaker.line)),
        ),
      );
    }
    const headline = d.report?.headline ?? 'The West is up for grabs';
    this.center.appendChild(
      h(
        'div.card.morning',
        h('div.stamp', `Night ${d.night} of ${d.nights}`),
        h('h1.headline', headline),
        ...rows,
        h(
          'div.card-actions',
          this.showData && d.report ? h('button.th-btn', { onclick: () => this.control('replay') }, '↻ Watch again') : null,
          h('button.go', { onclick: () => this.send({ type: 'continue' }) }, `Plan Night ${d.night} ▸`),
        ),
      ),
    );
    audio.music('briefing');
  }

  // --- Curtain (hotseat) -------------------------------------------------------------------------------

  private curtain(m: Extract<ToTheater, { type: 'curtain' }>) {
    this.setPhase('curtain');
    audio.music('briefing');
    this.seat(m.factions, m.kinds, m.state);
    clear(this.center);
    const fd = faction(m.faction);
    const timer = h('div.timer');
    this.center.appendChild(
      h(
        'div.card.curtain',
        { style: fcStyle(m.faction) },
        svg(leaderSvg(m.faction, 'neutral', { size: 150, bg: true }), 'curtain-dog'),
        h('div', h('div.stamp', `${fd.short} is planning`), m.waiting.length ? h('div.waiting', `Then: ${m.waiting.map((f) => faction(f).short).join(', ')}`) : null, timer),
      ),
    );
    if (m.deadline) {
      const tick = () => {
        if (this.phase !== 'curtain' || !timer.isConnected) return;
        const left = Math.max(0, Math.ceil((m.deadline! - Date.now()) / 1000));
        timer.textContent = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`;
        if (left > 0) setTimeout(tick, 500);
      };
      tick();
    }
  }

  // --- The show -------------------------------------------------------------------------------------------

  private show(d: ShowData) {
    this.setPhase('show');
    this.showData = d;
    this.seat(d.factions, d.kinds, d.before);
    this.renderBoard();
    clear(this.center);
    const title = h('div.title-stamp', `Night ${d.night}`);
    this.center.appendChild(title);
    audio.play('sting');
    audio.music(d.night >= this.content.rules.open_hqs ? 'glovesoff' : 'sortie');
    setTimeout(() => title.classList.add('out'), 1300);
    setTimeout(() => {
      title.remove();
      if (this.phase === 'show' && this.showData === d) this.startPlayer(d);
    }, 1700);
  }

  private startPlayer(d: ShowData) {
    const map = this.map!;
    this.player?.destroy();
    this.player = new ShowPlayer(map, this.content, d.factions, d.events, d.before, d.after, d.captions, {
      caption: (c) => this.setCaption(c),
      score: (seat, delta) => this.bumpScore(seat, delta),
      sound: (k) => audio.play(k),
      ended: () => this.afterShow(d),
    });
    this.renderHud();
    this.status();
  }

  private afterShow(d: ShowData) {
    const next = d.next;
    if (next.type === 'morning') this.morning(next.data);
    else if (next.type === 'final') this.final(next.data);
    else {
      this.setPhase('morning');
      this.seat(d.factions, d.kinds, d.after);
      clear(this.center);
      this.center.appendChild(h('div.card.morning', h('div.stamp', `Night ${d.night}`), h('div.card-actions', h('button.th-btn', { onclick: () => this.control('replay') }, '↻ Watch again'), h('button.go', { onclick: () => this.send({ type: 'continue' }) }, 'Back ▸'))));
      audio.play('report');
    }
  }

  // --- Final results ----------------------------------------------------------------------------------------

  private final(d: FinalData) {
    this.setPhase('final');
    audio.music('theme');
    this.seat(d.factions, d.kinds, d.state);
    clear(this.center);
    const scores = d.state.seats.map((s) => s.score);
    const best = Math.max(...scores);
    const winners = d.factions.filter((_, i) => scores[i] === best);
    const w = winners[0];
    const order = d.factions.map((_, i) => i).sort((a, b) => scores[b] - scores[a]);
    this.center.appendChild(
      h(
        'div.card.final',
        { style: fcStyle(w) },
        h('div.stamp', winners.length > 1 ? 'Shared Bragging Rights' : 'Bragging Rights'),
        h('h1.headline', d.headline),
        h(
          'div.winner',
          svg(leaderSvg(w, 'happy', { size: 170, bg: true }), 'winner-dog'),
          h('div', h('div.winner-name', winners.map((f) => faction(f).name).join(' & ')), d.line ? h('div.bubble', ...spoken(d.line)) : null),
        ),
        h(
          'div.final-body',
          h(
            'ol.standings',
            ...order.map((i) => {
              const s = d.state.seats[i];
              return h('li', { style: fcStyle(s.faction) }, svg(emblemSvg(s.faction, { size: 26 }), 'chip-emblem'), h('span.st-name', faction(s.faction).short), h('span.st-score', String(s.score)));
            }),
          ),
          svg(raceChart(d), 'race'),
        ),
        h('div.card-actions', h('button.go', { onclick: () => this.send({ type: 'continue' }) }, 'Back to the menu')),
      ),
    );
    audio.play('fanfare');
  }
}

/** The score after each night, one line per faction. */
function raceChart(d: FinalData): string {
  const hist = [d.factions.map(() => 0), ...d.state.history];
  const W = 520;
  const H = 200;
  const pad = 26;
  const padR = 64;
  const max = Math.max(10, ...hist.flat());
  const n = Math.max(1, hist.length - 1);
  const x = (i: number) => pad + (i / n) * (W - pad - padR);
  const y = (v: number) => H - pad - (v / max) * (H - pad * 2);
  let out = `<svg viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">`;
  for (let i = 0; i <= n; i++) {
    out += `<line x1="${x(i)}" y1="${pad}" x2="${x(i)}" y2="${H - pad}" stroke="#231e1a" stroke-opacity="0.12" stroke-width="1"/>`;
    out += `<text x="${x(i)}" y="${H - 8}" fill="#51463c" font-size="11" text-anchor="middle">${i === 0 ? '' : i}</text>`;
  }
  d.factions.forEach((f, s) => {
    const pts = hist.map((r, i) => `${x(i).toFixed(1)},${y(r[s] ?? 0).toFixed(1)}`).join(' ');
    out += `<polyline points="${pts}" fill="none" stroke="#231e1a" stroke-width="5" stroke-linejoin="round" stroke-linecap="round"/>`;
    out += `<polyline points="${pts}" fill="none" stroke="${faction(f).color}" stroke-width="3" stroke-linejoin="round" stroke-linecap="round"/>`;
  });
  const last = hist[hist.length - 1];
  const labels = d.factions.map((f, s) => ({ f, y: y(last[s] ?? 0) + 4 })).sort((a, b) => a.y - b.y);
  for (let i = 1; i < labels.length; i++) labels[i].y = Math.max(labels[i].y, labels[i - 1].y + 13);
  for (const l of labels) out += `<text x="${W - padR + 6}" y="${l.y.toFixed(1)}" fill="#231e1a" font-size="12" font-weight="700">${faction(l.f).short.replace(/&/g, '&amp;')}</text>`;
  return out + '</svg>';
}

