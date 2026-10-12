// The map of the West as a printed postcard: cream land, soft blue sea, each
// faction's home washed in its ink, and a band of TV static east of the
// Mississippi. At night the same map dims to twilight. Shared by the planning
// screen and the Theater.

import { Application, Container, Graphics, Texture, TilingSprite } from 'pixi.js';
import type { Content, FactionId, GameState, P, PatrolMark, RaidMark } from '../types';
import {
  FRINGE, INK, INK_2, LAND, NIGHT, OCEAN, PAPER, dashedCircle, dashedLine, drawPips, drawPrize, drawSite, factionColor, mix,
} from '../theme';

/** Another place a label may go if its first spot is taken: a = 0 centered on p, 1 to its right, -1 to its left. */
export interface LabelAlt {
  p: P;
  a: -1 | 0 | 1;
}

interface LabelEntry {
  el: HTMLElement;
  pos: P;
  alts: LabelAlt[] | null;
  w: number;
  h: number;
}

export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export const SITE_R = 10;
export const PRIZE_R = 10;

export class MapView {
  readonly content: Content;
  readonly root: HTMLElement;
  readonly app = new Application();
  readonly world = new Container();
  readonly layers = {
    base: new Container(),
    territory: new Container(),
    marks: new Container(),
    night: new Container(),
    plan: new Container(),
    sites: new Container(),
    units: new Container(),
    fx: new Container(),
  };
  readonly labels: HTMLDivElement;
  cam: Camera = { x: 0, y: 0, zoom: 1 };
  camTarget: Camera | null = null;
  shake = 0;
  seatFactions: FactionId[] = [];
  state: GameState | null = null;
  /** Rings drawn around sites or prizes while dragging: key "s3" / "p1" -> color. */
  highlights = new Map<string, number>();
  private siteG = new Graphics();
  private hiG = new Graphics();
  private marksG = new Graphics();
  private nightG = new Graphics();
  private staticSprite: TilingSprite | null = null;
  private labelEls = new Map<string, LabelEntry>();
  /** Screen space kept clear at the edges (the Theater's scoreboard). */
  insets = { top: 0, bottom: 0 };
  private tickers: ((dt: number) => void)[] = [];
  private lastW = 0;
  private lastH = 0;
  private dirtyLabels = true;
  private destroyed = false;
  private nightK = 0;
  private staticClock = 0;

  constructor(root: HTMLElement, content: Content) {
    this.root = root;
    this.content = content;
    this.labels = document.createElement('div');
    this.labels.className = 'map-labels';
    this.cam = { x: content.width / 2, y: content.height / 2, zoom: 1 };
  }

  async init(): Promise<void> {
    await this.app.init({
      resizeTo: this.root,
      background: OCEAN,
      antialias: true,
      autoDensity: true,
      resolution: Math.min(window.devicePixelRatio || 1, 2),
      preference: 'webgl',
    });
    this.app.canvas.classList.add('map-canvas');
    this.root.appendChild(this.app.canvas);
    this.root.appendChild(this.labels);
    this.app.stage.addChild(this.world);
    const L = this.layers;
    this.world.addChild(L.base, L.territory, L.marks, L.night, L.plan, L.sites, L.units, L.fx);
    L.marks.addChild(this.marksG);
    L.night.addChild(this.nightG);
    L.sites.addChild(this.hiG, this.siteG);
    this.drawBase();
    this.app.ticker.add((t) => this.tick(t.deltaMS));
    this.resizer = new ResizeObserver(() => {
      if (!this.destroyed) this.app.resize();
    });
    this.resizer.observe(this.root);
  }
  private resizer: ResizeObserver | null = null;

  destroy() {
    this.destroyed = true;
    this.resizer?.disconnect();
    this.app.destroy(true, { children: true });
    this.labels.remove();
  }

  onTick(f: (dt: number) => void): () => void {
    this.tickers.push(f);
    return () => {
      this.tickers = this.tickers.filter((x) => x !== f);
    };
  }

  // --- Camera -------------------------------------------------------------------

  get fitScale(): number {
    const w = this.app.screen.width;
    const h = this.app.screen.height;
    return Math.min(w / (this.content.width * 0.86), h / (this.content.height * 0.86));
  }

  get scale(): number {
    return this.fitScale * this.cam.zoom;
  }

  toScreen(p: P): P {
    const s = this.scale;
    return [this.app.screen.width / 2 + (p[0] - this.cam.x) * s, this.app.screen.height / 2 + (p[1] - this.cam.y) * s];
  }

  toMap(x: number, y: number): P {
    const s = this.scale;
    return [(x - this.app.screen.width / 2) / s + this.cam.x, (y - this.app.screen.height / 2) / s + this.cam.y];
  }

