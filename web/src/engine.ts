// Loads the Rust engine (WebAssembly) and the RON content, and wraps every
// call in typed JSON. The simulation never touches the network, the clock or
// the screen; this is the only door in and out.

import init, { Engine as RawEngine } from './wasm/flyover_wasm.js';
import wasmUrl from './wasm/flyover_wasm_bg.wasm?url';
import type {
  Content, EventRec, FactionId, Mission, Orders, P, Preview, PublicView, SeatView, TargetInfo, Warning, WeakSpot,
} from './types';

const FILES = ['map', 'units', 'factions', 'rules', 'patch_notes', 'flavor'] as const;

export interface Resolved {
  state: string;
  events: EventRec[];
  hash: string;
}

export class Engine {
  readonly content: Content;
  private raw: RawEngine;

  private constructor(raw: RawEngine) {
    this.raw = raw;
    this.content = JSON.parse(raw.content_json()) as Content;
  }

  static async load(): Promise<Engine> {
    await init({ module_or_path: wasmUrl });
    const base = new URL('content/', document.baseURI);
    const texts = await Promise.all(
      FILES.map(async (f) => {
        const r = await fetch(new URL(`${f}.ron`, base));
        if (!r.ok) throw new Error(`Could not load content/${f}.ron (${r.status})`);
        return r.text();
      }),
    );
    const [map, units, factions, rules, patch, flavor] = texts;
    return new Engine(new RawEngine(map, units, factions, rules, patch, flavor));
  }

  newGame(seed: number, factions: FactionId[]): string {
    return this.raw.new_game(JSON.stringify({ seed, factions }));
  }
  resolve(state: string, orders: Orders[]): Resolved {
    const out = JSON.parse(this.raw.resolve(state, JSON.stringify(orders)));
    return { state: JSON.stringify(out.state), events: out.events, hash: out.hash };
  }
  seatView(state: string, seat: number): SeatView {
    return JSON.parse(this.raw.seat_view(state, seat));
  }
  publicView(state: string): PublicView {
    return JSON.parse(this.raw.public_view(state));
  }
  previews(state: string, orders: Orders): Preview[] {
    return JSON.parse(this.raw.previews(state, JSON.stringify(orders)));
  }
  preview(state: string, orders: Orders, sq: number, mission: Mission, waypoint: P | null): { preview: Preview | null; error: string | null } {
    return JSON.parse(this.raw.preview(state, JSON.stringify(orders), JSON.stringify({ sq, mission, waypoint })));
  }
  checkMission(state: string, sq: number, mission: Mission): string | null {
    return JSON.parse(this.raw.check_mission(state, sq, JSON.stringify(mission)));
  }
  advise(state: string, seat: number, orders: Orders): Warning[] {
    return JSON.parse(this.raw.advise(state, seat, JSON.stringify(orders)));
  }
  suggest(state: string, seat: number, level: 'easy' | 'normal'): Orders {
    return JSON.parse(this.raw.suggest(state, seat, level));
  }
  carryOver(state: string, seat: number): Orders {
    return JSON.parse(this.raw.carry_over(state, seat));
  }
  weakSpot(state: string, viewer: number, target: number): WeakSpot {
    return JSON.parse(this.raw.weak_spot(state, viewer, target));
  }
  bestTargets(state: string, seat: number): TargetInfo[] {
    return JSON.parse(this.raw.best_targets(state, seat));
  }
  stateHash(state: string): string {
    return this.raw.state_hash(state);
  }
}
