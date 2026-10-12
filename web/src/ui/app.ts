// The planning window owns the game: menu, setup, the hotseat loop and saves.
// The Theater (another window, or an overlay here) shows the mornings and
// the nights.

import type { Engine } from '../engine';
import { Bus, type FinalData, type FromTheater, type MorningData, type ShowData, type ToTheater } from '../bus';
import { Theater } from './theater';
import { PlanningScreen, type Coach } from './planning';
import { factionCard, rulesModal } from './rules';
import { autosave, deleteSave, exportSave, importSave, listSaves } from '../save';
import { summarize, type NightSummary } from '../show/summary';
import type { FactionId, GameOptions, GameState, Orders, SaveFile, SeatConfig, SeatKind, EventRec } from '../types';
import { FACTIONS, emptyOrders } from '../types';
import { faction, fill, leaderLines, opensTonight, Voice } from '../text';
import { leaderSvg, emblemSvg, type Mood } from '../art/dogs';
import { h, clear, svg, toast, clearToast, modal } from './dom';
import { audio } from '../audio';
import { tutorialCoach } from './tutorial';

type Phase = 'menu' | 'setup' | 'morning' | 'curtain' | 'planning' | 'show' | 'final' | 'replay';

const MOOD: Record<string, Mood> = { hit: 'alarmed', landed: 'happy', ahead: 'happy', back: 'happy', behind: 'neutral', park: 'neutral', dawn: 'neutral', intro: 'happy' };

export class App {
  readonly engine: Engine;
  readonly root: HTMLElement;
  private screen = h('div.screen');
  private overlay = h('div.theater-overlay');
  private theater: Theater | null = null;
  private bus: Bus<FromTheater, ToTheater> | null = null;
  private theaterWin: Window | null = null;
  private remoteAlive = false;
  private lastTheaterMsg: ToTheater | null = null;
  private phase: Phase = 'menu';
  save: SaveFile | null = null;
  state = '';
  private voice = new Voice(0);
  private committed: (Orders | null)[] = [];
  private planning: PlanningScreen | null = null;
  private remoteStatus: Extract<FromTheater, { type: 'status' }> | null = null;
  private continueTo: (() => void) | null = null;

  constructor(root: HTMLElement, engine: Engine) {
    this.root = root;
    this.engine = engine;
    root.append(this.screen, this.overlay);
    this.overlay.style.display = 'none';
    this.bus = new Bus<FromTheater, ToTheater>((m) => this.fromTheater(m, true));
    window.addEventListener('beforeunload', () => this.theaterWin?.close());
  }

  // --- Theater link ---------------------------------------------------------------------------------

  private get twoWindow(): boolean {
    return this.save?.options.theater === 'two' && this.remoteAlive;
  }

  private ensureLocalTheater(): Theater {
    if (!this.theater) this.theater = new Theater(this.overlay, this.engine.content, (m) => this.fromTheater(m, false));
    return this.theater;
  }

  private toTheater(m: ToTheater) {
    if (m.type !== 'control' && m.type !== 'sound') this.lastTheaterMsg = m;
    if (this.twoWindow) {
      this.bus!.send(m);
      this.overlay.style.display = 'none';
      audio.music(null); // the TV window plays the music
    } else {
      const t = this.ensureLocalTheater();
      const visible = m.type === 'morning' || m.type === 'show' || m.type === 'final' || (m.type === 'control' && this.overlay.style.display !== 'none');
      if (m.type !== 'control' && m.type !== 'sound') {
        this.overlay.style.display = visible ? 'block' : 'none';
        t.setVisible(visible);
      }
      void t.handle(m);
    }
  }

  private fromTheater(m: FromTheater, remote: boolean) {
    if (remote && m.type === 'ready') {
      this.remoteAlive = true;
      if (this.lastTheaterMsg) this.bus!.send(this.lastTheaterMsg);
      this.bus!.send({ type: 'sound', on: audio.on, music: audio.musicEnabled });
      if (this.overlay.style.display !== 'none' && this.save?.options.theater === 'two') {
        this.overlay.style.display = 'none';
        this.renderController();
      }
      toast('TV window connected. Drag it to the TV and press F11.');
      return;
    }
    if (remote && !this.twoWindow) return;
    if (!remote && this.twoWindow) return;
    if (m.type === 'continue') this.onContinue();
    if (m.type === 'status') {
      this.remoteStatus = m;
      if (this.twoWindow && (this.phase === 'show' || this.phase === 'morning' || this.phase === 'replay')) this.renderController();
    }
  }