  /** Frame the playable West (the Static is cropped off the right edge). */
  fit() {
    this.cam = this.home();
    this.camTarget = null;
    this.dirtyLabels = true;
  }

  /** The whole West in view: every site and prize, clear of the screen's insets. */
  home(): Camera {
    const c = this.content;
    const w = this.app.screen?.width ?? 0;
    const h = this.app.screen?.height ?? 0;
    if (!w || !h) return { x: c.width * 0.45, y: c.height * 0.47, zoom: 1 };
    const pts = [...c.sites.map((s) => s.pos), ...c.prizes.map((p) => p.pos), ...c.landmarks.map((l) => l.pos)];
    const pad = 46;
    const x0 = Math.min(...pts.map((p) => p[0])) - pad;
    const x1 = Math.max(...pts.map((p) => p[0])) + pad;
    const y0 = Math.min(...pts.map((p) => p[1])) - pad;
    const y1 = Math.max(...pts.map((p) => p[1])) + pad;
    const avail = Math.max(100, h - this.insets.top - this.insets.bottom);
    const scale = Math.min(w / (x1 - x0), avail / (y1 - y0));
    return { x: (x0 + x1) / 2, y: (y0 + y1) / 2 - (this.insets.top - this.insets.bottom) / 2 / scale, zoom: scale / this.fitScale };
  }

  focus(x: number, y: number, zoom: number) {
    this.camTarget = { x, y, zoom };
  }

  zoomAt(sx: number, sy: number, factor: number) {
    const before = this.toMap(sx, sy);
    this.cam.zoom = Math.min(3.5, Math.max(0.85, this.cam.zoom * factor));
    const after = this.toMap(sx, sy);
    this.cam.x += before[0] - after[0];
    this.cam.y += before[1] - after[1];
    this.camTarget = null;
    this.dirtyLabels = true;
  }

  panBy(dx: number, dy: number) {
    const s = this.scale;
    this.cam.x -= dx / s;
    this.cam.y -= dy / s;
    this.camTarget = null;
    this.dirtyLabels = true;
  }

  private tick(dt: number) {
    if (this.destroyed) return;
    if (this.camTarget) {
      const k = 1 - Math.pow(0.004, dt / 1000);
      this.cam.x += (this.camTarget.x - this.cam.x) * k;
      this.cam.y += (this.camTarget.y - this.cam.y) * k;
      this.cam.zoom += (this.camTarget.zoom - this.cam.zoom) * k;
      this.dirtyLabels = true;
    }
    const s = this.scale;
    let sx = 0;
    let sy = 0;
    if (this.shake > 0.01) {
      sx = (Math.random() - 0.5) * this.shake * 12;
      sy = (Math.random() - 0.5) * this.shake * 12;
      this.shake *= Math.pow(0.02, dt / 1000);
      this.dirtyLabels = true;
    }
    this.world.scale.set(s);
    this.world.position.set(this.app.screen.width / 2 - this.cam.x * s + sx, this.app.screen.height / 2 - this.cam.y * s + sy);
    if (this.app.screen.width !== this.lastW || this.app.screen.height !== this.lastH) {
      this.lastW = this.app.screen.width;
      this.lastH = this.app.screen.height;
      this.dirtyLabels = true;
    }
    // The Static flickers, slowly: a printed map shouldn't buzz.
    this.staticClock += dt;
    if (this.staticSprite && this.staticClock > 140) {
      this.staticClock = 0;
      this.staticSprite.tilePosition.set(Math.random() * 128, Math.random() * 128);
    }
    for (const f of [...this.tickers]) {
      try {
        f(dt);
      } catch (e) {
        console.error(e);
      }
    }
    if (this.dirtyLabels) this.placeLabels();
  }

  // --- The printed map ---------------------------------------------------------------

