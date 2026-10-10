// The planning window and the Theater window talk over BroadcastChannel; no
// server needed. The Theater only ever receives public information.

import type { EventRec, FactionId, PublicView, SeatKind } from './types';

export interface DawnData {
  night: number;
  nights: number;
  stage: number;
  factions: FactionId[];
  kinds: SeatKind[];
  pv: PublicView;
  headline: string;
  highlights: string[];
  intro: boolean;
}

export interface ShowData {
  night: number;
  factions: FactionId[];
  kinds: SeatKind[];
  events: EventRec[];
  before: PublicView;
  after: PublicView;
  last: boolean;
}

export interface FinalData {
  factions: FactionId[];
  kinds: SeatKind[];
  pv: PublicView;
}

export type ToTheater =
  | { type: 'idle'; pv: PublicView | null; factions: FactionId[]; kinds: SeatKind[]; text?: string }
  | { type: 'dawn'; data: DawnData }
  | { type: 'curtain'; seat: number; faction: FactionId; pv: PublicView; factions: FactionId[]; kinds: SeatKind[]; deadline: number | null; waiting: FactionId[] }
  | { type: 'show'; data: ShowData }
  | { type: 'final'; data: FinalData }
  | { type: 'control'; action: 'pause' | 'speed' | 'skip' | 'replay'; value?: number }
  | { type: 'sound'; on: boolean };

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
