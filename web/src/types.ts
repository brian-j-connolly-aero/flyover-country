// Mirrors of the Rust types that cross the WebAssembly boundary as JSON.

import type { MorningData } from './bus';

export type P = [number, number];

export type FactionId = 'Pacific' | 'Cascadia' | 'FrontRange' | 'Texas' | 'Dreamland' | 'GrainBelt' | 'Beehive';
export const FACTIONS: FactionId[] = ['Pacific', 'Cascadia', 'FrontRange', 'Texas', 'Dreamland', 'GrainBelt', 'Beehive'];

export type Region = FactionId | 'BigEmpty';
export type SiteKind = 'HQ' | 'Factory' | 'Airfield' | 'Jewel';
export type UnitKind = 'Fighter' | 'Bomber';

export interface ZoneDef { id: string; name: string; region: Region; center: P; poly: P[] }
export interface SiteDef { id: string; name: string; kind: SiteKind; faction: FactionId; pos: P; tagline: string }
export interface PrizeDef { id: string; name: string; pos: P; tagline: string }
export interface LandmarkDef { id: string; name: string; pos: P; tagline: string }

export interface FactionDef {
  id: FactionId; name: string; short: string; home: string; color: string;
  leader: string; title: string; ace: string; perk: string; perk_text: string;
}

export interface RulesDef {
  nights: number; open_factories: number; open_hqs: number;
  stage_names: [string, string, string];
  ticks: number; launch_tick: number; speed: number;
  start_supply: number; max_squadrons: number; planes: number; big_bomber_planes: number;
  costs: { fighter: number; bomber: number; repair: number; move_fast: number };
  health: { hq: number; factory: number; airfield: number; jewel: number };
  income: { hq: number; factory_base: number; prize: number };
  combat: { rounds: number; hit: number; ace_hit: number; flak_hit: number; flak_shots: number; hq_flak_shots: number; bomb_hit: number; crop_duster_hit: number };
  patrol: { radius: number; mountain_radius: number };
  score: { pip: number; big_pip: number; plane: number; prize: number };
  bark_park: { first_night: number; last_night: number; gap: number; low_squadrons: number; supply: number; squadrons: number; fetch_pips: number; fetch_tick: number };
}

export interface LeaderLines {
  intro: string[]; dawn: string[]; ahead: string[]; behind: string[]; hit: string[]; landed: string[];
  park: string[]; back: string[]; fetch: string[]; win: string[]; lose: string[];
}
export interface Radio { intercept: string[]; jumped: string[]; hit: string[]; miss: string[]; prize: string[] }
export interface Flavor {
  leaders: Record<FactionId, LeaderLines>;
  radio: Record<FactionId, Radio>;
  headlines: Record<string, string[]>;
}

export interface Content {
  width: number; height: number;
  zones: ZoneDef[]; fringe: P[][]; mississippi: P[];
  sites: SiteDef[]; prizes: PrizeDef[]; landmarks: LandmarkDef[];
  factions: FactionDef[]; rules: RulesDef; flavor: Flavor;
}

// --- State ---------------------------------------------------------------------------------

export interface Stats { pips_dealt: number; pips_taken: number; downed: number; lost: number; prizes: number }
export interface SeatState {
  faction: FactionId; score: number; supply: number; doghouse: boolean; been_to_park: boolean;
  fetch: boolean; built: [number, number]; stats: Stats;
}
export interface SiteState { health: number; max: number; active: boolean }
export interface Squadron { id: number; seat: number; kind: UnitKind; planes: number; max: number; ace: boolean; num: number }
export type Target = { t: 'Site'; site: number } | { t: 'Prize'; prize: number };
export interface PatrolMark { seat: number; at: P; r: number }
export interface RaidMark { seat: number; from: P; to: P; target: Target }