  openTheaterWindow() {
    const url = new URL('theater.html', document.baseURI);
    this.theaterWin = window.open(url.href, 'flyover-theater', 'popup,width=1280,height=720');
    if (!this.theaterWin) toast('The browser blocked the TV window. Allow pop-ups, or play on one screen.', 5000);
  }

  private onContinue() {
    const next = this.continueTo;
    if (!next) return;
    this.continueTo = null;
    next();
  }

  // --- Screens --------------------------------------------------------------------------------------

  private show(el: HTMLElement) {
    this.planning?.destroy();
    this.planning = null;
    clear(this.screen);
    this.screen.appendChild(el);
  }

  async menu() {
    this.phase = 'menu';
    this.save = null;
    this.continueTo = null;
    this.overlay.style.display = 'none';
    this.theater?.setVisible(false);
    const saves = await listSaves();
    const latest = saves.find((s) => !s.tutorial && s.nights.length < this.engine.content.rules.nights);
    const dogs = h('div.menu-dogs', ...FACTIONS.map((f) => svg(leaderSvg(f, 'happy', { size: 110 }), 'menu-dog')));
    const toggle = (label: string, get: () => boolean, set: (v: boolean) => void) => {
      const b = h('button.chipbtn', { onclick: () => { set(!get()); this.bus?.send({ type: 'sound', on: audio.on, music: audio.musicEnabled }); b.textContent = `${label}: ${get() ? 'on' : 'off'}`; } }, `${label}: ${get() ? 'on' : 'off'}`);
      return b;
    };
    audio.music('theme');
    this.show(
      h(
        'div.menu',
        h('h1.menu-title', 'Flyover Country'),
        h('div.menu-sub', 'An air war over the West for 1 to 7 players. Led by dogs.'),
        dogs,
        h(
          'div.menu-btns',
          h('button.go', { onclick: () => this.setup() }, 'New game'),
          latest ? h('button.btn', { onclick: () => this.load(latest) }, `Continue: Night ${latest.nights.length + 1}`) : null,
          h('button.btn', { onclick: () => this.tutorial() }, 'Learn by playing'),
          h('button.btn', { onclick: () => this.loadScreen() }, 'Saved games'),
          h('button.btn', { onclick: () => rulesModal() }, 'Rules'),
        ),
        h('div.menu-foot', toggle('Sound', () => audio.on, (v) => audio.setEnabled(v)), toggle('Music', () => audio.musicEnabled, (v) => audio.setMusic(v))),
      ),
    );
  }