  private drawBase() {
    const c = this.content;
    const g = new Graphics();
    // A faint graticule on the sea, like an old chart.
    for (let x = -200; x <= c.width + 200; x += 60) g.moveTo(x, -200).lineTo(x, c.height + 200);
    for (let y = -200; y <= c.height + 200; y += 60) g.moveTo(-200, y).lineTo(c.width + 200, y);
    g.stroke({ width: 0.6, color: mix(OCEAN, INK, 0.25), alpha: 0.35 });
    for (const poly of c.fringe) g.poly(poly.flat()).fill({ color: FRINGE });
    for (const z of c.zones) g.poly(z.poly.flat()).fill({ color: LAND });
    this.layers.base.addChild(g);
    // The Static: everything east of the Mississippi, where the signal died.
    const sprite = new TilingSprite({ texture: noiseTexture(), width: c.width + 600, height: c.height + 600 });
    sprite.position.set(0, -300);
    sprite.alpha = 0.9;
    const mask = new Graphics();
    const river = c.mississippi;
    const pts: number[] = [river[0][0], -300];
    for (const p of river) pts.push(p[0], p[1]);
    pts.push(river[river.length - 1][0], c.height + 300, c.width + 600, c.height + 300, c.width + 600, -300);
    mask.poly(pts).fill({ color: 0xffffff });
    sprite.mask = mask;
    const edge = new Graphics();
    edge.poly(river.flat(), false).stroke({ width: 2.2, color: INK_2, alpha: 0.75 });
    this.layers.base.addChild(sprite, mask, edge);
    this.staticSprite = sprite;
    // Night: the same postcard printed in blue ink. A multiply wash keeps
    // land, sea and each faction's color apart; everything that moves sits on top.
    this.nightG.rect(-400, -400, c.width + 1200, c.height + 800).fill({ color: NIGHT });
    this.nightG.blendMode = 'multiply';
    this.nightG.alpha = 0;
  }

  /** 0 = day (planning), 1 = full night (the show). */
  setNight(k: number) {
    this.nightK = k;
    this.nightG.alpha = k;
    this.labels.classList.toggle('night', k > 0.5);
  }

  get night(): number {
    return this.nightK;
  }

  /** Wash each seated faction's home in its ink; unseated homes join the Big Empty. */
  setSeats(factions: FactionId[]) {
    this.seatFactions = factions;
    const c = this.content;
    this.layers.territory.removeChildren();
    const fill = new Graphics();
    const lines = new Graphics();
    const borders = new Graphics();
    const seated = (r: string) => r !== 'BigEmpty' && factions.includes(r as FactionId);
    for (const z of c.zones) {
      if (seated(z.region)) fill.poly(z.poly.flat()).fill({ color: factionColor(z.region as FactionId), alpha: 0.2 });
    }
    // Old state lines, thin and dotted.
    for (const z of c.zones) {
      for (let i = 0; i < z.poly.length; i++) dashedLine(lines, z.poly[i], z.poly[(i + 1) % z.poly.length], 2.5, 0.8, INK_2, 0.35);
    }
    // Each faction's outer border in its ink.
    const edges = new Map<string, { a: P; b: P; regions: string[] }>();
    for (const z of c.zones) {
      for (let i = 0; i < z.poly.length; i++) {
        const a = z.poly[i];
        const b = z.poly[(i + 1) % z.poly.length];
        const k = a[0] < b[0] || (a[0] === b[0] && a[1] < b[1]) ? `${a}|${b}` : `${b}|${a}`;
        const e = edges.get(k) ?? { a, b, regions: [] };
        e.regions.push(z.region);
        edges.set(k, e);
      }
    }
    for (const e of edges.values()) {
      for (const r of new Set(e.regions)) {
        const interior = e.regions.length === 2 && e.regions[0] === e.regions[1];
        if (interior || !seated(r)) continue;
        const col = mix(factionColor(r as FactionId), INK, 0.25);
        borders.moveTo(e.a[0], e.a[1]).lineTo(e.b[0], e.b[1]).stroke({ width: 2.2, color: col, alpha: 0.9 });
      }
    }
    this.layers.territory.addChild(fill, lines, borders);
    this.removeLabels('zone-');
    for (const z of c.zones) {
      const fallen = z.region !== 'BigEmpty' && !seated(z.region);
      this.label(`zone-${z.id}`, z.name.toUpperCase(), z.center, fallen ? 'label-zone fallen' : 'label-zone');
    }
    this.dirtyLabels = true;
  }

  ownerFaction(seat: number | null | undefined): FactionId | null {
    return seat === null || seat === undefined ? null : this.seatFactions[seat] ?? null;
  }

  /** Sites, prizes and the dog park, from a game state. */
  setState(st: GameState) {
    this.state = st;
    this.drawSites();
  }

