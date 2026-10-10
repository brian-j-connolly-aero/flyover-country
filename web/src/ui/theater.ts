// The Theater: a ten-foot interface for the TV across the room. It only ever
// sees public information. Used full-screen in its own window (two-window
// mode) or as an overlay in the planning window (single-screen mode).

import { MapView } from '../map/mapview';
import { ShowPlayer } from '../show/player';
import { summarize, finalHeadline, type NightSummary } from '../show/summary';
import type { DawnData, FinalData, FromTheater, ShowData, ToTheater } from '../bus';
import type { Content, FactionId, PublicSeat, PublicView, SeatKind } from '../types';
import { any, esc, faction, leader, patchText, pick, stageBlurb, stageName, turns, effectText } from '../text';
import { leaderSvg, emblemSvg } from '../art/dogs';
import { h, clear, svg } from './dom';
import { audio } from '../audio';

export class Theater {
  readonly root: HTMLElement;
  readonly content: Content;
  private map: MapView | null = null;
  private send: (m: FromTheater) => void;
  private board = h('div.th-board');
  private center = h('div.th-center');
  private subs = h('div.th-subs');
  private banner = h('div.th-banner');
  private hud = h('div.th-hud');
  private mapEl = h('div.th-map');
  private player: ShowPlayer | null = null;
  private factions: FactionId[] = [];
  private kinds: SeatKind[] = [];
  private scores: number[] = [];
  private seatInfo: PublicSeat[] = [];
  /** Seats at Project Bark Park during tonight's show. */
  private atPark = new Set<number>();
  private phase = 'idle';
  private chatterQueue: { who: string; color: string; text: string; pri: number; at: number }[] = [];
  private lastLine = 0;
  private fillerTimer: number | null = null;
  private summary: NightSummary | null = null;
  private showData: ShowData | null = null;
  private ready: Promise<void>;
  private keyHandler = (e: KeyboardEvent) => this.onKey(e);