  private setup() {
    this.phase = 'setup';
    const picked: FactionId[] = [];
    const kinds = new Map<FactionId, SeatKind>();
    const options: GameOptions = { timer: 0, theater: 'single', sound: audio.on };
    const grid = h('div.setup-grid');
    const seats = h('div.setup-seats');
    const start = h('button.go', { disabled: true, onclick: () => go() }, 'Start');
    const render = () => {
      clear(grid);
      for (const f of FACTIONS) {
        const i = picked.indexOf(f);
        const card = factionCard(f);
        card.classList.add('pickable');
        if (i >= 0) {
          card.classList.add('picked');
          card.appendChild(h('div.pick-n', String(i + 1)));
        }
        card.addEventListener('click', () => {
          if (i >= 0) picked.splice(i, 1);
          else if (picked.length < 7) {
            picked.push(f);
            if (!kinds.has(f)) kinds.set(f, picked.length === 1 ? 'human' : 'normal');
            audio.play('click');
          }
          render();
        });
        grid.appendChild(card);
      }
      clear(seats);
      seats.appendChild(h('h3', picked.length ? `${picked.length} sides` : 'Pick 2 to 7 sides'));
      picked.forEach((f) => {
        const k = kinds.get(f) ?? 'human';
        const seg = (v: SeatKind, label: string) => h(`button.seg${k === v ? '.on' : ''}`, { onclick: () => { kinds.set(f, v); render(); } }, label);
        seats.appendChild(h('div.seat-row', { style: { '--fc': faction(f).color } as unknown as CSSStyleDeclaration }, svg(emblemSvg(f, { size: 24 })), h('span.seat-name', faction(f).short), h('span.segs', seg('human', 'Person'), seg('easy', 'Easy bot'), seg('normal', 'Bot'))));
      });
      const humans = picked.filter((f) => kinds.get(f) === 'human').length;
      (start as HTMLButtonElement).disabled = picked.length < 2 || humans === 0;
      start.textContent = picked.length < 2 ? `Pick ${2 - picked.length} more` : humans === 0 ? 'Make one a person' : 'Start';
    };
    const opt = (label: string, values: [string, string][], get: () => string, set: (v: string) => void) => {
      const segs = h('span.segs');
      const draw = () => {
        clear(segs);
        for (const [v, l] of values) segs.appendChild(h(`button.seg${get() === v ? '.on' : ''}`, { onclick: () => { set(v); draw(); } }, l));
      };
      draw();
      return h('div.opt-row', h('span', label), segs);
    };
    const go = () => {
      clearToast();
      const cfg = picked.map((f) => ({ faction: f, kind: kinds.get(f) ?? 'human' }));
      void this.start(cfg, options);
    };
    const random = () => {
      picked.length = 0;
      const pool = [...FACTIONS].sort(() => Math.random() - 0.5);
      picked.push(...pool.slice(0, 4));
      picked.forEach((f, i) => kinds.set(f, kinds.get(f) ?? (i === 0 ? 'human' : 'normal')));
      render();
    };
    render();
    this.show(
      h(
        'div.setup',
        h('div.setup-head', h('button.btn', { onclick: () => this.menu() }, '◂ Menu'), h('h1', 'Who’s flying?'), h('button.btn', { onclick: random }, 'Random')),
        h(
          'div.setup-body',
          grid,
          h(
            'div.setup-side',
            seats,
            h(
              'div.setup-opts',
              opt('Timer', [['0', 'Off'], ['180', '3 min'], ['300', '5 min']], () => String(options.timer), (v) => (options.timer = Number(v))),
              opt('Screens', [['single', 'One'], ['two', 'Laptop + TV']], () => options.theater, (v) => {
                options.theater = v as 'single' | 'two';
                if (v === 'two' && !this.remoteAlive) this.openTheaterWindow();
              }),
            ),
            start,
          ),
        ),
      ),
    );
  }

  private async start(seats: SeatConfig[], options: GameOptions, tutorial = false) {
    const seed = Math.floor(Math.random() * 2 ** 31);
    const factions = seats.map((s) => s.faction);
    const state = this.engine.newGame(seed, factions);
    const save: SaveFile = {
      version: 2,
      id: crypto.randomUUID ? crypto.randomUUID() : String(Date.now()),
      created: Date.now(),
      updated: Date.now(),
      seed,
      factions,
      seats,
      options,
      nights: [],
      snapshots: [state],
      tutorial,
      voice: {},
    };
    this.save = save;
    this.state = state;
    this.voice = new Voice(seed, save.voice);
    save.morning = this.morningData(null, this.gs());
    await autosave(save);
    this.morning();
  }

  load(save: SaveFile) {
    this.save = save;
    save.voice ??= {};
    this.voice = new Voice(save.seed, save.voice);
    this.state = save.snapshots[save.snapshots.length - 1];
    if (save.options.theater === 'two' && !this.remoteAlive) this.openTheaterWindow();
    if (this.over(this.gs()) || save.nights.length >= this.engine.content.rules.nights) {
      this.final();
      return;
    }
    if (!save.morning || save.morning.night !== this.gs().night) save.morning = this.morningData(null, this.gs());
    this.morning();
  }

  private gs(state = this.state): GameState {
    return this.engine.parse(state);
  }

  private over(gs: GameState): boolean {
    return gs.night > this.engine.content.rules.nights;
  }

  private factions(): FactionId[] {
    return this.save!.factions;
  }
  private kinds(): SeatKind[] {
    return this.save!.seats.map((s) => s.kind);
  }

  private line(f: FactionId, pool: keyof ReturnType<typeof leaderLines>, params: Record<string, string> = {}): string | null {
    const l = this.voice.take(`leader.${f}.${pool}`, leaderLines(f)[pool]);
    return l ? fill(l, params) : null;
  }

