// TypeScript mirrors of the JSON the Rust engine produces and accepts.

export type FactionId = 'Pacific' | 'Cascadia' | 'FrontRange' | 'Texas' | 'Dreamland' | 'GrainBelt' | 'Beehive';
export const FACTIONS: FactionId[] = ['Pacific', 'Cascadia', 'FrontRange', 'Texas', 'Dreamland', 'GrainBelt', 'Beehive'];

export type P = [number, number];

export type NodeKind =
  | 'Capital' | 'City' | 'Fuel' | 'Factory' | 'Airbase' | 'Radar' | 'AirDefense'
  | 'Junction' | 'CrownJewel' | 'Lake' | 'Boneyard' | 'MassDriver' | 'TruckStop';
export type LinkKind = 'Highway' | 'Rail' | 'Pipeline';
export type Region = FactionId | 'BigEmpty';
export type UnitKind =
  | 'Fighter' | 'Bomber' | 'Scout' | 'Jammer' | 'Drone' | 'Floatplane'
  | 'Watchtower' | 'Longhorn' | 'Prototype' | 'CropDuster' | 'Gull';
export type CombatClass = 'Fighter' | 'Bomber' | 'Scout' | 'Jammer' | 'Drone';
export type OrderKind = 'Rest' | 'Attack' | 'Guard' | 'Escort' | 'Sweep' | 'Scout' | 'Jam' | 'Rebase';

export interface MapNode {
  idx: number; id: string; name: string; kind: NodeKind; home: FactionId | null;
  zone: number; pos: P; objective: number; tagline: string;
}
export interface Zone { idx: number; id: string; name: string; region: Region; center: P; poly: P[]; radius: number }
export interface Link { idx: number; a: number; b: number; kind: LinkKind }

export interface UnitDef {
  kind: UnitKind; name: string; faction: FactionId | null; class: CombatClass;
  cost: number; speed: number; fuel_rate: number; strike: number; strike_mult: number;
  air: number[]; ad_evasion: number; armor: number; orders: OrderKind[];
  one_way: boolean; stealth: boolean; radar: number; buildable: boolean; max_owned: number; desc: string;
}

export interface FactionDef {
  id: FactionId; name: string; short: string; home: string; color: string; shape: string;
  emblem: string; leader: string; plays_like: string; ace: string; unique: UnitKind; jewel: string;
  card: { signature: [string, string]; quirk: [string, string]; unique: [string, string]; jewel: [string, string]; ace: [string, string] };
}

export interface PatchNote { version: string; text: string; effect: Record<string, number> }

export interface AceDef { callsign: string; launch: string[]; kill: string[]; duel: string[]; downed: string[]; returns: string[] }
export interface LeaderDef {
  name: string; title: string; bio: string; pick: string[]; dawn: string[]; winning: string[];
  losing: string[]; our_jewel_hit: string[]; victory: string[]; defeat: string[];
}
export interface Flavor {
  chatter: Record<string, Record<string, string[]>>;
  aces: Record<string, AceDef>;
  leaders: Record<string, LeaderDef>;
  advisor: Record<string, Record<string, string[]>>;
  headlines: Record<string, string[]>;
  curtain: Record<string, string[]>;
}

export interface Rules {
  nights: number; open_season: number; gloves_off: number; stage_names: [string, string, string]; ticks: number;
  economy: Record<string, number>;
  scoring: Record<string, number>;
  factions: Record<string, number>;
  radar: Record<string, number>;
  air_defense: { range: number; grainbelt_range: number } & Record<string, unknown>;
}

export interface Content {
  width: number; height: number;
  nodes: MapNode[]; zones: Zone[]; links: Link[];
  fringe: P[][]; mississippi: P[];
  units: UnitDef[]; factions: FactionDef[]; rules: Rules; patch_notes: PatchNote[]; flavor: Flavor;
}

// --- Orders -----------------------------------------------------------------------

export type Mission =
  | { t: 'Rest' }
  | { t: 'Attack'; node: number }
  | { t: 'Guard'; node: number }
  | { t: 'GuardZone'; zone: number }
  | { t: 'Escort'; sq: number }
  | { t: 'Sweep'; zone: number }
  | { t: 'Scout'; zone: number }
  | { t: 'JamZone'; zone: number }
  | { t: 'JamEscort'; sq: number }
  | { t: 'Rebase'; node: number };

