// The glowing map of the West: dark navy, old state borders as faint dotted
// lines, faction territory as glowing outlines, and animated static east of
// the Mississippi. Shared by the planning window and the Theater.

import { Application, Container, Graphics, Texture, TilingSprite } from 'pixi.js';
import type { Content, FactionId, P } from '../types';
import { NAVY, NEUTRAL, drawNode, factionColor, nodeRadius } from '../theme';

export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export interface NodeState {
  owner: number | null;
  health: number;
  active: boolean;
  smoke?: boolean;
  exact?: boolean;
}

export class MapView {
  readonly content: Content;
  readonly root: HTMLElement;
  readonly app = new Application();
  readonly world = new Container();
  readonly layers = {
    base: new Container(),
    territory: new Container(),
    links: new Container(),
    nodes: new Container(),
    overlay: new Container(),
    units: new Container(),
    fx: new Container(),
  };
  readonly labels: HTMLDivElement;
  cam: Camera = { x: 0, y: 0, zoom: 1 };
  camTarget: Camera | null = null;
  shake = 0;
  seatFactions: FactionId[] = [];
  nodes: NodeState[] = [];
  /** Highlight rings drawn on top of nodes: node idx -> color. */
  highlights = new Map<number, number>();
  private staticSprite: TilingSprite | null = null;
  private nodeG = new Graphics();
  private linkG = new Graphics();
  private hiG = new Graphics();
  private labelEls = new Map<string, { el: HTMLElement; pos: P }>();
  private tickers: ((dt: number) => void)[] = [];
  private lastW = 0;
  private lastH = 0;
  private dirtyLabels = true;
  private destroyed = false;
  linkFlows: Map<number, number> | null = null;
  flowSeat: number | null = null;

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
      background: NAVY,
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
    this.world.addChild(L.base, L.territory, L.links, L.nodes, L.overlay, L.units, L.fx);
    L.links.addChild(this.linkG);
    L.nodes.addChild(this.nodeG, this.hiG);
    this.drawBase();
    this.drawLandmarks();
    this.app.ticker.add((t) => this.tick(t.deltaMS));
    // Follow the container, not just the window (panels resize, overlays show and hide).
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
    return Math.min(w / (this.content.width + 40), h / (this.content.height + 40));
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

  fit() {
    this.cam = { x: this.content.width / 2 + 10, y: this.content.height / 2, zoom: 1 };
    this.camTarget = null;
    this.dirtyLabels = true;
  }

  /** Ease the camera toward a target (the show's director uses this). */
  focus(x: number, y: number, zoom: number) {
    this.camTarget = { x, y, zoom };
  }