  /** The morning card: last night in a few lines, one dog talking, and anything new. */
  private morningData(sum: NightSummary | null, gs: GameState): MorningData {
    const save = this.save!;
    const intro = save.nights.length === 0;
    let speaker: MorningData['speaker'] = null;
    if (intro) {
      const seat = Math.max(0, save.seats.findIndex((s) => s.kind === 'human'));
      const line = this.line(save.factions[seat], 'intro');
      if (line) speaker = { seat, mood: 'happy', line };
    } else if (sum?.speaker) {
      const sp = sum.speaker;
      const pool = sp.pool === 'behind' && gs.seats[sp.seat].doghouse ? 'park' : sp.pool;
      const line = this.line(save.factions[sp.seat], pool, sp.params);
      if (line) speaker = { seat: sp.seat, mood: MOOD[pool] ?? 'neutral', line };
    }
    const news: string[] = [];
    const opens = opensTonight(gs.night);
    if (opens) news.push(opens);
    const park = gs.seats.map((s, i) => (s.doghouse ? faction(save.factions[i]).short : null)).filter((x): x is string => !!x);
    if (park.length) news.push(`${park.join(' and ')} can hide at Project Bark Park tonight.`);
    if (gs.night === this.engine.content.rules.nights) news.push('Last night of the war.');
    return {
      night: gs.night,
      nights: this.engine.content.rules.nights,
      factions: save.factions,
      kinds: this.kinds(),
      state: gs,
      report: sum ? { headline: sum.headline, highlights: sum.highlights, gained: sum.gained } : null,
      speaker,
      news,
      intro,
    };
  }

  private finalData(gs: GameState): FinalData {
    const save = this.save!;
    const scores = gs.seats.map((s) => s.score);
    const best = Math.max(...scores);
    const w = scores.indexOf(best);
    const head = this.voice.take('headline.win', this.engine.content.flavor.headlines.win) ?? '{attacker} wins the West!';
    return {
      factions: save.factions,
      kinds: this.kinds(),
      state: gs,
      headline: fill(head, { attacker: faction(save.factions[w]).short, n: String(best) }),
      line: this.line(save.factions[w], 'win'),
    };
  }

  // --- The nightly loop ---------------------------------------------------------------------------------

  private morning() {
    const save = this.save!;
    this.phase = 'morning';
    this.committed = save.seats.map(() => null);
    this.continueTo = () => this.nextPlanner();
    this.toTheater({ type: 'morning', data: save.morning ?? this.morningData(null, this.gs()) });
    if (this.twoWindow) this.renderController();
    else this.show(h('div.wait'));
  }

  private nextPlanner() {
    const save = this.save!;
    const seat = save.seats.findIndex((s, i) => s.kind === 'human' && !this.committed[i]);
    if (seat < 0) {
      this.resolve();
      return;
    }
    const humans = save.seats.filter((s) => s.kind === 'human').length;
    if (humans === 1 || save.tutorial) this.plan(seat);
    else this.curtain(seat);
  }

  private waiting(seat: number): FactionId[] {
    const save = this.save!;
    return save.seats.map((s, i) => ({ s, i })).filter(({ s, i }) => s.kind === 'human' && !this.committed[i] && i !== seat).map(({ i }) => save.factions[i]);
  }

  private curtain(seat: number) {
    this.phase = 'curtain';
    const save = this.save!;
    const f = save.factions[seat];
    const fd = faction(f);
    this.toTheater({ type: 'curtain', seat, faction: f, state: this.gs(), factions: this.factions(), kinds: this.kinds(), deadline: null, waiting: this.waiting(seat) });
    this.overlay.style.display = 'none';
    const hold = h('button.hold-btn', 'Hold to start');
    let timer: number | null = null;
    let raf = 0;
    const startHold = (e: Event) => {
      e.preventDefault();
      const t0 = performance.now();
      const fillBar = () => {
        const k = Math.min(1, (performance.now() - t0) / 800);
        hold.style.setProperty('--p', String(k));
        if (k < 1) raf = requestAnimationFrame(fillBar);
      };
      fillBar();
      timer = window.setTimeout(() => {
        cancelAnimationFrame(raf);
        this.plan(seat);
      }, 800);
    };
    const stopHold = () => {
      if (timer !== null) clearTimeout(timer);
      timer = null;
      cancelAnimationFrame(raf);
      hold.style.setProperty('--p', '0');
    };
    hold.addEventListener('pointerdown', startHold);
    hold.addEventListener('pointerup', stopHold);
    hold.addEventListener('pointerleave', stopHold);
    hold.addEventListener('keydown', (e) => {
      if ((e.key === 'Enter' || e.key === ' ') && timer === null) startHold(e);
    });
    hold.addEventListener('keyup', stopHold);
    this.show(
      h(
        'div.curtain',
        { style: { '--fc': fd.color } as unknown as CSSStyleDeclaration },
        svg(leaderSvg(f, 'neutral', { size: 200, bg: true }), 'curtain-dog'),
        h('div.curtain-pass', `Pass to ${fd.short}`),
        h('div.curtain-note', 'Everyone else, eyes on the TV.'),
        hold,
      ),
    );
  }

