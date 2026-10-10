// The planning window owns the game: menu, setup, the hotseat loop and saves.
// The Theater (another window, or an overlay here) only gets public views.

import type { Engine } from '../engine';
import { Bus, type DawnData, type FromTheater, type ShowData, type ToTheater } from '../bus';
import { Theater } from './theater';
import { PlanningScreen, type Coach } from './planning';
import { factionCard, rulesModal } from './rules';
import { autosave, deleteSave, exportSave, importSave, listSaves } from '../save';
import { summarize, leaderHeadline } from '../show/summary';
import type { FactionId, GameOptions, Orders, PublicView, SaveFile, SeatConfig, SeatKind, EventRec } from '../types';
import { FACTIONS } from '../types';
import { any, faction, leader, turns } from '../text';
import { leaderSvg, emblemSvg } from '../art/dogs';
import { h, clear, svg, toast, modal } from './dom';
import { audio } from '../audio';
import { tutorialCoach } from './tutorial';

type Phase = 'menu' | 'setup' | 'dawn' | 'curtain' | 'planning' | 'show' | 'report' | 'final' | 'replay';

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
  private committed: (Orders | null)[] = [];
  private planning: PlanningScreen | null = null;
  private lastSummary: { headline: string; highlights: string[] } | null = null;
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

  // --- Theater link -------------------------------------------------------------------------------------

  private get twoWindow(): boolean {
    return this.save?.options.theater === 'two' && this.remoteAlive;
  }

  private ensureLocalTheater(): Theater {
    if (!this.theater) {
      this.theater = new Theater(this.overlay, this.engine.content, (m) => this.fromTheater(m, false));
    }
    return this.theater;
  }

  private toTheater(m: ToTheater) {
    if (m.type !== 'control' && m.type !== 'sound') this.lastTheaterMsg = m;
    if (this.twoWindow) {
      this.bus!.send(m);
      this.overlay.style.display = 'none';
    } else {
      const t = this.ensureLocalTheater();
      const visible = m.type === 'dawn' || m.type === 'show' || m.type === 'final' || (m.type === 'control' && this.overlay.style.display !== 'none');
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
      this.bus!.send({ type: 'sound', on: audio.on });
      if (this.overlay.style.display !== 'none' && this.save?.options.theater === 'two') {
        this.overlay.style.display = 'none';
        this.renderController();
      }
      toast('Theater window connected. Drag it to the TV and press F11.');
      return;
    }
    if (remote && !this.twoWindow) return;
    if (!remote && this.twoWindow) return;
    if (m.type === 'continue') this.onContinue();
    if (m.type === 'status') {
      this.remoteStatus = m;
      if (this.twoWindow && (this.phase === 'show' || this.phase === 'report' || this.phase === 'dawn')) this.renderController();
    }
  }

  openTheaterWindow() {
    const url = new URL('theater.html', document.baseURI);
    this.theaterWin = window.open(url.href, 'flyover-theater', 'popup,width=1280,height=720');
    if (!this.theaterWin) toast('The browser blocked the Theater window. Allow pop-ups, or play on one screen.', 5000);
  }

  private onContinue() {
    const next = this.continueTo;
    if (!next) return;
    this.continueTo = null;
    next();
  }

  // --- Screens -------------------------------------------------------------------------------------------

  private show(el: HTMLElement) {
    this.planning?.destroy();
    this.planning = null;
    clear(this.screen);
    this.screen.appendChild(el);
  }

  async menu() {
    this.phase = 'menu';
    this.save = null;
    this.overlay.style.display = 'none';
    this.theater?.setVisible(false);
    const saves = await listSaves();
    const latest = saves.find((s) => !s.tutorial && s.nights.length < this.engine.content.rules.nights);
    const dogs = h('div.menu-dogs', ...FACTIONS.map((f) => svg(leaderSvg(f, 'happy', { size: 120 }), 'menu-dog')));
    const sound = h('button.pbtn', { onclick: () => { audio.setEnabled(!audio.on); this.bus?.send({ type: 'sound', on: audio.on }); sound.textContent = audio.on ? 'Sound: on' : 'Sound: off'; } }, audio.on ? 'Sound: on' : 'Sound: off');
    this.show(
      h(
        'div.menu',
        h('div.menu-title', 'FLYOVER COUNTRY'),
        h('div.menu-sub', 'Seven breakaway states. Eight nights. One evening. The Bragging Rights go to the best dog.'),
        dogs,
        h(
          'div.menu-btns',
          h('button.big-btn', { onclick: () => this.setup() }, 'New War'),
          latest ? h('button.big-btn.alt', { onclick: () => this.load(latest) }, `Continue (Night ${latest.nights.length + 1}, ${latest.factions.map((f) => faction(f).short).join(', ')})`) : null,
          h('button.big-btn.alt', { onclick: () => this.tutorial() }, 'Night Zero: the 5-minute tutorial'),
          h('button.big-btn.alt', { onclick: () => this.loadScreen() }, 'Saved wars and replays'),
          h('button.big-btn.alt', { onclick: () => rulesModal() }, 'Rules'),
        ),
        h('div.menu-foot', sound, h('span', 'Best with a laptop for planning and a TV for the show. Everything runs in your browser; nothing is sent anywhere.')),
      ),
    );
  }

  private setup() {
    this.phase = 'setup';
    const picked: FactionId[] = [];
    const kinds = new Map<FactionId, SeatKind>();
    const options: GameOptions = { timer: 90, theater: 'single', sound: audio.on };
    const grid = h('div.setup-grid');
    const seats = h('div.setup-seats');
    const start = h('button.big-btn', { disabled: true, onclick: () => go() }, 'Start the war');
    const render = () => {
      clear(grid);
      for (const f of FACTIONS) {
        const i = picked.indexOf(f);
        const card = factionCard(f, { compact: true });
        card.classList.add('pickable');
        if (i >= 0) card.classList.add('picked');
        card.addEventListener('click', () => {
          if (i >= 0) picked.splice(i, 1);
          else if (picked.length < 4) {
            picked.push(f);
            if (!kinds.has(f)) kinds.set(f, picked.length === 1 ? 'human' : 'normal');
            toast(turns(any(leader(f).pick) ?? '').map((t) => (t.who ? `${t.who}: ${t.text}` : t.text)).join('  '), 3200);
          }
          render();
        });
        grid.appendChild(card);
      }
      clear(seats);
      seats.appendChild(h('div.side-h', `Seats (${picked.length}/4). Bots fill any seat you like.`));
      picked.forEach((f, i) => {
        const k = kinds.get(f) ?? 'human';
        const seg = (v: SeatKind, label: string) => h(`button.seg${k === v ? '.on' : ''}`, { onclick: () => { kinds.set(f, v); render(); } }, label);
        seats.appendChild(
          h('div.seat-row', { style: { '--fc': faction(f).color } as unknown as CSSStyleDeclaration }, h('span.seat-n', `${i + 1}`), h('span.seat-name', faction(f).short), h('span.segs', seg('human', 'Human'), seg('easy', 'Bot: Easy'), seg('normal', 'Bot: Normal'))),
        );
      });
      const humans = picked.filter((f) => kinds.get(f) === 'human').length;
      (start as HTMLButtonElement).disabled = picked.length !== 4 || humans === 0;
      start.textContent = picked.length !== 4 ? `Pick ${4 - picked.length} more` : humans === 0 ? 'Make at least one seat human' : 'Start the war';
    };
    const opt = (label: string, values: [string, string][], get: () => string, set: (v: string) => void) => {
      const row = h('div.opt-row', h('span', label));
      const segs = h('span.segs');
      const draw = () => {
        clear(segs);
        for (const [v, l] of values) segs.appendChild(h(`button.seg${get() === v ? '.on' : ''}`, { onclick: () => { set(v); draw(); } }, l));
      };
      draw();
      row.appendChild(segs);
      return row;
    };
    const go = () => {
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
        h('div.setup-head', h('button.pbtn', { onclick: () => this.menu() }, '◂ Menu'), h('h1', 'Pick four factions'), h('button.pbtn', { onclick: random }, 'Random lineup')),
        h(
          'div.setup-body',
          grid,
          h(
            'div.setup-side',
            h(
              'div.setup-side-body',
              seats,
              h(
                'div.setup-opts',
                opt('Planning timer', [['0', 'Off'], ['90', '90 s'], ['180', '3 min']], () => String(options.timer), (v) => (options.timer = Number(v))),
                opt('Screens', [['single', 'One screen'], ['two', 'Laptop + TV']], () => options.theater, (v) => {
                  options.theater = v as 'single' | 'two';
                  if (v === 'two' && !this.remoteAlive) this.openTheaterWindow();
                }),
                h('div.side-note', 'Laptop + TV: a Theater window opens; drag it onto the TV. It only ever shows public information.'),
              ),
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
      version: 1,
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
    };
    this.save = save;
    this.state = state;
    this.lastSummary = null;
    await autosave(save);
    this.dawn();
  }

  load(save: SaveFile) {
    this.save = save;
    this.state = save.snapshots[save.snapshots.length - 1];
    this.lastSummary = null;
    if (save.options.theater === 'two' && !this.remoteAlive) this.openTheaterWindow();
    if (save.nights.length >= this.engine.content.rules.nights) this.final();
    else this.dawn();
  }

  private factions(): FactionId[] {
    return this.save!.factions;
  }
  private kinds(): SeatKind[] {
    return this.save!.seats.map((s) => s.kind);
  }

  private pv(state = this.state): PublicView {
    return this.engine.publicView(state);
  }

  // --- The nightly loop ------------------------------------------------------------------------------------

  private dawn() {
    const save = this.save!;
    const pv = this.pv();
    if (pv.over) {
      this.final();
      return;
    }
    this.phase = 'dawn';
    this.committed = save.seats.map(() => null);
    const intro = save.nights.length === 0;
    const headline = intro
      ? 'THE WEST IS UP FOR GRABS'
      : this.lastSummary?.headline ?? leaderHeadline(this.factions(), pv.seats.map((s) => s.score), pv.night) ?? 'ANOTHER DAWN OVER THE WEST';
    const data: DawnData = {
      night: pv.night,
      nights: pv.nights,
      stage: pv.stage,
      factions: this.factions(),
      kinds: this.kinds(),
      pv,
      headline,
      highlights: intro
        ? ['Years after the Great Unplugging, the West has split into breakaway states. Nobody remembers who unplugged it. Everyone blames California.', 'Eight nights. Most Bragging Rights wins. Nobody dies on screen.']
        : this.lastSummary?.highlights ?? [],
      intro,
    };
    this.continueTo = () => this.nextPlanner();
    this.toTheater({ type: 'dawn', data });
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
    if (humans === 1 && !save.tutorial) this.plan(seat);
    else if (save.tutorial) this.plan(seat);
    else this.curtain(seat);
  }

  private curtain(seat: number) {
    this.phase = 'curtain';
    const save = this.save!;
    const f = save.factions[seat];
    const fd = faction(f);
    const deadline = null;
    const waiting = save.seats.map((s, i) => ({ s, i })).filter(({ s, i }) => s.kind === 'human' && !this.committed[i] && i !== seat).map(({ i }) => save.factions[i]);
    this.toTheater({ type: 'curtain', seat, faction: f, pv: this.pv(), factions: this.factions(), kinds: this.kinds(), deadline, waiting });
    this.overlay.style.display = 'none';
    const hold = h('button.hold-btn', 'Hold to reveal your map');
    let timer: number | null = null;
    let raf = 0;
    const startHold = (e: Event) => {
      e.preventDefault();
      const t0 = performance.now();
      const fill = () => {
        const k = Math.min(1, (performance.now() - t0) / 1000);
        hold.style.setProperty('--p', String(k));
        if (k < 1) raf = requestAnimationFrame(fill);
      };
      fill();
      timer = window.setTimeout(() => {
        cancelAnimationFrame(raf);
        this.plan(seat);
      }, 1000);
    };
    const stopHold = () => {
      if (timer !== null) clearTimeout(timer);
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
        svg(leaderSvg(f, 'neutral', { size: 220, bg: true }), 'curtain-dog'),
        h('div.curtain-pass', `Pass to ${fd.short.toUpperCase()}`),
        h('div.curtain-eyes', 'Everyone else, eyes on the TV.'),
        hold,
        h('div.curtain-note', `Night ${this.pv().night}. ${leader(f).name} is waiting.`),
      ),
    );
  }

  private plan(seat: number) {
    this.phase = 'planning';
    const save = this.save!;
    const prev = save.nights.length ? save.nights[save.nights.length - 1].orders[seat] : null;
    const carry = this.engine.carryOver(this.state, seat);
    const initial: Orders = prev ? { ...prev, missions: carry.missions, builds: [], repairs: [], mass_driver: prev.mass_driver, revenge: null } : carry;
    clear(this.screen);
    this.planning?.destroy();
    const humans = save.seats.filter((s) => s.kind === 'human').length;
    if (humans > 1 && !save.tutorial) {
      const waiting = save.seats.map((s, i) => ({ s, i })).filter(({ s, i }) => s.kind === 'human' && !this.committed[i] && i !== seat).map(({ i }) => save.factions[i]);
      this.toTheater({ type: 'curtain', seat, faction: save.factions[seat], pv: this.pv(), factions: this.factions(), kinds: this.kinds(), deadline: save.options.timer ? Date.now() + save.options.timer * 1000 : null, waiting });
    } else {
      this.overlay.style.display = 'none';
    }
    const coach: Coach | null = save.tutorial ? tutorialCoach() : null;
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
          this.show(h('div.curtain.locked', h('div.curtain-pass', 'Orders locked.'), h('div.curtain-eyes', 'Pass the laptop on.'), h('button.big-btn', { onclick: () => this.nextPlanner() }, 'Next player ▸')));
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
    void autosave(save);
    const night = save.nights.length;
    this.playNight(night, before, r.state, r.events, false);
  }

  private playNight(night: number, before: string, after: string, events: EventRec[], replay: boolean) {
    this.phase = replay ? 'replay' : 'show';
    const pub = events.filter((e) => e.public);
    const pvBefore = this.pv(before);
    const pvAfter = this.pv(after);
    const sum = summarize(this.engine.content, pub, this.factions(), night, night * 977);
    if (!replay) this.lastSummary = { headline: sum.headline, highlights: sum.highlights };
    const data: ShowData = {
      night,
      factions: this.factions(),
      kinds: this.kinds(),
      events: pub,
      before: pvBefore,
      after: pvAfter,
      last: !replay && pvAfter.over,
    };
    this.continueTo = () => {
      if (replay) {
        void this.loadScreen();
        return;
      }
      if (pvAfter.over) this.final();
      else this.dawn();
    };
    this.toTheater({ type: 'show', data });
    if (this.twoWindow) this.renderController();
    else this.show(h('div.wait'));
  }

  private final() {
    this.phase = 'final';
    const pv = this.pv();
    this.continueTo = null;
    this.toTheater({ type: 'final', data: { factions: this.factions(), kinds: this.kinds(), pv } });
    const back = h('div.final-actions', h('button.big-btn', { onclick: () => this.menu() }, 'Back to the menu'), h('button.big-btn.alt', { onclick: () => this.save && exportSave(this.save) }, 'Export this war'));
    if (this.twoWindow) {
      this.show(h('div.controller', h('h2', 'The war is over. Results are on the TV.'), back));
    } else {
      this.overlay.style.display = 'block';
      this.show(h('div.wait'));
      this.overlay.querySelector('.th-final')?.appendChild(back);
      setTimeout(() => this.overlay.querySelector('.th-final')?.appendChild(back), 50);
    }
  }

  /** In two-window mode the laptop becomes a remote while the TV plays. */
  private renderController() {
    const st = this.remoteStatus;
    const btn = (label: string, on: () => void) => h('button.pbtn', { onclick: on }, label);
    const ctl = (action: 'pause' | 'speed' | 'skip' | 'replay', value?: number) => this.toTheater({ type: 'control', action, value });
    const what = this.phase === 'dawn' ? 'The dawn briefing is on the TV.' : this.phase === 'show' || this.phase === 'replay' ? 'The show is on the TV.' : 'On the TV.';
    this.show(
      h(
        'div.controller',
        h('h2', what),
        this.phase === 'show' || this.phase === 'replay'
          ? h('div.ctl-row', btn(st?.paused ? '▶ Play' : '❚❚ Pause', () => ctl('pause')), btn('1×', () => ctl('speed', 1)), btn('2×', () => ctl('speed', 2)), btn('4×', () => ctl('speed', 4)), btn('Skip to report', () => ctl('skip')), btn('Play of the Night', () => ctl('replay')))
          : null,
        this.phase === 'dawn' || st?.phase === 'report' || st?.phase === 'final'
          ? h('button.big-btn', { onclick: () => this.onContinue() }, this.phase === 'dawn' ? 'Start planning ▸' : 'Continue ▸')
          : h('div.side-note', 'Continue appears here once the morning report is up.'),
        h('div.side-note', 'Anyone can press Space on the TV window too.'),
        h('button.pbtn', { onclick: () => { this.save!.options.theater = 'single'; this.remoteAlive = false; if (this.lastTheaterMsg) this.toTheater(this.lastTheaterMsg); } }, 'Show it on this screen instead'),
      ),
    );
  }

  // --- Saves, replays, tutorial -------------------------------------------------------------------------------

  async loadScreen() {
    this.phase = 'menu';
    this.overlay.style.display = 'none';
    const saves = await listSaves();
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
        h('div.save-meta', `${s.tutorial ? 'Tutorial · ' : ''}${s.nights.length}/${this.engine.content.rules.nights} nights · ${new Date(s.updated).toLocaleString()}`),
        h(
          'div.save-btns',
          s.nights.length < this.engine.content.rules.nights ? h('button.pbtn', { onclick: () => this.load(s) }, 'Resume') : h('button.pbtn', { onclick: () => this.load(s) }, 'Results'),
          ...s.nights.map((_, i) => h('button.pbtn.small', { title: `Replay Night ${i + 1}`, onclick: () => this.replay(s, i + 1) }, `▶${i + 1}`)),
          h('button.pbtn', { onclick: () => exportSave(s) }, 'Export'),
          h('button.pbtn.danger', { onclick: async () => { await deleteSave(s.id); void this.loadScreen(); } }, 'Delete'),
        ),
      ),
    );
    this.show(
      h(
        'div.loadscreen',
        h('div.setup-head', h('button.pbtn', { onclick: () => this.menu() }, '◂ Menu'), h('h1', 'Saved wars'), h('button.pbtn', { onclick: () => file.click() }, 'Import a save file')),
        file,
        rows.length ? h('div.save-list', ...rows) : h('p.side-note', 'No saved wars yet. Every game autosaves at dawn.'),
      ),
    );
  }

  /** Replay any past night: re-resolve it from its snapshot and orders. The hash must match. */
  private replay(save: SaveFile, night: number) {
    this.save = save;
    const before = save.snapshots[night - 1];
    const r = this.engine.resolve(before, save.nights[night - 1].orders);
    if (r.hash !== save.nights[night - 1].hash) {
      toast(`Replay mismatch on Night ${night}: this save came from a different version of the game.`, 6000);
    }
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
        svg(leaderSvg('Pacific', 'happy', { size: 140, bg: true })),
        h('h2', 'Night Zero'),
        h('p', "Princess Donut here, Founder and Chief Princess Officer of the Pacific Venture Republic. Let's run one night together: read the briefing, give a few orders, then watch the show."),
        h('p', 'Three easy bots will play the other seats.'),
      ),
    );
  }
}
