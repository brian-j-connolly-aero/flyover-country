//! Data-driven content: the map, the factions and the rules live in Rusty
//! Object Notation (RON) files, parsed here and compiled into a [`World`] with
//! integer indices for the simulation.

use crate::geom::{inside, P};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FactionId {
    Pacific,
    Cascadia,
    FrontRange,
    Texas,
    Dreamland,
    GrainBelt,
    Beehive,
}

impl FactionId {
    pub const ALL: [FactionId; 7] = [
        FactionId::Pacific,
        FactionId::Cascadia,
        FactionId::FrontRange,
        FactionId::Texas,
        FactionId::Dreamland,
        FactionId::GrainBelt,
        FactionId::Beehive,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Which home region a zone belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    Pacific,
    Cascadia,
    FrontRange,
    Texas,
    Dreamland,
    GrainBelt,
    Beehive,
    BigEmpty,
}

impl Region {
    pub fn faction(self) -> Option<FactionId> {
        Some(match self {
            Region::Pacific => FactionId::Pacific,
            Region::Cascadia => FactionId::Cascadia,
            Region::FrontRange => FactionId::FrontRange,
            Region::Texas => FactionId::Texas,
            Region::Dreamland => FactionId::Dreamland,
            Region::GrainBelt => FactionId::GrainBelt,
            Region::Beehive => FactionId::Beehive,
            Region::BigEmpty => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteKind {
    HQ,
    Factory,
    Airfield,
    Jewel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UnitKind {
    Fighter,
    Bomber,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZoneDef {
    pub id: String,
    pub name: String,
    pub region: Region,
    pub center: P,
    pub poly: Vec<P>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteDef {
    pub id: String,
    pub name: String,
    pub kind: SiteKind,
    pub faction: FactionId,
    pub pos: P,
    #[serde(default)]
    pub tagline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrizeDef {
    pub id: String,
    pub name: String,
    pub pos: P,
    #[serde(default)]
    pub tagline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LandmarkDef {
    pub id: String,
    pub name: String,
    pub pos: P,
    #[serde(default)]
    pub tagline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapDef {
    pub width: i32,
    pub height: i32,
    pub zones: Vec<ZoneDef>,
    #[serde(default)]
    pub fringe: Vec<Vec<P>>,
    #[serde(default)]
    pub mississippi: Vec<P>,
    pub sites: Vec<SiteDef>,
    pub prizes: Vec<PrizeDef>,
    #[serde(default)]
    pub landmarks: Vec<LandmarkDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactionDef {
    pub id: FactionId,
    pub name: String,
    pub short: String,
    pub home: String,
    pub color: String,
    pub leader: String,
    pub title: String,
    pub ace: String,
    pub perk: String,
    pub perk_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Costs {
    pub fighter: i32,
    pub bomber: i32,
    pub repair: i32,
    /// What any new squadron costs California while its perk works.
    pub move_fast: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Health {
    pub hq: i32,
    pub factory: i32,
    pub airfield: i32,
    pub jewel: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Income {
    pub hq: i32,
    pub factory_base: i32,
    pub prize: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Combat {
    pub rounds: i32,
    pub hit: i32,
    pub ace_hit: i32,
    pub flak_hit: i32,
    pub flak_shots: i32,
    pub hq_flak_shots: i32,
    pub bomb_hit: i32,
    pub crop_duster_hit: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patrol {
    pub radius: i32,
    pub mountain_radius: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Score {
    pub pip: i32,
    pub big_pip: i32,
    pub plane: i32,
    pub prize: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarkPark {
    pub first_night: u8,
    pub last_night: u8,
    pub gap: i32,
    pub low_squadrons: usize,
    pub supply: i32,
    pub squadrons: usize,
    pub fetch_pips: i32,
    pub fetch_tick: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RulesDef {
    pub nights: u8,
    pub open_factories: u8,
    pub open_hqs: u8,
    pub stage_names: (String, String, String),
    pub ticks: u32,
    pub launch_tick: u32,
    pub speed: i32,
    pub start_supply: i32,
    pub max_squadrons: usize,
    pub planes: i32,
    pub big_bomber_planes: i32,
    pub costs: Costs,
    pub health: Health,
    pub income: Income,
    pub combat: Combat,
    pub patrol: Patrol,
    pub score: Score,
    pub bark_park: BarkPark,
    /// Faction perks on (off only for balance experiments).
    #[serde(default = "yes")]
    pub perks: bool,
}

fn yes() -> bool {
    true
}

/// The raw text of each content file.
#[derive(Clone, Copy, Debug)]
pub struct ContentFiles<'a> {
    pub factions: &'a str,
    pub map: &'a str,
    pub rules: &'a str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Content {
    pub factions: Vec<FactionDef>,
    pub map: MapDef,
    pub rules: RulesDef,
}

impl Content {
    pub fn parse(files: &ContentFiles) -> Result<Content, String> {
        let factions: Vec<FactionDef> =
            ron::from_str(files.factions).map_err(|e| format!("factions.ron: {e}"))?;
        let map: MapDef = ron::from_str(files.map).map_err(|e| format!("map.ron: {e}"))?;
        let rules: RulesDef = ron::from_str(files.rules).map_err(|e| format!("rules.ron: {e}"))?;
        Ok(Content {
            factions,
            map,
            rules,
        })
    }
}

/// Content plus lookups the simulation needs.
#[derive(Clone, Debug)]
pub struct World {
    pub content: Content,
    /// For each faction: its site indices by kind (HQ, Factory, Airfield, Jewel).
    pub faction_sites: Vec<[u16; 4]>,
    pub bark_park: P,
}

pub fn kind_slot(k: SiteKind) -> usize {
    match k {
        SiteKind::HQ => 0,
        SiteKind::Factory => 1,
        SiteKind::Airfield => 2,
        SiteKind::Jewel => 3,
    }
}

impl World {
    pub fn new(content: Content) -> Result<World, String> {
        if content.factions.len() != 7 {
            return Err("factions.ron must list all seven factions".into());
        }
        for (i, f) in content.factions.iter().enumerate() {
            if f.id.index() != i {
                return Err(format!("factions.ron: {:?} is out of order", f.id));
            }
        }
        let mut faction_sites = vec![[u16::MAX; 4]; 7];
        for (i, s) in content.map.sites.iter().enumerate() {
            let slot = &mut faction_sites[s.faction.index()][kind_slot(s.kind)];
            if *slot != u16::MAX {
                return Err(format!(
                    "map.ron: {:?} has two {:?} sites",
                    s.faction, s.kind
                ));
            }
            *slot = i as u16;
        }
        for (f, slots) in faction_sites.iter().enumerate() {
            if slots.contains(&u16::MAX) {
                return Err(format!(
                    "map.ron: {:?} needs an HQ, Factory, Airfield and Jewel",
                    FactionId::ALL[f]
                ));
            }
        }
        let bark_park = content
            .map
            .landmarks
            .iter()
            .find(|l| l.id == "bark_park")
            .map(|l| l.pos)
            .ok_or("map.ron: missing the bark_park landmark")?;
        Ok(World {
            content,
            faction_sites,
            bark_park,
        })
    }

    pub fn from_files(files: &ContentFiles) -> Result<World, String> {
        World::new(Content::parse(files)?)
    }

    pub fn rules(&self) -> &RulesDef {
        &self.content.rules
    }

    pub fn site(&self, i: u16) -> &SiteDef {
        &self.content.map.sites[i as usize]
    }

    pub fn site_of(&self, f: FactionId, k: SiteKind) -> u16 {
        self.faction_sites[f.index()][kind_slot(k)]
    }

    pub fn faction(&self, f: FactionId) -> &FactionDef {
        &self.content.factions[f.index()]
    }

    /// The region a point lies in, if it's on the map's land.
    pub fn region_at(&self, p: P) -> Option<Region> {
        self.content
            .map
            .zones
            .iter()
            .find(|z| inside(p, &z.poly))
            .map(|z| z.region)
    }

    pub fn max_health(&self, k: SiteKind) -> i32 {
        let h = &self.rules().health;
        match k {
            SiteKind::HQ => h.hq,
            SiteKind::Factory => h.factory,
            SiteKind::Airfield => h.airfield,
            SiteKind::Jewel => h.jewel,
        }
    }
}