  private plan(seat: number) {
    this.phase = 'planning';
    const save = this.save!;
    const prev = save.nights.length ? save.nights[save.nights.length - 1].orders[seat] : null;
    const initial: Orders = prev ? this.engine.carryOver(this.state, prev) : emptyOrders(seat);
    clear(this.screen);
    this.planning?.destroy();
    const humans = save.seats.filter((s) => s.kind === 'human').length;
    if (humans > 1 && !save.tutorial) {
      this.toTheater({ type: 'curtain', seat, faction: save.factions[seat], state: this.gs(), factions: this.factions(), kinds: this.kinds(), deadline: save.options.timer ? Date.now() + save.options.timer * 1000 : null, waiting: this.waiting(seat) });
    } else {
      this.overlay.style.display = 'none';
    }
    const coach: Coach | null = save.tutorial && save.nights.length === 0 ? tutorialCoach() : null;
    const screen = new PlanningScreen(this.screen, {
      engine: this.engine,
      state: this.state,
      seat,
      factions: this.factions(),
      kinds: this.kinds(),
      initial,
      timer: save.tutorial ? 0 : save.options.timer,
      coach,
      onCommit: (o) => {
        this.committed[seat] = o;
        const more = save.seats.some((s, i) => s.kind === 'human' && !this.committed[i]);
        if (more) {
          this.show(h('div.curtain.locked', h('div.curtain-pass', 'Orders in.'), h('div.curtain-note', 'Pass the laptop on.'), h('button.go', { onclick: () => this.nextPlanner() }, 'Next player ▸')));
        } else {
          this.nextPlanner();
        }
      },
    });
    this.planning = screen;
    void screen.init();
  }

  private resolve() {
    const save = this.save!;
    const before = this.state;
    const orders: Orders[] = save.seats.map((s, i) => {
      if (s.kind === 'human') return this.committed[i]!;
      return this.engine.suggest(before, i, s.kind === 'easy' ? 'easy' : 'normal');
    });
    const r = this.engine.resolve(before, orders);
    save.nights.push({ orders, hash: r.hash });
    save.snapshots.push(r.state);
    this.state = r.state;
    this.playNight(save.nights.length, before, r.state, r.events, false);
  }

  private playNight(night: number, before: string, after: string, events: EventRec[], replay: boolean) {
    this.phase = replay ? 'replay' : 'show';
    const save = this.save!;
    const gsBefore = this.gs(before);
    const gsAfter = this.gs(after);
    // A replay borrows a copy of the dogs' memory so it doesn't use up fresh lines.
    const voice = replay ? new Voice(save.seed, { ...save.voice }) : this.voice;
    const sum = summarize(this.engine.content, events, this.factions(), gsBefore, gsAfter, voice);
    let next: ShowData['next'];
    if (replay) next = { type: 'back' };
    else if (this.over(gsAfter)) next = { type: 'final', data: this.finalData(gsAfter) };
    else {
      save.morning = this.morningData(sum, gsAfter);
      next = { type: 'morning', data: save.morning };
    }
    if (!replay) void autosave(save);
    const data: ShowData = { night, factions: this.factions(), kinds: this.kinds(), events, before: gsBefore, after: gsAfter, captions: sum.captions, next };
    this.continueTo = () => {
      if (replay) void this.loadScreen();
      else if (this.over(gsAfter)) void this.menu();
      else {
        this.phase = 'morning';
        this.committed = save.seats.map(() => null);
        this.nextPlanner();
      }
    };
    this.toTheater({ type: 'show', data });
    if (this.twoWindow) this.renderController();
    else this.show(h('div.wait'));
  }

  private final() {
    this.phase = 'final';
    this.continueTo = () => void this.menu();
    this.toTheater({ type: 'final', data: this.finalData(this.gs()) });
    if (this.twoWindow) this.show(h('div.controller', h('h2', 'Results are on the TV.'), h('button.go', { onclick: () => this.menu() }, 'Back to the menu')));
    else this.show(h('div.wait'));
  }