export interface GameState {
  version: number; seed: number; night: number;
  seats: SeatState[]; sites: SiteState[]; prizes: { owner: number | null }[];
  squadrons: Squadron[]; next_sq: number;
  last: { patrols: PatrolMark[]; raids: RaidMark[] };
  history: number[][];
}

export type Mission = { t: 'Rest' } | { t: 'Patrol'; at: P } | { t: 'Fly'; target: Target };
export interface MissionOrder { sq: number; mission: Mission }
export interface Orders {
  seat: number; missions: MissionOrder[]; builds: UnitKind[]; repairs: number[];
  retreat: boolean; fetch: number | null;
}

export function emptyOrders(seat: number): Orders {
  return { seat, missions: [], builds: [], repairs: [], retreat: false, fetch: null };
}

// --- Events ----------------------------------------------------------------------------------

export interface Loss { sq: number; planes: number }
export type ScoreReason = 'Strike' | 'Downed' | 'Prize' | 'Fetch';
export interface DawnReport {
  income: number[]; prizes: (number | null)[]; built: UnitKind[][]; returned: number[]; doghouse: number[];
}

export type GameEvent =
  | { t: 'NightStart'; night: number; stage: number }
  | { t: 'BarkPark'; seat: number }
  | { t: 'Patrol'; seat: number; sq: number; at: P; r: number; planes: number }
  | { t: 'Launch'; raid: number; seat: number; sqs: number[]; from: P; to: P; target: Target; fighters: number; bombers: number; arrive: number }
  | { t: 'Intercept'; raid: number; seat: number; patrol: number; patrol_seat: number; at: P; raid_lost: Loss[]; patrol_lost: number; cloud: boolean; wiped: boolean }
  | { t: 'Flak'; raid: number; seat: number; site: number; victim: number; raid_lost: Loss[] }
  | { t: 'Strike'; raid: number; seat: number; site: number; victim: number; bombers: number; hits: number; health: number }
  | { t: 'Bounce'; raid: number; seat: number; site: number; victim: number }
  | { t: 'Hold'; raid: number; seat: number; prize: number }
  | { t: 'PrizeFight'; raid: number; seat: number; prize: number; holder: number; raid_lost: Loss[]; holder_lost: Loss[]; taken: boolean }
  | { t: 'Fetch'; seat: number; site: number; victim: number; hits: number; health: number; bounced: boolean }
  | { t: 'Score'; seat: number; delta: number; reason: ScoreReason }
  | { t: 'SquadronDown'; sq: number; seat: number; ace: boolean }
  | ({ t: 'Dawn' } & DawnReport);

export type EventRec = { tick: number } & GameEvent;

// --- Planning previews and tips -------------------------------------------------------------

export interface RaidPreview { target: Target; from: P; to: P; sqs: number[]; fighters: number; bombers: number; crosses: PatrolMark[] }
export interface Cover { site: number; patrols: number[] }
export interface Preview { orders: Orders; raids: RaidPreview[]; cover: Cover[]; spend: number; supply_left: number }
export interface Tip { key: string; level: number; text: string; site: number | null; sq: number | null }
export interface Checked { preview: Preview; tips: Tip[] }

// --- Saves -------------------------------------------------------------------------------------

export type SeatKind = 'human' | 'easy' | 'normal';
export interface SeatConfig { faction: FactionId; kind: SeatKind }
export interface GameOptions { timer: number; theater: 'single' | 'two'; sound: boolean }
export interface SaveNight { orders: Orders[]; hash: string }
export interface SaveFile {
  version: 2;
  id: string; created: number; updated: number;
  seed: number; factions: FactionId[]; seats: SeatConfig[]; options: GameOptions;
  nights: SaveNight[];
  /** Game state JSON at each dawn; snapshots[0] is the start of Night 1. */
  snapshots: string[];
  tutorial?: boolean;
  /** How many lines each dog has said so far, so nobody repeats themselves. */
  voice?: Record<string, number>;
  /** The morning card for the night about to be planned. */
  morning?: MorningData;
}