  zoomAt(sx: number, sy: number, factor: number) {
    const before = this.toMap(sx, sy);
    this.cam.zoom = Math.min(4, Math.max(0.8, this.cam.zoom * factor));
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
      const k = 1 - Math.pow(0.0025, dt / 1000);
      this.cam.x += (this.camTarget.x - this.cam.x) * k;
      this.cam.y += (this.camTarget.y - this.cam.y) * k;
      this.cam.zoom += (this.camTarget.zoom - this.cam.zoom) * k;
      this.dirtyLabels = true;
    }
    const s = this.scale;
    let sx = 0;
    let sy = 0;
    if (this.shake > 0.01) {
      sx = (Math.random() - 0.5) * this.shake * 14;
      sy = (Math.random() - 0.5) * this.shake * 14;
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
    if (this.staticSprite) {
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

  // --- Static base map ----------------------------------------------------------------

  private drawBase() {
    const c = this.content;
    const g = new Graphics();
    // Ocean grid, faint.
    for (let x = -200; x <= c.width + 200; x += 40) g.moveTo(x, -200).lineTo(x, c.height + 200);
    for (let y = -200; y <= c.height + 200; y += 40) g.moveTo(-200, y).lineTo(c.width + 200, y);
    g.stroke({ width: 0.5, color: 0x1b2d52, alpha: 0.35 });
    // Fringe states fade toward the Static.
    for (const poly of c.fringe) {
      g.poly(poly.flat()).fill({ color: 0x0c1730, alpha: 0.9 });
      dotted(g, poly, 0x2a3c63, 0.5);
    }
    this.layers.base.addChild(g);
    // The Static: animated noise east of the Mississippi.
    const tex = noiseTexture();
    const sprite = new TilingSprite({ texture: tex, width: c.width + 400, height: c.height + 400 });
    sprite.position.set(0, -200);
    sprite.alpha = 0.55;
    const mask = new Graphics();
    const river = c.mississippi;
    const pts: number[] = [river[0][0], -200];
    for (const p of river) pts.push(p[0], p[1]);
    pts.push(river[river.length - 1][0], c.height + 200, c.width + 400, c.height + 200, c.width + 400, -200);
    mask.poly(pts).fill({ color: 0xffffff });
    sprite.mask = mask;
    this.layers.base.addChild(sprite, mask);
    this.staticSprite = sprite;
    const edge = new Graphics();
    edge.poly(river.flat(), false).stroke({ width: 6, color: 0xb9c6dd, alpha: 0.12 });
    edge.poly(river.flat(), false).stroke({ width: 1.5, color: 0xdfe7f5, alpha: 0.45 });
    this.layers.base.addChild(edge);
  }

  /** Places that aren't nodes. Project Bark Park: a fenced lawn, a pond and a ball. */
  private drawLandmarks() {
    const g = new Graphics();
    for (const lm of this.content.landmarks ?? []) {
      const [x, y] = lm.pos;
      if (lm.id === 'bark_park') {
        g.roundRect(x - 9, y - 6, 18, 12, 3).fill({ color: 0x2f7d4a, alpha: 0.95 });
        g.roundRect(x - 9, y - 6, 18, 12, 3).stroke({ width: 0.8, color: 0xe9f5e1, alpha: 0.9 });
        for (let i = 0; i <= 6; i++) g.circle(x - 9 + i * 3, y - 6, 0.55).fill({ color: 0xffffff });
        g.ellipse(x - 3.5, y + 1.5, 3, 2).fill({ color: 0x6fb2ff, alpha: 0.9 });
        g.circle(x + 4, y + 1, 2.3).fill({ color: 0xd7f542 });
        g.moveTo(x + 2.3, y + 0.2).quadraticCurveTo(x + 4, y + 1.6, x + 5.7, y + 0.2).stroke({ width: 0.5, color: 0xffffff });
      }
      this.label(`lm-${lm.id}`, lm.name.toUpperCase(), [x, y + 13], 'label-landmark');
    }
    // Above the territory fills (Four Corners sits right on a border), under the nodes.
    this.layers.nodes.addChildAt(g, 0);
  }

  /** Redraw territory for the seated factions (unseated home regions are Fallen). */
  setSeats(factions: FactionId[]) {
    this.seatFactions = factions;
    const c = this.content;
    this.layers.territory.removeChildren();
    const fill = new Graphics();
    const glow = new Graphics();
    const lines = new Graphics();
    const regionKey = (r: string) => (r === 'BigEmpty' || !factions.includes(r as FactionId) ? 'neutral' : r);
    for (const z of c.zones) {
      const key = regionKey(z.region);
      const color = key === 'neutral' ? 0x132344 : factionColor(key as FactionId);
      const alpha = key === 'neutral' ? (z.region === 'BigEmpty' ? 0.85 : 0.6) : 0.1;
      fill.poly(z.poly.flat()).fill({ color: key === 'neutral' ? color : NAVY, alpha: key === 'neutral' ? alpha : 1 });
      if (key !== 'neutral') fill.poly(z.poly.flat()).fill({ color, alpha });
      dotted(lines, z.poly, 0x3b5487, 0.45);
    }
    // Glowing borders where regions meet.
    const edges = new Map<string, { a: P; b: P; regions: string[] }>();
    for (const z of c.zones) {
      const key = regionKey(z.region);
      for (let i = 0; i < z.poly.length; i++) {
        const a = z.poly[i];
        const b = z.poly[(i + 1) % z.poly.length];
        const k = a[0] < b[0] || (a[0] === b[0] && a[1] < b[1]) ? `${a}|${b}` : `${b}|${a}`;
        const e = edges.get(k) ?? { a, b, regions: [] };
        e.regions.push(key);
        edges.set(k, e);
      }
    }
    for (const e of edges.values()) {
      for (const key of new Set(e.regions)) {
        const interior = e.regions.length === 2 && e.regions[0] === e.regions[1];
        if (interior || key === 'neutral') continue;
        const color = factionColor(key as FactionId);
        glow.moveTo(e.a[0], e.a[1]).lineTo(e.b[0], e.b[1]).stroke({ width: 7, color, alpha: 0.12 });
        glow.moveTo(e.a[0], e.a[1]).lineTo(e.b[0], e.b[1]).stroke({ width: 3, color, alpha: 0.25 });
        glow.moveTo(e.a[0], e.a[1]).lineTo(e.b[0], e.b[1]).stroke({ width: 1.2, color, alpha: 0.9 });
      }
    }
    glow.blendMode = 'add';
    this.layers.territory.addChild(fill, lines, glow);
    // Zone labels.
    for (const z of c.zones) {
      const key = regionKey(z.region);
      const fallen = z.region !== 'BigEmpty' && key === 'neutral';
      this.label(`zone-${z.idx}`, z.name.toUpperCase() + (fallen ? '\nFALLEN' : ''), z.center, 'label-zone');
    }
    this.dirtyLabels = true;
  }

  // --- Nodes and links ------------------------------------------------------------------

  setNodes(states: NodeState[]) {
    this.nodes = states;
    this.drawNodes();
    this.drawLinks();
  }

  ownerFaction(seat: number | null): FactionId | null {
    return seat === null || seat === undefined ? null : this.seatFactions[seat] ?? null;
  }

  drawNodes() {
    const g = this.nodeG;
    g.clear();
    for (const n of this.content.nodes) {
      const st = this.nodes[n.idx] ?? { owner: null, health: 2, active: true };
      const color = n.kind === 'Lake' ? 0x6fb2ff : st.owner === null ? NEUTRAL : factionColor(this.ownerFaction(st.owner));
      const r = nodeRadius(n.kind);
      if (n.objective > 0 && n.kind !== 'Lake') {
        g.circle(n.pos[0], n.pos[1], r * 2.1).stroke({ width: 1, color: 0xffffff, alpha: 0.25 });
      }
      drawNode(g, n.kind, n.pos[0], n.pos[1], r, color, st.health, st.active);
      if (st.smoke) {
        g.circle(n.pos[0] - 4, n.pos[1] - 6, 6).fill({ color: 0xbfc7d5, alpha: 0.35 });
        g.circle(n.pos[0] + 4, n.pos[1] - 8, 5).fill({ color: 0xbfc7d5, alpha: 0.3 });
      }
      if (st.exact === false && st.owner !== null) {
        g.circle(n.pos[0] + r, n.pos[1] - r, 1.6).fill({ color: 0xffffff, alpha: 0.5 });
      }
    }
    this.drawHighlights();
  }

  drawHighlights() {
    const g = this.hiG;
    g.clear();
    for (const [idx, color] of this.highlights) {
      const n = this.content.nodes[idx];
      const r = nodeRadius(n.kind) * 2.2;
      g.circle(n.pos[0], n.pos[1], r).stroke({ width: 2, color, alpha: 0.9 });
      g.circle(n.pos[0], n.pos[1], r + 3).stroke({ width: 4, color, alpha: 0.2 });
    }
  }

  /** Links: faint everywhere; thick glowing flow lines for one seat's network. */
  drawLinks() {
    const g = this.linkG;
    g.clear();
    for (const l of this.content.links) {
      const a = this.content.nodes[l.a].pos;
      const b = this.content.nodes[l.b].pos;
      const oa = this.nodes[l.a]?.owner ?? null;
      const ob = this.nodes[l.b]?.owner ?? null;
      const same = oa !== null && oa === ob;
      const color = same ? factionColor(this.ownerFaction(oa)) : 0x5a6f96;
      const flow = this.linkFlows?.get(l.idx) ?? 0;
      const mine = this.flowSeat !== null && same && oa === this.flowSeat;
      const broken = mine && ((this.nodes[l.a]?.health ?? 2) === 0 || (this.nodes[l.b]?.health ?? 2) === 0);
      if (broken) {
        dashed(g, a, b, 0xff3b3b, 2, 0.9);
        continue;
      }
      if (mine && flow > 0) {
        g.moveTo(a[0], a[1]).lineTo(b[0], b[1]).stroke({ width: 1.5 + flow * 0.9, color, alpha: 0.28 });
        g.moveTo(a[0], a[1]).lineTo(b[0], b[1]).stroke({ width: 1.2, color, alpha: 0.9 });
      } else if (l.kind === 'Pipeline') {
        dashed(g, a, b, color, 1, same ? 0.45 : 0.22, 2, 4);
      } else if (l.kind === 'Rail') {
        dashed(g, a, b, color, 1, same ? 0.5 : 0.25, 7, 3);
      } else {
        g.moveTo(a[0], a[1]).lineTo(b[0], b[1]).stroke({ width: 0.8, color, alpha: same ? 0.45 : 0.2 });
      }
    }
  }

  setFlows(seat: number | null, links: [number, number, number][] | null) {
    this.flowSeat = seat;
    this.linkFlows = links ? new Map(links.map(([l, s, f]) => [l, s + f])) : null;
    this.drawLinks();
  }

  // --- HTML labels (crisp at any zoom) ----------------------------------------------------

  label(id: string, text: string, pos: P, cls: string): HTMLElement {
    let entry = this.labelEls.get(id);
    if (!entry) {
      const el = document.createElement('div');
      this.labels.appendChild(el);
      entry = { el, pos };
      this.labelEls.set(id, entry);
    }
    entry.el.className = `map-label ${cls}`;
    entry.el.textContent = text;
    entry.pos = pos;
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

  placeLabels() {
    this.dirtyLabels = false;
    const zoom = this.cam.zoom;
    this.labels.style.setProperty('--zoom', String(zoom));
    for (const { el, pos } of this.labelEls.values()) {
      const [x, y] = this.toScreen(pos);
      el.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px)`;
    }
  }

  markLabelsDirty() {
    this.dirtyLabels = true;
  }

  // --- Hit testing ---------------------------------------------------------------------------

  nodeAt(sx: number, sy: number, radiusPx = 16): number | null {
    let best: number | null = null;
    let bd = radiusPx * radiusPx;
    for (const n of this.content.nodes) {
      const [x, y] = this.toScreen(n.pos);
      const d = (x - sx) ** 2 + (y - sy) ** 2;
      if (d < bd) {
        bd = d;
        best = n.idx;
      }
    }
    return best;
  }

  zoneAt(p: P): number | null {
    for (const z of this.content.zones) if (pointInPoly(p, z.poly)) return z.idx;
    return null;
  }
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

function dotted(g: Graphics, poly: P[], color: number, alpha: number) {
  for (let i = 0; i < poly.length; i++) {
    dashed(g, poly[i], poly[(i + 1) % poly.length], color, 0.7, alpha, 2, 5);
  }
}

export function dashed(g: Graphics, a: P, b: P, color: number, width: number, alpha: number, on = 5, off = 4) {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len = Math.hypot(dx, dy);
  if (len < 0.01) return;
  const ux = dx / len;
  const uy = dy / len;
  for (let d = 0; d < len; d += on + off) {
    const e = Math.min(len, d + on);
    g.moveTo(a[0] + ux * d, a[1] + uy * d).lineTo(a[0] + ux * e, a[1] + uy * e);
  }
  g.stroke({ width, color, alpha });
}

let noiseTex: Texture | null = null;
function noiseTexture(): Texture {
  if (noiseTex) return noiseTex;
  const c = document.createElement('canvas');
  c.width = 128;
  c.height = 128;
  const ctx = c.getContext('2d')!;
  const img = ctx.createImageData(128, 128);
  for (let i = 0; i < img.data.length; i += 4) {
    const v = Math.random() * 255;
    img.data[i] = v * 0.8;
    img.data[i + 1] = v * 0.85;
    img.data[i + 2] = v;
    img.data[i + 3] = 160 + Math.random() * 95;
  }
  ctx.putImageData(img, 0, 0);
  noiseTex = Texture.from(c);
  return noiseTex;
}
