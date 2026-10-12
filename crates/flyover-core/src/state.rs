//! Game state, orders and the event log. Everything here is plain data that
//! round-trips through JSON: a save is the seed plus every night's orders.

use crate::content::{FactionId, UnitKind};
use crate::geom::P;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub version: u32,
    pub seed: u64,
    /// The night about to be played, from 1. Past the last night, the game is over.
    pub night: u8,
    pub seats: Vec<SeatState>,
    /// One entry per site on the map. Sites of factions not in the game are inactive.
    pub sites: Vec<SiteState>,
    pub prizes: Vec<PrizeState>,
    pub squadrons: Vec<Squadron>,
    pub next_sq: u32,
    /// Last night's patrols and raids, left on the map as faded marks.
    pub last: LastNight,
    /// Everyone's score after each night (index 0 = after Night 1).
    pub history: Vec<Vec<i32>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SeatState {
    pub faction: Option<FactionId>,
    pub score: i32,
    pub supply: i32,
    /// Can go to Project Bark Park tonight.
    pub doghouse: bool,
    /// Has used the park this game.
    pub been_to_park: bool,
    /// The Big Fetch is loaded and can be thrown.
    pub fetch: bool,
    /// Squadrons of each kind ever fielded (fighters, bombers), for names like "Bombers 3".
    pub built: [u8; 2],
    pub stats: Stats,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub pips_dealt: i32,
    pub pips_taken: i32,
    pub downed: i32,
    pub lost: i32,
    pub prizes: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SiteState {
    pub health: i32,
    pub max: i32,
    pub active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrizeState {
    pub owner: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Squadron {
    pub id: u32,
    pub seat: u8,
    pub kind: UnitKind,
    pub planes: i32,
    pub max: i32,
    #[serde(default)]
    pub ace: bool,
    /// Its number among its owner's squadrons, for names like "Bombers 2".
    pub num: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LastNight {
    pub patrols: Vec<PatrolMark>,
    pub raids: Vec<RaidMark>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatrolMark {
    pub seat: u8,
    pub at: P,
    pub r: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RaidMark {
    pub seat: u8,
    pub from: P,
    pub to: P,
    pub target: Target,
}

/// Where a raid is headed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Target {
    Site { site: u16 },
    Prize { prize: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "t")]
pub enum Mission {
    #[default]
    Rest,
    /// Fighters fly a circle here and jump any enemy raid that crosses it.
    Patrol { at: P },
    /// Fly to a target: bombers strike, fighters escort them (or sweep alone),
    /// and at a prize everyone tries to take it.
    Fly { target: Target },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionOrder {
    pub sq: u32,
    pub mission: Mission,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Orders {
    pub seat: u8,
    #[serde(default)]
    pub missions: Vec<MissionOrder>,
    #[serde(default)]
    pub builds: Vec<UnitKind>,
    /// One entry per pip to repair (a site can appear more than once).
    #[serde(default)]
    pub repairs: Vec<u16>,
    #[serde(default)]
    pub retreat: bool,
    #[serde(default)]
    pub fetch: Option<u16>,
}

impl Orders {
    pub fn empty(seat: u8) -> Orders {
        Orders {
            seat,
            ..Default::default()
        }
    }
    pub fn mission(&self, sq: u32) -> Mission {
        self.missions
            .iter()
            .find(|m| m.sq == sq)
            .map(|m| m.mission)
            .unwrap_or_default()
    }
}

/// One squadron's losses in a fight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Loss {
    pub sq: u32,
    pub planes: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreReason {
    Strike,
    Downed,
    Prize,
    Fetch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Event {
    NightStart {
        night: u8,
        stage: u8,
    },
    /// This faction spends the night at Project Bark Park.
    BarkPark {
        seat: u8,
    },
    Patrol {
        seat: u8,
        sq: u32,
        at: P,
        r: i32,
        planes: i32,
    },
    Launch {
        raid: u32,
        seat: u8,
        sqs: Vec<u32>,
        from: P,
        to: P,
        target: Target,
        fighters: i32,
        bombers: i32,
        /// Tick it arrives at the target.
        arrive: u32,
    },
    /// A patrol jumped a raid that flew into its circle.
    Intercept {
        raid: u32,
        seat: u8,
        patrol: u32,
        patrol_seat: u8,
        at: P,
        raid_lost: Vec<Loss>,
        patrol_lost: i32,
        /// A Cascadian raid in the clouds: the patrol only got one round.
        cloud: bool,
        /// The raid had no planes left afterwards.
        wiped: bool,
    },
    Flak {
        raid: u32,
        seat: u8,
        site: u16,
        victim: u8,
        raid_lost: Vec<Loss>,
    },
    Strike {
        raid: u32,
        seat: u8,
        site: u16,
        victim: u8,
        bombers: i32,
        hits: i32,
        health: i32,
    },
    /// The target's owner is at Project Bark Park: the bombs bounce.
    Bounce {
        raid: u32,
        seat: u8,
        site: u16,
        victim: u8,
    },
    /// First to arrive: this raid holds the prize.
    Hold {
        raid: u32,
        seat: u8,
        prize: u16,
    },
    /// A raid fought the holders of a prize.
    PrizeFight {
        raid: u32,
        seat: u8,
        prize: u16,
        holder: u8,
        raid_lost: Vec<Loss>,
        holder_lost: Vec<Loss>,
        /// The arriving raid took the prize.
        taken: bool,
    },
    Fetch {
        seat: u8,
        site: u16,
        victim: u8,
        hits: i32,
        health: i32,
        bounced: bool,
    },
    Score {
        seat: u8,
        delta: i32,
        reason: ScoreReason,
    },
    /// A squadron lost its last plane. Every pilot got a parachute.
    SquadronDown {
        sq: u32,
        seat: u8,
        ace: bool,
    },
    Dawn(Box<DawnReport>),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DawnReport {
    /// Supply earned per seat.
    pub income: Vec<i32>,
    /// Prize owners after tonight.
    pub prizes: Vec<Option<u8>>,
    /// Squadrons built per seat.
    pub built: Vec<Vec<UnitKind>>,
    /// Seats back from Project Bark Park.
    pub returned: Vec<u8>,
    /// Seats that may go to the park next night.
    pub doghouse: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventRec {
    pub tick: u32,
    #[serde(flatten)]
    pub e: Event,
}