export interface SqOrder { sq: number; mission: Mission; waypoint: P | null }
export interface Bet { on: number; node: number | null; victim: number | null; stake: number }
export interface Orders {
  seat: number;
  missions: SqOrder[];
  builds: UnitKind[];
  repairs: number[];
  harvest_bid: number;
  bet: Bet | null;
  intel_sale: { price: number; only: number | null } | null;
  intel_request: { zone: number; max_price: number } | null;
  mass_driver: number | null;
  revenge: number | null;
}

// --- State ----------------------------------------------------------------------

export interface Squadron {
  id: number; seat: number; kind: UnitKind; strength: number; base: number; stars: number;
  fights: number; ace: boolean; order: Mission; waypoint: P | null; down_until: number;
}

export type EffectKind = 'NoDrones' | 'FactoriesHalf' | 'RadarSealed' | 'FuelHalf' | 'StripDark' | 'NoFirstShot';
export interface Effect { seat: number; kind: EffectKind; from: number; until: number }
export interface NightRecord { night: number; scores: number[] }

export interface PublicSeat {
  seat: number; faction: FactionId; score: number; pride_lost: number; planes_lost: number;
  kills: number; steps_dealt: number; steps_taken: number; scrappy: boolean;
}

export interface NodeView {
  owner: number | null; health: number; exact: boolean; smoke: boolean; capturable: boolean;
  active: boolean; attack: string | null; guard: string | null;
}
export interface EnemyView { sq: number; seat: number; kind: UnitKind; strength: number; base: number; ace: boolean; night: number }
export interface BuildOption { kind: UnitKind; cost: number; ok: boolean; reason: string | null }
export interface Income { supply: number; fuel: number; raw_supply: number; raw_fuel: number; links: [number, number, number][] }
export interface Reveal { zone: number; radar_of: number | null; half: boolean }
export interface PendingBuild { seat: number; kind: UnitKind; night: number; stars: number }

export interface SeatView {
  seat: number; faction: FactionId; night: number; nights: number; stage: number; over: boolean;
  supply: number; fuel: number; income_supply: number; income_fuel: number; cap_supply: number; cap_fuel: number;
  build_capacity: number; seats: PublicSeat[]; nodes: NodeView[]; squadrons: Squadron[]; enemies: EnemyView[];
  revealed: Reveal[]; flows: Income; effects: Effect[]; patch: number | null; harvest: number;
  texas_grudge: number[]; beehive_grudge: number[]; revenge_against: number | null; mass_driver_holder: number | null; mass_driver_fired: boolean;
  pending: PendingBuild[]; builds: BuildOption[]; carry_over: Orders; history: NightRecord[];
}

export interface PublicNode { owner: number | null; health: number; active: boolean }
export interface PublicView {
  night: number; nights: number; stage: number; over: boolean; seats: PublicSeat[]; nodes: PublicNode[];
  effects: Effect[]; patch: number | null; harvest: number; mass_driver_holder: number | null; mass_driver_fired: boolean; history: NightRecord[];
}

export type RiskColor = 'Green' | 'Yellow' | 'Red';
export interface Preview {
  sq: number; path: P[]; fuel: number; risk: number; color: RiskColor; reason: string;
  ad_sites: number; escorted: boolean; jammed: boolean; in_static: boolean;
}

export interface Warning { key: string; severity: number; params: Record<string, string>; sq: number | null; node: number | null }
export interface WeakSpot { seat: number; nodes: number[]; supply_loss: number; fuel_loss: number; pct: number; resource: string }
export interface TargetInfo { node: number; value: number; why: string }

// --- Events ---------------------------------------------------------------------

export interface SqSnap { sq: number; seat: number; kind: UnitKind; strength: number; ace: boolean; stars: number }
export interface Shot { from: number; to: number; hit: boolean }
export type MissionTag = 'Attack' | 'Guard' | 'Sweep' | 'Scout' | 'Jam' | 'Rebase' | 'Smoke' | 'Watch';
export type ScoreReason = 'Strike' | 'Jewel' | 'Splash' | 'Ace' | 'Ground' | 'Objective' | 'Casserole' | 'MassDriver' | 'Revenge';

