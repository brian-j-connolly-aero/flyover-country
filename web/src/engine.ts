// Loads the Rust engine (WebAssembly) and the RON content, and wraps every
// call in typed JSON. The simulation never touches the network, the clock or
// the screen; this is the only door in and out.

import init, { Engine as RawEngine } from './wasm/flyover_wasm.js';
import wasmUrl from './wasm/flyover_wasm_bg.wasm?url';
import type { Checked, Content, EventRec, FactionId, GameState, Mission, Orders } from './types';

const FILES = ['map', 'factions', 'rules', 'flavor'] as const;

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
    const [map, factions, rules, flavor] = texts;
    return new Engine(new RawEngine(map, factions, rules, flavor));
  }

  newGame(seed: number, factions: FactionId[]): string {
    return this.raw.new_game(JSON.stringify({ seed, factions }));
  }
  resolve(state: string, orders: Orders[]): Resolved {
    const out = JSON.parse(this.raw.resolve(state, JSON.stringify(orders)));
    const next = JSON.stringify(out.state);
    return { state: next, events: out.events, hash: this.raw.state_hash(next) };
  }
  parse(state: string): GameState {
    return JSON.parse(state) as GameState;
  }
  check(state: string, orders: Orders): Checked {
    return JSON.parse(this.raw.check(state, JSON.stringify(orders)));
  }
  missionError(state: string, sq: number, mission: Mission): string {
    return this.raw.mission_error(state, sq, JSON.stringify(mission));
  }
  patrolError(state: string, seat: number, x: number, y: number): string {
    return this.raw.patrol_error(state, seat, Math.round(x), Math.round(y));
  }
  siteError(state: string, seat: number, site: number): string {
    return this.raw.site_error(state, seat, site);
  }
  suggest(state: string, seat: number, level: 'easy' | 'normal'): Orders {
    return JSON.parse(this.raw.suggest(state, seat, level));
  }
  carryOver(state: string, prev: Orders): Orders {
    return JSON.parse(this.raw.carry_over(state, JSON.stringify(prev)));
  }
  stateHash(state: string): string {
    return this.raw.state_hash(state);
  }
}