  drawSites() {
    const g = this.siteG;
    g.clear();
    const st = this.state;
    const c = this.content;
    // Project Bark Park, a fenced lawn with a pond and a ball.
    for (const lm of c.landmarks) {
      if (lm.id !== 'bark_park') continue;
      const [x, y] = lm.pos;
      g.roundRect(x - 11 + 1.2, y - 7 + 1.4, 22, 14, 3).fill({ color: INK, alpha: 0.35 });
      g.roundRect(x - 11, y - 7, 22, 14, 3).fill({ color: 0x8fbf6a }).stroke({ width: 1.4, color: INK });
      for (let i = 0; i <= 7; i++) g.moveTo(x - 10 + i * 2.85, y - 7).lineTo(x - 10 + i * 2.85, y - 4.5);
      g.stroke({ width: 0.8, color: INK, alpha: 0.7 });
      g.ellipse(x - 4, y + 1.5, 3.4, 2.2).fill({ color: 0x6fa8d6 }).stroke({ width: 0.7, color: INK, alpha: 0.6 });
      g.circle(x + 4.5, y + 1, 2.6).fill({ color: 0xd7e63e }).stroke({ width: 0.7, color: INK });
      this.label(`lm-${lm.id}`, lm.name, [x, y + 15], 'label-landmark', around(lm.pos, 11));
    }
    c.sites.forEach((s, i) => {
      const ss = st?.sites[i];
      const active = ss ? ss.active : this.seatFactions.includes(s.faction);
      if (!active) {
        this.removeLabel(`site-${i}`);
        return;
      }
      const color = factionColor(s.faction);
      const r = s.kind === 'HQ' ? SITE_R * 1.15 : SITE_R;
      drawSite(g, s.kind, s.pos[0], s.pos[1], r, color, ss ? ss.health : 3);
      if (ss) drawPips(g, s.pos[0], s.pos[1] + r + 4.5, ss.health, ss.max, 3.2);
      this.label(`site-${i}`, s.name, [s.pos[0], s.pos[1] + r + 12], 'label-site', around(s.pos, r));
    });
    c.prizes.forEach((p, i) => {
      const owner = st?.prizes[i]?.owner;
      const color = owner === null || owner === undefined ? null : factionColor(this.ownerFaction(owner));
      drawPrize(g, p.pos[0], p.pos[1], PRIZE_R, color);
      this.label(`prize-${i}`, p.name, [p.pos[0], p.pos[1] + PRIZE_R + 8], 'label-prize', around(p.pos, PRIZE_R));
    });
    this.drawHighlights();
  }

  drawHighlights() {
    const g = this.hiG;
    g.clear();
    for (const [key, color] of this.highlights) {
      const i = Number(key.slice(1));
      const pos = key[0] === 's' ? this.content.sites[i]?.pos : this.content.prizes[i]?.pos;
      if (!pos) continue;
      g.circle(pos[0], pos[1], SITE_R * 1.9).fill({ color, alpha: 0.18 }).stroke({ width: 2.4, color, alpha: 0.95 });
    }
  }

  /** Last night's patrols and raids, faint and dashed: a hint of habits. */
  setMarks(last: { patrols: PatrolMark[]; raids: RaidMark[] } | null, except: number | null = null) {
    const g = this.marksG;
    g.clear();
    if (!last) return;
    for (const r of last.raids) {
      if (r.seat === except) continue;
      dashedLine(g, r.from, r.to, 5, 1.6, factionColor(this.ownerFaction(r.seat)), 0.5);
    }
    for (const p of last.patrols) {
      if (p.seat === except) continue;
      const col = factionColor(this.ownerFaction(p.seat));
      g.circle(p.at[0], p.at[1], p.r).fill({ color: col, alpha: 0.06 });
      dashedCircle(g, p.at[0], p.at[1], p.r, 5, 1.6, col, 0.55);
    }
  }

  // --- HTML labels (crisp at any zoom) ----------------------------------------------------

  label(id: string, text: string, pos: P, cls: string, alts: LabelAlt[] | null = null): HTMLElement {
    let entry = this.labelEls.get(id);
    if (!entry) {
      const el = document.createElement('div');
      this.labels.appendChild(el);
      entry = { el, pos, alts, w: -1, h: -1 };
      this.labelEls.set(id, entry);
    }
    const name = `map-label ${cls}`;
    if (entry.el.className !== name || entry.el.textContent !== text) {
      entry.el.className = name;
      entry.el.textContent = text;
      entry.w = -1;
    }
    entry.pos = pos;
    entry.alts = alts;
    this.dirtyLabels = true;
    return entry.el;
  }

  removeLabel(id: string) {
    const e = this.labelEls.get(id);
    if (e) {
      e.el.remove();
      this.labelEls.delete(id);
    }
  }

  removeLabels(prefix: string) {
    for (const id of [...this.labelEls.keys()]) if (id.startsWith(prefix)) this.removeLabel(id);
  }