export type GameEvent =
  | { t: 'NightStart'; night: number; stage: number; patch: number | null; harvest_winner: number | null; harvest_bid: number; harvest_value: number }
  | { t: 'Launch'; group: number; seat: number; mission: MissionTag; squadrons: SqSnap[]; base: number; target_node: number | null; target_zone: number | null }
  | { t: 'Track'; group: number; seat: number; pts: [number, number, number][] }
  | { t: 'Spotted'; group: number; seat: number; x: number; y: number }
  | {
      t: 'Engage'; id: number; x: number; y: number; a_seat: number; b_seat: number; a_group: number; b_group: number;
      a: SqSnap[]; b: SqSnap[]; first_shot: number | null; ace_duel: boolean; rounds: Shot[][]; escaped: number[]; after: SqSnap[];
    }
  | { t: 'Flak'; site: number; site_seat: number; group: number; seat: number; shots: Shot[]; x: number; y: number }
  | {
      t: 'Splash'; sq: number; seat: number; kind: UnitKind; ace: boolean; by_seat: number | null; by_sq: number | null;
      by_site: number | null; ground: boolean; x: number; y: number;
    }
  | { t: 'Jiggle'; sq: number; seat: number }
  | { t: 'Abort'; group: number; seat: number }
  | { t: 'Strike'; group: number; seat: number; node: number; victim: number | null; kinds: UnitKind[]; hits: number; steps: number; health: number; jewel: boolean }
  | { t: 'Smoke'; group: number; seat: number; node: number }
  | { t: 'JamOn'; group: number; seat: number; zone: number }
  | { t: 'Pivot'; group: number; seat: number; node: number }
  | { t: 'Vanished'; sq: number; seat: number; x: number; y: number }
  | { t: 'LostInStatic'; sq: number; seat: number; x: number; y: number }
  | { t: 'MassDriver'; seat: number; node: number; victim: number | null; steps: number; health: number }
  | { t: 'Revenge'; seat: number; node: number; victim: number | null; steps: number; health: number }
  | { t: 'JewelEffect'; seat: number; kind: EffectKind; nights: number }
  | { t: 'BetWin'; seat: number; on: number; node: number | null; victim: number | null; stake: number; payout: number }
  | { t: 'BetLost'; seat: number; stake: number }
  | { t: 'Score'; seat: number; delta: number; reason: ScoreReason; node: number | null; victim: number | null }
  | { t: 'Capture'; node: number; seat: number; prev: number | null }
  | { t: 'ScoutReport'; seat: number; zone: number; half: boolean }
  | { t: 'IntelSold'; seller: number; buyer: number; zone: number; price: number }
  | { t: 'Grounded'; seat: number; sq: number; reason: string }
  | { t: 'Built'; seat: number; sq: number; kind: UnitKind; node: number }
  | { t: 'BuildFailed'; seat: number; kind: UnitKind; reason: string }
  | { t: 'Repaired'; seat: number; node: number; health: number; paid: boolean }
  | { t: 'Salvage'; seat: number; sq: number; kind: UnitKind }
  | { t: 'AceReturns'; seat: number; sq: number; kind: UnitKind }
  | { t: 'Casserole'; seat: number; host: number | null; supply: number }
  | { t: 'Income'; seat: number; supply: number; fuel: number }
  | { t: 'Dawn'; night: number; scores: number[]; scrappy: number[]; patch: number | null; harvest: number };

export interface EventRec { tick: number; public: boolean; seats: number[]; e: GameEvent }

// --- Session ----------------------------------------------------------------------

export type SeatKind = 'human' | 'easy' | 'normal';
export interface SeatConfig { faction: FactionId; kind: SeatKind }
export interface GameOptions { timer: number; theater: 'two' | 'single'; sound: boolean }

export interface SaveNight { orders: Orders[]; hash: string }
export interface SaveFile {
  version: 1;
  id: string;
  created: number;
  updated: number;
  seed: number;
  factions: FactionId[];
  seats: SeatConfig[];
  options: GameOptions;
  nights: SaveNight[];
  /** Game state JSON at each dawn; snapshots[0] is the start of Night 1. */
  snapshots: string[];
  tutorial?: boolean;
}

export const EMPTY_ORDERS = (seat: number): Orders => ({
  seat, missions: [], builds: [], repairs: [], harvest_bid: 0, bet: null,
  intel_sale: null, intel_request: null, mass_driver: null, revenge: null,
});
