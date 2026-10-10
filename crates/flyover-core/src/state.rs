//! Game state, orders and the event log. Everything here serializes to JSON
//! for saves, the browser and the command-line tools.

use crate::content::{FactionId, UnitKind, P};
use serde::{Deserialize, Serialize};

pub const STATE_VERSION: u32 = 1;
pub const MAX_STRENGTH: u8 = 4;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub version: u32,
    pub seed: u64,
    /// The next night to resolve (1-based). Greater than the night count once the war is over.
    pub night: u8,
    pub seats: Vec<SeatState>,
    pub nodes: Vec<NodeState>,
    pub squadrons: Vec<Squadron>,
    pub next_sq: u32,
    pub effects: Vec<Effect>,
    pub intel: Vec<Intel>,
    /// Tonight's Patch Notes (index into patch_notes), when California is playing.
    pub patch: Option<u16>,
    /// Tonight's Harvest Shipment value, when the Grain Belt is playing.
    pub harvest: i32,
    pub pending: Vec<PendingBuild>,
    pub smoke: Vec<Smoke>,
    /// Seats tied for last place at dawn; their strikes score extra tonight.
    pub scrappy: Vec<u8>,
    /// Seats that hit Texas last night (Remember the Alamo).
    pub texas_grudge: Vec<u8>,
    /// Seats that ever hit Utah or a node Utah guarded (Oh My Heck).
    pub beehive_grudge: Vec<u8>,
    /// Grain Belt's free revenge strike: the seat that hit the Ball of Twine.
    pub revenge_against: Option<u8>,
    pub history: Vec<NightRecord>,
    /// Node health as everyone knows it (strikes are public; repairs are not).
    #[serde(default)]
    pub public_health: Vec<u8>,
    /// The White Sands Mass Driver has one shot per game.
    #[serde(default)]
    pub mass_driver_fired: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeatState {
    pub faction: FactionId,
    pub supply: i32,
    pub fuel: i32,
    pub score: i32,
    pub pride_lost: i32,
    pub income_supply: i32,
    pub income_fuel: i32,
    pub planes_lost: i32,
    pub kills: i32,
    pub steps_dealt: i32,
    pub steps_taken: i32,
    /// Project Bark Park: may retreat tonight (decided at dawn, public).
    #[serde(default)]
    pub doghouse: bool,
    /// Already spent a night at the park this game.
    #[serde(default)]
    pub been_to_park: bool,
    /// One shot of the Big Fetch, loaded and waiting (public).
    #[serde(default)]
    pub fetch: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeState {
    /// Seat index that holds the node (None = neutral or fallen).
    pub owner: Option<u8>,
    /// 2 = full output, 1 = half, 0 = knocked out.
    pub health: u8,
    /// Last night this node took a hit (repairs wait a night).
    pub hit_night: u8,
    /// False for a fallen faction's crown jewel: a landmark, nothing more.
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Squadron {
    pub id: u32,
    pub seat: u8,
    pub kind: UnitKind,
    pub strength: u8,
    pub base: u16,
    pub stars: u8,
    pub fights: u8,
    pub ace: bool,
    /// Standing order; carries over to the next night.
    pub order: Mission,
    pub waypoint: Option<P>,
    /// 0 = active; otherwise the night it rejoins (downed aces, the Longhorn).
    pub down_until: u8,
}

impl Squadron {
    pub fn active(&self) -> bool {
        self.down_until == 0 && self.strength > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    /// The Last Server Farm is down: no drones launch.
    NoDrones,
    /// Grand Coulee Dam hit: factories at half power.
    FactoriesHalf,
    /// Cheyenne Mountain sealed: radar blind.
    RadarSealed,
    /// Gulf Refinery hit: fuel income halved.
    FuelHalf,
    /// Hoover Dam hit: the Strip goes dark, zero Supply.
    StripDark,
    /// Fry Sauce Reserve hit: Utah's guards lose their first shot.
    NoFirstShot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub seat: u8,
    pub kind: EffectKind,
    /// Active for nights from..=until.
    pub from: u8,
    pub until: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingBuild {
    pub seat: u8,
    pub kind: UnitKind,
    /// Appears at the dawn before this night.
    pub night: u8,
    pub stars: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Smoke {
    pub node: u16,
    pub seat: u8,
    /// Active through the end of this night.
    pub until: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Intel {
    /// Last known health per node (what this seat has seen).
    pub known: Vec<u8>,
    pub ghosts: Vec<Ghost>,
    /// Zones revealed for this planning phase (scouted or bought).
    pub revealed: Vec<Reveal>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ghost {
    pub sq: u32,
    pub seat: u8,
    pub kind: UnitKind,
    pub strength: u8,
    pub node: u16,
    pub night: u8,
    pub ace: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reveal {
    pub zone: u16,
    /// Bought from Colorado: limited to what Colorado's radar covers.
    pub radar_of: Option<u8>,
    /// Cascadian overcast: scouts see only half.
    pub half: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NightRecord {
    pub night: u8,
    pub scores: Vec<i32>,
}

// --- Orders --------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "t")]
pub enum Mission {
    #[default]
    Rest,
    Attack {
        node: u16,
    },
    Guard {
        node: u16,
    },
    GuardZone {
        zone: u16,
    },
    Escort {
        sq: u32,
    },
    Sweep {
        zone: u16,
    },
    Scout {
        zone: u16,
    },
    JamZone {
        zone: u16,
    },
    JamEscort {
        sq: u32,
    },
    Rebase {
        node: u16,
    },
}

impl Mission {
    pub fn kind(&self) -> crate::content::OrderKind {
        use crate::content::OrderKind as K;
        match self {
            Mission::Rest => K::Rest,
            Mission::Attack { .. } => K::Attack,
            Mission::Guard { .. } | Mission::GuardZone { .. } => K::Guard,
            Mission::Escort { .. } => K::Escort,
            Mission::Sweep { .. } => K::Sweep,
            Mission::Scout { .. } => K::Scout,
            Mission::JamZone { .. } | Mission::JamEscort { .. } => K::Jam,
            Mission::Rebase { .. } => K::Rebase,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SqOrder {
    pub sq: u32,
    pub mission: Mission,
    #[serde(default)]
    pub waypoint: Option<P>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bet {
    /// The seat whose mission is bet on.
    pub on: u8,
    /// Bet that they hit this exact node...
    #[serde(default)]
    pub node: Option<u16>,
    /// ...or anything belonging to this seat.
    #[serde(default)]
    pub victim: Option<u8>,
    pub stake: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntelSale {
    pub price: i32,
    #[serde(default)]
    pub only: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntelRequest {
    pub zone: u16,
    pub max_price: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Orders {
    pub seat: u8,
    /// Squadron orders in priority order (fuel is spent top-down).
    #[serde(default)]
    pub missions: Vec<SqOrder>,
    #[serde(default)]
    pub builds: Vec<UnitKind>,
    #[serde(default)]
    pub repairs: Vec<u16>,
    #[serde(default)]
    pub harvest_bid: i32,
    #[serde(default)]
    pub bet: Option<Bet>,
    #[serde(default)]
    pub intel_sale: Option<IntelSale>,
    #[serde(default)]
    pub intel_request: Option<IntelRequest>,
    #[serde(default)]
    pub mass_driver: Option<u16>,
    #[serde(default)]
    pub revenge: Option<u16>,
    /// Spend tonight at Project Bark Park (only while in the doghouse).
    #[serde(default)]
    pub retreat: bool,
    /// Fire the Big Fetch at this node.
    #[serde(default)]
    pub fetch: Option<u16>,
}

// --- Events --------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SqSnap {
    pub sq: u32,
    pub seat: u8,
    pub kind: UnitKind,
    pub strength: u8,
    pub ace: bool,
    pub stars: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    pub from: u32,
    pub to: u32,
    pub hit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionTag {
    Attack,
    Guard,
    Sweep,
    Scout,
    Jam,
    Rebase,
    Smoke,
    Watch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreReason {
    Strike,
    Jewel,
    Splash,
    Ace,
    Ground,
    Objective,
    Casserole,
    MassDriver,
    Revenge,
    Fetch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Event {
    NightStart {
        night: u8,
        stage: u8,
        patch: Option<u16>,
        harvest_winner: Option<u8>,
        harvest_bid: i32,
        harvest_value: i32,
    },
    Launch {
        group: u32,
        seat: u8,
        mission: MissionTag,
        squadrons: Vec<SqSnap>,
        base: u16,
        target_node: Option<u16>,
        target_zone: Option<u16>,
    },
    /// Positions every few ticks: (tick, x, y).
    Track {
        group: u32,
        seat: u8,
        pts: Vec<(i32, i32, i32)>,
    },
    Spotted {
        group: u32,
        seat: u8,
        x: i32,
        y: i32,
    },
    Engage {
        id: u32,
        x: i32,
        y: i32,
        a_seat: u8,
        b_seat: u8,
        a_group: u32,
        b_group: u32,
        a: Vec<SqSnap>,
        b: Vec<SqSnap>,
        first_shot: Option<u8>,
        ace_duel: bool,
        rounds: Vec<Vec<Shot>>,
        escaped: Vec<u32>,
        after: Vec<SqSnap>,
    },
    Flak {
        site: u16,
        site_seat: u8,
        group: u32,
        seat: u8,
        shots: Vec<Shot>,
        x: i32,
        y: i32,
    },
    Splash {
        sq: u32,
        seat: u8,
        kind: UnitKind,
        ace: bool,
        by_seat: Option<u8>,
        by_sq: Option<u32>,
        by_site: Option<u16>,
        ground: bool,
        x: i32,
        y: i32,
    },
    Jiggle {
        sq: u32,
        seat: u8,
    },
    Abort {
        group: u32,
        seat: u8,
    },
    Strike {
        group: u32,
        seat: u8,
        node: u16,
        victim: Option<u8>,
        kinds: Vec<UnitKind>,
        hits: i32,
        steps: i32,
        health: u8,
        jewel: bool,
    },
    Smoke {
        group: u32,
        seat: u8,
        node: u16,
    },
    JamOn {
        group: u32,
        seat: u8,
        zone: u16,
    },
    Pivot {
        group: u32,
        seat: u8,
        node: u16,
    },
    Vanished {
        sq: u32,
        seat: u8,
        x: i32,
        y: i32,
    },
    LostInStatic {
        sq: u32,
        seat: u8,
        x: i32,
        y: i32,
    },
    MassDriver {
        seat: u8,
        node: u16,
        victim: Option<u8>,
        steps: i32,
        health: u8,
    },
    Revenge {
        seat: u8,
        node: u16,
        victim: Option<u8>,
        steps: i32,
        health: u8,
    },
    /// A faction spends the night at Project Bark Park: untouchable.
    BarkPark {
        seat: u8,
    },
    /// An attack on a faction at the park bounces off the fence.
    Bounce {
        seat: u8,
        node: u16,
        victim: u8,
        /// The flight group, for strikes; None for the Mass Driver and friends.
        group: Option<u32>,
    },
    /// The Big Fetch lands.
    Fetch {
        seat: u8,
        node: u16,
        victim: Option<u8>,
        steps: i32,
        health: u8,
    },
    /// Back from the park at dawn: fresh squadrons, Supply, the Big Fetch.
    ParkReturn {
        seat: u8,
        built: Vec<UnitKind>,
        supply: i32,
    },
    JewelEffect {
        seat: u8,
        kind: EffectKind,
        nights: u8,
    },
    BetWin {
        seat: u8,
        on: u8,
        node: Option<u16>,
        victim: Option<u8>,
        stake: i32,
        payout: i32,
    },
    BetLost {
        seat: u8,
        stake: i32,
    },
    Score {
        seat: u8,
        delta: i32,
        reason: ScoreReason,
        node: Option<u16>,
        victim: Option<u8>,
    },
    Capture {
        node: u16,
        seat: u8,
        prev: Option<u8>,
    },
    ScoutReport {
        seat: u8,
        zone: u16,
        half: bool,
    },
    IntelSold {
        seller: u8,
        buyer: u8,
        zone: u16,
        price: i32,
    },
    Grounded {
        seat: u8,
        sq: u32,
        reason: String,
    },
    Built {
        seat: u8,
        sq: u32,
        kind: UnitKind,
        node: u16,
    },
    BuildFailed {
        seat: u8,
        kind: UnitKind,
        reason: String,
    },
    Repaired {
        seat: u8,
        node: u16,
        health: u8,
        paid: bool,
    },
    Salvage {
        seat: u8,
        sq: u32,
        kind: UnitKind,
    },
    AceReturns {
        seat: u8,
        sq: u32,
        kind: UnitKind,
    },
    Casserole {
        seat: u8,
        host: Option<u8>,
        supply: i32,
    },
    Income {
        seat: u8,
        supply: i32,
        fuel: i32,
    },
    Dawn {
        night: u8,
        scores: Vec<i32>,
        scrappy: Vec<u8>,
        patch: Option<u16>,
        harvest: i32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventRec {
    pub tick: i32,
    /// Seen by everyone (the sky); otherwise only by `seats`.
    pub public: bool,
    #[serde(default)]
    pub seats: Vec<u8>,
    pub e: Event,
}

impl EventRec {
    pub fn visible_to(&self, seat: u8) -> bool {
        self.public || self.seats.contains(&seat)
    }
}