  /**
   * Put every label on screen. Labels that name a place try a few spots
   * around it and take the first one that doesn't cover another label or
   * another place's token.
   */
  placeLabels() {
    this.dirtyLabels = false;
    this.labels.style.setProperty('--zoom', String(this.cam.zoom));
    this.labels.style.setProperty('--scale', String(this.scale));
    const s = this.scale;
    const taken: number[][] = [];
    const tokens = (pos: P, r: number) => {
      const [x, y] = this.toScreen(pos);
      taken.push([x - r * s, y - r * s, x + r * s, y + (r + 6) * s]);
    };
    this.content.sites.forEach((site, i) => {
      if (!this.state || this.state.sites[i].active) tokens(site.pos, SITE_R);
    });
    this.content.prizes.forEach((p) => tokens(p.pos, PRIZE_R));
    const free = (r: number[]) => !taken.some((t) => r[0] < t[2] && r[2] > t[0] && r[1] < t[3] && r[3] > t[1]);
    for (const e of this.labelEls.values()) {
      const { el } = e;
      if (!e.alts) {
        const [x, y] = this.toScreen(e.pos);
        el.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px)`;
        el.style.translate = '';
        continue;
      }
      if (e.w < 0) {
        e.w = el.offsetWidth;
        e.h = el.offsetHeight;
      }
      const options: LabelAlt[] = [{ p: e.pos, a: 0 }, ...e.alts];
      let pick: { x: number; y: number; a: number; r: number[] } | null = null;
      for (const o of options) {
        const [x, y] = this.toScreen(o.p);
        const r = o.a === 0 ? [x - e.w / 2, y - e.h / 2, x + e.w / 2, y + e.h / 2] : o.a === 1 ? [x, y - e.h / 2, x + e.w, y + e.h / 2] : [x - e.w, y - e.h / 2, x, y + e.h / 2];
        const fit = { x, y, a: o.a, r: [r[0] - 2, r[1] - 1, r[2] + 2, r[3] + 1] };
        if (free(fit.r)) {
          pick = fit;
          break;
        }
      }
      // Nowhere to go: hide it until the camera moves in (hovering still names it).
      el.style.visibility = pick ? '' : 'hidden';
      if (!pick) continue;
      taken.push(pick.r);
      el.style.transform = `translate(${pick.x.toFixed(1)}px, ${pick.y.toFixed(1)}px)`;
      el.style.translate = pick.a === 0 ? '' : pick.a === 1 ? '0 -50%' : '-100% -50%';
    }
  }

  markLabelsDirty() {
    this.dirtyLabels = true;
  }

  // --- Hit testing ---------------------------------------------------------------------------

  /** The site or prize nearest a screen point, within `radiusPx`. */
  pickAt(sx: number, sy: number, radiusPx = 22): { site: number } | { prize: number } | null {
    let best: { site: number } | { prize: number } | null = null;
    let bd = radiusPx * radiusPx;
    this.content.sites.forEach((s, i) => {
      if (this.state && !this.state.sites[i].active) return;
      const [x, y] = this.toScreen(s.pos);
      const d = (x - sx) ** 2 + (y - sy) ** 2;
      if (d < bd) {
        bd = d;
        best = { site: i };
      }
    });
    this.content.prizes.forEach((p, i) => {
      const [x, y] = this.toScreen(p.pos);
      const d = (x - sx) ** 2 + (y - sy) ** 2;
      if (d < bd) {
        bd = d;
        best = { prize: i };
      }
    });
    return best;
  }

  regionAt(p: P): string | null {
    for (const z of this.content.zones) if (pointInPoly(p, z.poly)) return z.region;
    return null;
  }
}

/** Spots around a token for its label: above, right, left. */
function around(pos: P, r: number): LabelAlt[] {
  return [
    { p: [pos[0], pos[1] - r - 9], a: 0 },
    { p: [pos[0] + r + 3, pos[1]], a: 1 },
    { p: [pos[0] - r - 3, pos[1]], a: -1 },
  ];
}

export function pointInPoly(p: P, poly: P[]): boolean {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const [xi, yi] = poly[i];
    const [xj, yj] = poly[j];
    if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

let noiseTex: Texture | null = null;
/** Paper-toned static: grain in ink and cream, not neon. */
function noiseTexture(): Texture {
  if (noiseTex) return noiseTex;
  const c = document.createElement('canvas');
  c.width = 128;
  c.height = 128;
  const ctx = c.getContext('2d')!;
  const img = ctx.createImageData(128, 128);
  const paper = [0xc9, 0xbd, 0xa6];
  const ink = [0x4a, 0x42, 0x3a];
  for (let i = 0; i < img.data.length; i += 4) {
    const t = Math.random();
    for (let k = 0; k < 3; k++) img.data[i + k] = paper[k] + (ink[k] - paper[k]) * t;
    img.data[i + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
  noiseTex = Texture.from(c);
  return noiseTex;
}

export { PAPER };