  /** In two-window mode the laptop becomes a remote while the TV plays. */
  private renderController() {
    const st = this.remoteStatus;
    const btn = (label: string, on: () => void) => h('button.btn', { onclick: on }, label);
    const ctl = (action: 'pause' | 'speed' | 'skip' | 'replay', value?: number) => this.toTheater({ type: 'control', action, value });
    const playing = st?.phase === 'show';
    this.show(
      h(
        'div.controller',
        h('h2', playing ? 'The night is on the TV.' : 'Look at the TV.'),
        playing ? h('div.ctl-row', btn(st?.paused ? '▶ Play' : '❚❚ Pause', () => ctl('pause')), btn('1×', () => ctl('speed', 1)), btn('2×', () => ctl('speed', 2)), btn('Skip ⏭', () => ctl('skip'))) : null,
        !playing ? h('button.go', { onclick: () => this.onContinue() }, 'Continue ▸') : null,
        h('button.link', { onclick: () => { this.save!.options.theater = 'single'; this.remoteAlive = false; if (this.lastTheaterMsg) this.toTheater(this.lastTheaterMsg); } }, 'Show it on this screen instead'),
      ),
    );
  }

  // --- Saves, replays, tutorial ------------------------------------------------------------------------------

  async loadScreen() {
    this.phase = 'menu';
    this.overlay.style.display = 'none';
    const saves = await listSaves();
    const nights = this.engine.content.rules.nights;
    const file = h('input', { type: 'file', accept: '.flyover,.json,.gz', style: { display: 'none' } }) as HTMLInputElement;
    file.addEventListener('change', async () => {
      const f = file.files?.[0];
      if (!f) return;
      try {
        const s = await importSave(f);
        await autosave(s);
        toast('Imported.');
        void this.loadScreen();
      } catch (e) {
        toast(String((e as Error).message ?? e), 4000);
      }
    });
    const rows = saves.map((s) =>
      h(
        'div.save-row',
        h('div.save-who', ...s.factions.map((f) => svg(emblemSvg(f, { size: 26 }), 'save-emblem')), h('span', s.factions.map((f) => faction(f).short).join(' · '))),
        h('div.save-meta', `${s.tutorial ? 'Practice · ' : ''}Night ${Math.min(s.nights.length + 1, nights)} of ${nights} · ${new Date(s.updated).toLocaleDateString()}`),
        h(
          'div.save-btns',
          h('button.btn', { onclick: () => this.load(s) }, s.nights.length < nights ? 'Resume' : 'Results'),
          s.nights.length ? h('select.replay-pick', { onchange: (e: Event) => { const v = Number((e.target as HTMLSelectElement).value); if (v) this.replay(s, v); } }, h('option', { value: '' }, 'Watch a night…'), ...s.nights.map((_, i) => h('option', { value: String(i + 1) }, `Night ${i + 1}`))) : null,
          h('button.btn', { onclick: () => exportSave(s) }, 'Export'),
          h('button.btn.danger', { onclick: async () => { await deleteSave(s.id); void this.loadScreen(); } }, 'Delete'),
        ),
      ),
    );
    this.show(
      h(
        'div.loadscreen',
        h('div.setup-head', h('button.btn', { onclick: () => this.menu() }, '◂ Menu'), h('h1', 'Saved games'), h('button.btn', { onclick: () => file.click() }, 'Import')),
        file,
        rows.length ? h('div.save-list', ...rows) : h('p.note', 'No saved games yet. Every game saves itself each morning.'),
      ),
    );
  }

  /** Watch any past night again: re-resolve it from its snapshot and orders. */
  private replay(save: SaveFile, night: number) {
    this.save = save;
    save.voice ??= {};
    const before = save.snapshots[night - 1];
    const r = this.engine.resolve(before, save.nights[night - 1].orders);
    if (r.hash !== save.nights[night - 1].hash) toast(`Night ${night} plays differently in this version of the game.`, 5000);
    this.state = save.snapshots[save.snapshots.length - 1];
    this.playNight(night, before, r.state, r.events, true);
  }

  private tutorial() {
    const seats: SeatConfig[] = [
      { faction: 'Pacific', kind: 'human' },
      { faction: 'Dreamland', kind: 'easy' },
      { faction: 'Beehive', kind: 'easy' },
      { faction: 'FrontRange', kind: 'easy' },
    ];
    void this.start(seats, { timer: 0, theater: 'single', sound: audio.on }, true);
    modal(
      h(
        'div.confirm',
        svg(leaderSvg('Pacific', 'happy', { size: 130, bg: true })),
        h('h2', 'Learn by playing'),
        h('p', 'You’re California. Three easy bots play the others. Princess Donut will show you around your first night.'),
      ),
    );
  }
}
