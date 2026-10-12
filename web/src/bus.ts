// The planning window and the Theater window talk over BroadcastChannel; no
// server needed. The Theater never sees tonight's orders: only the game
// state (all of which is public) and what happened once a night resolves.

import type { EventRec, FactionId, GameState, SeatKind } from './types';
import type { Caption } from './show/summary';
import type { Mood } from './art/dogs';

/** The morning card: last night in a few lines, one dog, and what's new. */
export interface MorningData {
  /** The night about to be planned. */
  night: number;
  nights: number;
  factions: FactionId[];
  kinds: SeatKind[];
  state: GameState;
  report: { headline: string; highlights: string[]; gained: number[] } | null;
  speaker: { seat: number; mood: Mood; line: string } | null;
  news: string[];
  intro: boolean;
}

export interface FinalData {
  factions: FactionId[];
  kinds: SeatKind[];
  state: GameState;
  headline: string;
  line: string | null;
}

export interface ShowData {
  night: number;
  factions: FactionId[];
  kinds: SeatKind[];
  events: EventRec[];
  before: GameState;
  after: GameState;
  captions: Caption[];
  /** What to put up when the show ends. */
  next: { type: 'morning'; data: MorningData } | { type: 'final'; data: FinalData } | { type: 'back' };
}

export type ToTheater =
  | { type: 'idle'; state: GameState | null; factions: FactionId[]; kinds: SeatKind[]; text?: string }
  | { type: 'morning'; data: MorningData }
  | { type: 'curtain'; seat: number; faction: FactionId; state: GameState; factions: FactionId[]; kinds: SeatKind[]; deadline: number | null; waiting: FactionId[] }
  | { type: 'show'; data: ShowData }
  | { type: 'final'; data: FinalData }
  | { type: 'control'; action: 'pause' | 'speed' | 'skip' | 'replay'; value?: number }
  | { type: 'sound'; on: boolean; music?: boolean };

export type FromTheater =
  | { type: 'ready' }
  | { type: 'continue' }
  | { type: 'status'; phase: string; paused: boolean; speed: number; t: number };

const NAME = 'flyover-country';

export class Bus<In, Out> {
  private ch: BroadcastChannel;
  constructor(onMessage: (m: In) => void) {
    this.ch = new BroadcastChannel(NAME);
    this.ch.onmessage = (e) => onMessage(e.data as In);
  }
  send(m: Out) {
    this.ch.postMessage(m);
  }
  close() {
    this.ch.close();
  }
}