  constructor(root: HTMLElement, content: Content, send: (m: FromTheater) => void) {
    this.root = root;
    this.content = content;
    this.send = send;
    root.classList.add('theater');
    root.append(this.mapEl, this.board, this.center, this.subs, this.banner, this.hud);
    this.map = new MapView(this.mapEl, content);
    this.ready = this.map.init().then(() => this.map!.fit());
    window.addEventListener('keydown', this.keyHandler);
    setInterval(() => this.pumpChatter(), 250);
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
      else if (this.phase === 'report' || this.phase === 'dawn') this.send({ type: 'continue' });
    } else if (this.phase === 'show') {
      if (e.key === '1') this.control('speed', 1);
      if (e.key === '2') this.control('speed', 2);
      if (e.key === '4') this.control('speed', 4);
      if (e.key === 's' || e.key === 'S') this.control('skip');
    } else if (this.phase === 'report' && (e.key === 'r' || e.key === 'R')) {
      this.control('replay');
    }
  }

  async handle(m: ToTheater) {
    await this.ready;
    switch (m.type) {
      case 'idle':
        this.setPhase('idle');
        this.factions = m.factions;
        this.kinds = m.kinds;
        if (m.pv) this.applyPublic(m.pv);
        clear(this.center);
        if (m.text) this.center.appendChild(h('div.th-card.th-idle', h('div.th-title', m.text)));
        audio.music('theme');
        break;
      case 'dawn':
        this.dawn(m.data);
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
      this.chatterQueue = [];
      clear(this.subs);
    }
    this.phase = p;
    this.root.dataset.phase = p;
    if (this.fillerTimer !== null) {
      clearInterval(this.fillerTimer);
      this.fillerTimer = null;
    }
    this.status();
  }

  private status() {
    this.send({ type: 'status', phase: this.phase, paused: this.player?.paused ?? false, speed: this.player?.speed ?? 1, t: this.player?.t ?? 0 });
  }

  control(action: string, value?: number) {
    const p = this.player;
    if (action === 'replay' && this.phase === 'report') {
      this.playOfTheNight();
      return;
    }
    if (!p || this.phase !== 'show') return;
    if (action === 'pause') p.togglePause();
    if (action === 'speed' && value) p.setSpeed(value);
    if (action === 'skip') p.skip();
    this.renderHud();
    this.status();
  }

  // --- Shared pieces --------------------------------------------------------------------------

  private applyPublic(pv: PublicView) {
    const map = this.map!;
    if (map.seatFactions.join() !== this.factions.join()) map.setSeats(this.factions);
    map.setNodes(pv.nodes.map((n) => ({ owner: n.owner, health: n.health, active: n.active })));
    map.setFlows(null, null);
    this.scores = pv.seats.map((s) => s.score);
    this.seatInfo = pv.seats;
    this.renderBoard();
    this.labelCapitals();
  }

  private labelCapitals() {
    const map = this.map!;
    for (const n of this.content.nodes) {
      if (n.kind === 'CrownJewel' || n.kind === 'Boneyard' || n.kind === 'MassDriver') {
        map.label(`node-${n.idx}`, n.name, [n.pos[0], n.pos[1] + 12], 'label-node');
      }
    }
  }

  private renderBoard() {
    clear(this.board);
    const order = this.factions.map((_, i) => i);
    const best = Math.max(...this.scores);
    for (const i of order) {
      const f = this.factions[i];
      const fd = faction(f);
      const leaderClass = this.scores[i] === best && best > 0 ? ' leader' : '';
      this.board.appendChild(
        h(
          `div.th-score${leaderClass}`,
          { style: { '--fc': fd.color } as unknown as CSSStyleDeclaration, 'data-seat': String(i) },
          svg(emblemSvg(f, { size: 40 }), 'th-emblem'),
          h('div.th-score-name', fd.short, this.kinds[i] && this.kinds[i] !== 'human' ? h('span.bot', 'BOT') : null),
          h('div.th-score-num', String(this.scores[i] ?? 0)),
          this.tags(i),
        ),
      );
    }
  }

  /** Little tags under a score: scrappy, in the doghouse, at the park, Big Fetch loaded. */
  private tags(i: number): HTMLElement | null {
    const s = this.seatInfo[i];
    const out: HTMLElement[] = [];
    if (this.atPark.has(i)) out.push(h('span.th-tag.park', 'AT THE PARK'));
    else if (s?.doghouse && this.phase !== 'show') out.push(h('span.th-tag.dog', 'DOGHOUSE'));
    if (s?.fetch) out.push(h('span.th-tag.fetch', 'BIG FETCH'));
    if (s?.scrappy) out.push(h('span.th-tag', 'SCRAPPY'));
    return out.length ? h('div.th-tags', ...out) : null;
  }

  private bumpScore(seat: number, delta: number) {
    this.scores[seat] = (this.scores[seat] ?? 0) + delta;
    const el = this.board.querySelector(`[data-seat="${seat}"]`);
    if (!el) return;
    const num = el.querySelector('.th-score-num');
    if (num) num.textContent = String(this.scores[seat]);
    const pop = h('div.th-pop', `+${delta}`);
    el.appendChild(pop);
    setTimeout(() => pop.remove(), 1600);
    const best = Math.max(...this.scores);
    this.board.querySelectorAll('.th-score').forEach((s, i) => s.classList.toggle('leader', this.scores[i] === best && best > 0));
  }

  private queueChatter(who: string, color: string, text: string, pri: number) {
    this.chatterQueue.push({ who, color, text, pri, at: performance.now() });
    if (this.chatterQueue.length > 6) {
      this.chatterQueue.sort((a, b) => b.pri - a.pri || a.at - b.at);
      this.chatterQueue.length = 6;
    }
  }

  /** At most one radio line every four seconds, so the screen never becomes a chat log. */
  private pumpChatter() {
    const now = performance.now();
    if (now - this.lastLine < 4000 || this.chatterQueue.length === 0) return;
    this.chatterQueue = this.chatterQueue.filter((c) => now - c.at < 7000 || c.pri >= 4);
    if (!this.chatterQueue.length) return;
    this.chatterQueue.sort((a, b) => b.pri - a.pri || a.at - b.at);
    const c = this.chatterQueue.shift()!;
    this.lastLine = now;
    this.subtitle(c.who, c.color, c.text);
  }

  private subtitle(who: string, color: string, text: string) {
    clear(this.subs);
    // Mini-dialogues ("Griffin: ... | Pan: ...") name each speaker in turn.
    const parts = turns(text);
    const line = parts.some((t) => t.who)
      ? h('div.th-sub', ...parts.flatMap((t, i) => [i ? ' ' : '', h('span.th-sub-who', { style: { color } }, (t.who ?? who) + ':'), ' ', t.text]))
      : h('div.th-sub', h('span.th-sub-who', { style: { color } }, who + ':'), ' ', text);
    this.subs.appendChild(line);
    audio.play('squelch');
    setTimeout(() => line.classList.add('out'), 3600);
  }

  private flashBanner(title: string, sub: string, kind: string) {
    clear(this.banner);
    const b = h(`div.th-banner-box.${kind}`, h('div.th-banner-title', title), h('div.th-banner-sub', sub));
    this.banner.appendChild(b);
    setTimeout(() => b.classList.add('out'), 2600);
    setTimeout(() => b.remove(), 3300);
  }

  private renderHud() {
    clear(this.hud);
    if (this.phase !== 'show' || !this.player) return;
    const p = this.player;
    const btn = (label: string, on: () => void, active = false) => h(`button.th-btn${active ? '.on' : ''}`, { onclick: on }, label);
    this.hud.append(
      btn(p.paused ? '▶ Play' : '❚❚ Pause', () => this.control('pause')),
      btn('1×', () => this.control('speed', 1), p.speed === 1),
      btn('2×', () => this.control('speed', 2), p.speed === 2),
      btn('4×', () => this.control('speed', 4), p.speed === 4),
      btn('Skip to report ⏭', () => this.control('skip')),
      h('span.th-hint', 'Space pauses · 1/2/4 speed · S skips'),
    );
  }

  // --- Dawn briefing ----------------------------------------------------------------------------

  private dawn(d: DawnData) {
    this.setPhase('dawn');
    this.factions = d.factions;
    this.kinds = d.kinds;
    this.atPark.clear();
    this.applyPublic(d.pv);
    this.map!.focus(this.content.width / 2 + 10, this.content.height / 2, 1);
    clear(this.center);
    const pv = d.pv;
    const patch = patchText(pv.patch);
    const effects = pv.effects.filter((e) => e.from <= d.night && d.night <= e.until);
    const cards: HTMLElement[] = [];
    if (patch && this.factions.includes('Pacific')) {
      cards.push(h('div.th-mini', { style: { '--fc': faction('Pacific').color } as unknown as CSSStyleDeclaration }, h('div.th-mini-head', `PATCH NOTES ${patch.version}`), h('div', patch.text)));
    }
    if (pv.harvest > 0) {
      cards.push(h('div.th-mini', { style: { '--fc': faction('GrainBelt').color } as unknown as CSSStyleDeclaration }, h('div.th-mini-head', 'HARVEST SHIPMENT'), h('div', `Worth ${pv.harvest} Supply. Bid in secret tonight.`)));
    }
    if (d.stage === 2 && pv.mass_driver_holder !== null && !pv.mass_driver_fired) {
      cards.push(h('div.th-mini', h('div.th-mini-head', 'WHITE SANDS MASS DRIVER'), h('div', `${faction(this.factions[pv.mass_driver_holder]).short} holds it: one unstoppable shot, once per game.`)));
    }
    for (const e of effects) {
      cards.push(h('div.th-mini.warn', h('div.th-mini-head', faction(this.factions[e.seat]).short.toUpperCase()), h('div', effectText(e.kind))));
    }
    const names = (seats: PublicSeat[]) => seats.map((s) => faction(s.faction).short).join(' and ');
    const dog = pv.seats.filter((s) => s.doghouse);
    if (dog.length) {
      cards.push(h('div.th-mini.park', h('div.th-mini-head', 'PROJECT BARK PARK IS OPEN'), h('div', `${names(dog)} ${dog.length > 1 ? 'are' : 'is'} in the doghouse and may spend tonight at the park, untouchable.`)));
    }
    const balls = pv.seats.filter((s) => s.fetch);
    if (balls.length) {
      cards.push(h('div.th-mini.fetch', h('div.th-mini-head', 'BIG FETCH LOADED'), h('div', `${names(balls)} ${balls.length > 1 ? 'have' : 'has'} one stadium-sized tennis ball to throw.`)));
    }
    const story = d.intro ? null : pick(this.content.flavor.sidebar, d.night * 7919 + pv.seats.reduce((a, s) => a + s.score, 0));
    if (story) {
      const [tag, ...rest] = story.split(': ');
      cards.push(h('div.th-mini.sidebar', h('div.th-mini-head', `ALSO IN THE SENTINEL · ${rest.length ? tag : 'NEWS'}`), h('div', rest.length ? rest.join(': ') : story)));
    }
    const quips = this.factions.map((f, i) => {
      const L = leader(f);
      const winning = this.scores[i] === Math.max(...this.scores) && this.scores[i] > 0;
      const losing = this.scores[i] === Math.min(...this.scores) && Math.max(...this.scores) > 0;
      const back = d.returned?.includes(i) && L.park_return?.length;
      const list = d.intro ? L.pick : back ? L.park_return : winning ? L.winning : losing ? L.losing : L.dawn;
      const line = any(list) ?? '';
      return h(
        'div.th-quip',
        { style: { '--fc': faction(f).color } as unknown as CSSStyleDeclaration },
        svg(leaderSvg(f, winning || back ? 'happy' : 'neutral', { size: 96, bg: true }), 'th-quip-dog'),
        h('div.th-quip-text', h('div.th-quip-name', L.name), ...turns(line).map((t) => h('div', t.who ? h('b', t.who + ': ') : null, t.text))),
      );
    });
    this.center.appendChild(
      h(
        'div.th-card.th-dawn',
        h('div.th-paper', h('div.th-paper-mast', 'THE WESTERN SENTINEL'), h('div.th-paper-date', `Dawn before Night ${d.night} of ${d.nights}`), h('div.th-headline', d.headline)),
        h('div.th-stage', h('span.th-stage-name', `NIGHT ${d.night}: ${stageName(d.stage).toUpperCase()}`), h('span.th-stage-blurb', stageBlurb(d.stage))),
        d.highlights.length ? h('ul.th-highlights', ...d.highlights.map((x) => h('li', x))) : null,
        cards.length ? h('div.th-minis', ...cards) : null,
        h('div.th-quips', ...quips),
        h('button.th-continue', { onclick: () => this.send({ type: 'continue' }) }, 'Start planning ▸'),
      ),
    );
    audio.music('briefing');
  }

  // --- Curtain ------------------------------------------------------------------------------------

  private curtain(m: Extract<ToTheater, { type: 'curtain' }>) {
    this.setPhase('curtain');
    audio.music('briefing');
    this.factions = m.factions;
    this.kinds = m.kinds;
    this.applyPublic(m.pv);
    this.map!.focus(this.content.width / 2 + 10, this.content.height / 2, 1);
    clear(this.center);
    const fd = faction(m.faction);
    const filler = h('div.th-filler');
    const timer = h('div.th-timer');
    const setFiller = () => {
      const line = any(this.content.flavor.curtain[m.faction]) ?? `${fd.short} is planning...`;
      filler.textContent = line;
    };
    setFiller();
    this.center.appendChild(
      h(
        'div.th-card.th-curtain',
        { style: { '--fc': fd.color } as unknown as CSSStyleDeclaration },
        svg(leaderSvg(m.faction, 'neutral', { size: 160, bg: true }), 'th-curtain-dog'),
        h('div.th-curtain-text', h('div.th-pass', `${fd.short.toUpperCase()} IS PLANNING`), h('div.th-eyes', 'Everyone else, eyes on the TV.'), filler, timer),
        m.waiting.length ? h('div.th-waiting', 'Still to plan: ', m.waiting.map((f) => faction(f).short).join(', ')) : null,
      ),
    );
    this.fillerTimer = window.setInterval(() => {
      setFiller();
      if (m.deadline) {
        const left = Math.max(0, Math.ceil((m.deadline - Date.now()) / 1000));
        timer.textContent = `${left}s`;
      }
    }, 5000);
    if (m.deadline) {
      const tick = () => {
        if (this.phase !== 'curtain') return;
        const left = Math.max(0, Math.ceil((m.deadline! - Date.now()) / 1000));
        timer.textContent = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`;
        if (left > 0) setTimeout(tick, 500);
      };
      tick();
    }
  }

  // --- The show -------------------------------------------------------------------------------------

  private show(d: ShowData) {
    this.setPhase('show');
    this.factions = d.factions;
    this.kinds = d.kinds;
    this.showData = d;
    this.atPark = new Set(d.events.flatMap((r) => (r.e.t === 'BarkPark' ? [r.e.seat] : [])));
    this.applyPublic(d.before);
    this.player?.destroy();
    clear(this.center);
    this.chatterQueue = [];
    const title = h(
      'div.th-titlecard',
      h('div.th-title-night', `NIGHT ${d.night}: ${stageName(d.before.stage).toUpperCase()}`),
    );
    const ns = d.events.find((e) => e.e.t === 'NightStart')?.e;
    if (ns && ns.t === 'NightStart') {
      const patch = patchText(ns.patch);
      if (patch && this.factions.includes('Pacific')) title.appendChild(h('div.th-title-sub', `Patch Notes ${patch.version}: ${patch.text}`));
      if (ns.harvest_winner !== null) title.appendChild(h('div.th-title-sub', `Harvest Shipment: ${faction(this.factions[ns.harvest_winner]).short} wins with ${ns.harvest_bid} Supply`));
      if (d.night === d.before.nights) title.appendChild(h('div.th-title-sub', 'Final night: strike scores double.'));
    }
    this.center.appendChild(title);
    audio.play('sting');
    audio.music(d.before.stage >= 2 ? 'glovesoff' : 'sortie');
    setTimeout(() => title.classList.add('out'), 3400);
    setTimeout(() => {
      title.remove();
      this.startPlayer(d);
    }, 3900);
  }

  private startPlayer(d: ShowData) {
    const map = this.map!;
    this.player = new ShowPlayer(map, this.content, d.factions, d.events, d.before, d.after, {
      chatter: (who, color, text, pri) => this.queueChatter(who, color, text, pri),
      banner: (t, s, k) => this.flashBanner(t, s, k),
      score: (seat, delta) => this.bumpScore(seat, delta),
      sound: (k) => audio.play(k),
      ended: () => this.report(),
    });
    this.renderHud();
    this.status();
  }

  // --- Morning report -------------------------------------------------------------------------------

  private report() {
    const d = this.showData!;
    this.setPhase('report');
    this.chatterQueue = [];
    clear(this.subs);
    clear(this.hud);
    this.atPark.clear();
    this.applyPublic(d.after);
    this.map!.focus(this.content.width / 2 + 10, this.content.height / 2, 1);
    const s = summarize(this.content, d.events, d.factions, d.night, d.night * 977);
    this.summary = s;
    clear(this.center);
    const order = d.factions.map((_, i) => i).sort((a, b) => s.gained[b] - s.gained[a]);
    const cards = order.map((i) => {
      const f = d.factions[i];
      const fd = faction(f);
      const mood = s.jewelHit[i] ? 'alarmed' : s.gained[i] === Math.max(...s.gained) && s.gained[i] > 0 ? 'happy' : 'neutral';
      const L = leader(f);
      const quip = s.jewelHit[i] ? any(L.our_jewel_hit) : null;
      return h(
        'div.th-rep',
        { style: { '--fc': fd.color } as unknown as CSSStyleDeclaration },
        svg(leaderSvg(f, mood, { size: 110, bg: true }), 'th-rep-dog'),
        h('div.th-rep-name', fd.short),
        h('div.th-rep-gain', `+${s.gained[i]}`, h('span', ' Bragging Rights')),
        h(
          'div.th-rep-stats',
          h('div', `Squadrons lost: ${s.squadronsLost[i]}`),
          h('div', `Shot down: ${s.kills[i]}`),
          h('div', `Damage dealt / taken: ${s.stepsDealt[i]} / ${s.stepsTaken[i]}`),
          s.park[i] ? h('div.th-rep-park', 'Spent the night at Project Bark Park') : null,
        ),
        quip ? h('div.th-rep-quip', ...turns(quip).map((t) => h('div', t.who ? h('b', t.who + ': ') : null, t.text))) : null,
      );
    });
    const potn = s.play
      ? h('button.th-btn', { onclick: () => this.playOfTheNight() }, `▶ Play of the Night: ${s.play.label}`)
      : null;
    this.center.appendChild(
      h(
        'div.th-card.th-report',
        h('div.th-paper', h('div.th-paper-mast', 'MORNING REPORT'), h('div.th-paper-date', `After Night ${d.night}`), h('div.th-headline', s.headline)),
        h('div.th-reps', ...cards),
        s.highlights.length ? h('ul.th-highlights', ...s.highlights.map((x) => h('li', x))) : null,
        h('div.th-report-actions', potn, h('button.th-btn', { onclick: () => this.show(d) }, '↻ Replay the night'), h('button.th-continue', { onclick: () => this.send({ type: 'continue' }) }, d.last ? 'Final results ▸' : `Dawn of Night ${d.night + 1} ▸`)),
      ),
    );
    audio.play('report');
    audio.music('briefing');
  }

  private playOfTheNight() {
    const s = this.summary;
    if (!s?.play || !this.player) return;
    const card = this.center.querySelector('.th-report') as HTMLElement | null;
    card?.classList.add('dim');
    this.flashBanner('PLAY OF THE NIGHT', s.play.label.toUpperCase(), 'title');
    this.player.playRange(s.play, () => {
      card?.classList.remove('dim');
      this.map!.focus(this.content.width / 2 + 10, this.content.height / 2, 1);
    });
  }

  // --- Final screen ----------------------------------------------------------------------------------

  private final(d: FinalData) {
    this.setPhase('final');
    audio.music('theme');
    this.factions = d.factions;
    this.kinds = d.kinds;
    this.applyPublic(d.pv);
    clear(this.center);
    const scores = d.pv.seats.map((s) => s.score);
    const best = Math.max(...scores);
    const winners = d.factions.filter((_, i) => scores[i] === best);
    const w = winners[0];
    const L = leader(w);
    const order = d.factions.map((_, i) => i).sort((a, b) => scores[b] - scores[a]);
    this.center.appendChild(
      h(
        'div.th-card.th-final',
        { style: { '--fc': faction(w).color } as unknown as CSSStyleDeclaration },
        h('div.th-paper', h('div.th-paper-mast', 'THE WESTERN SENTINEL — FINAL EDITION'), h('div.th-headline', finalHeadline(d.factions, scores, 4242))),
        h(
          'div.th-winner',
          svg(leaderSvg(w, 'happy', { size: 200, bg: true }), 'th-winner-dog'),
          h(
            'div',
            h('div.th-winner-title', winners.length > 1 ? 'SHARED BRAGGING RIGHTS' : 'BRAGGING RIGHTS'),
            h('div.th-winner-name', winners.map((f) => faction(f).name).join(' & ')),
            h('div.th-winner-quip', ...turns(any(L.victory) ?? '').map((t) => h('div', t.who ? h('b', t.who + ': ') : null, t.text))),
          ),
        ),
        h(
          'div.th-final-grid',
          h(
          'div.th-final-table',
          h('div.th-final-row.head', h('span', 'Faction'), h('span', 'Bragging Rights'), h('span', 'Pride Lost'), h('span', 'Shot down'), h('span', 'Planes lost')),
          ...order.map((i) => {
            const s = d.pv.seats[i];
            return h(
              'div.th-final-row',
              { style: { '--fc': faction(s.faction).color } as unknown as CSSStyleDeclaration },
              h('span', faction(s.faction).short),
              h('span.big', String(s.score)),
              h('span', String(s.pride_lost)),
              h('span', String(s.kills)),
              h('span', String(s.planes_lost)),
            );
          }),
        ),
        h('div.th-race-box', h('div.th-mini-head', 'THE SCORE RACE'), svg(raceChart(d), 'th-race'))),
        h('div.th-losers', ...order.slice(1).map((i) => {
          const f = d.factions[i];
          return h('div.th-quip', { style: { '--fc': faction(f).color } as unknown as CSSStyleDeclaration }, svg(leaderSvg(f, 'neutral', { size: 64, bg: true }), 'th-quip-dog'), h('div.th-quip-text', ...turns(any(leader(f).defeat) ?? '').map((t) => h('div', t.who ? h('b', t.who + ': ') : null, t.text))));
        })),
      ),
    );
    audio.play('fanfare');
  }
}

/** The night-by-night score race, DEFCON style. */
function raceChart(d: FinalData): string {
  const hist = d.pv.history;
  const W = 640;
  const H = 220;
  const pad = 30;
  const padR = 70;
  const max = Math.max(10, ...hist.flatMap((r) => r.scores));
  const n = Math.max(1, hist.length - 1);
  const x = (i: number) => pad + (i / n) * (W - pad - padR);
  const y = (v: number) => H - pad - (v / max) * (H - pad * 2);
  let out = `<svg viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" class="race">`;
  for (let i = 0; i <= n; i++) {
    out += `<line x1="${x(i)}" y1="${pad}" x2="${x(i)}" y2="${H - pad}" stroke="#23365e" stroke-width="1"/>`;
    out += `<text x="${x(i)}" y="${H - 8}" fill="#7f93bb" font-size="12" text-anchor="middle">${i === 0 ? 'Start' : `N${hist[i]?.night ?? i}`}</text>`;
  }
  d.factions.forEach((f, s) => {
    const color = faction(f).color;
    const pts = hist.map((r, i) => `${x(i).toFixed(1)},${y(r.scores[s] ?? 0).toFixed(1)}`).join(' ');
    out += `<polyline points="${pts}" fill="none" stroke="${color}" stroke-width="3" stroke-linejoin="round"/>`;
    const last = hist[hist.length - 1];
    out += `<text x="${W - padR + 6}" y="${y(last.scores[s] ?? 0) + 4}" fill="${color}" font-size="12">${esc(faction(f).short)}</text>`;
  });
  out += '</svg>';
  return out;
}
