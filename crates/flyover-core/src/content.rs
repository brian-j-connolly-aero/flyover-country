//! Data-driven content: the map, units, factions, rules and patch notes all
//! live in Rusty Object Notation (RON) files, parsed here and compiled into a
//! [`World`] with integer indices for the simulation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type P = (i32, i32);

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
pub enum NodeKind {
    Capital,
    City,
    Fuel,
    Factory,
    Airbase,
    Radar,
    AirDefense,
    Junction,
    CrownJewel,
    Lake,
    Boneyard,
    MassDriver,
    TruckStop,
}

impl NodeKind {
    /// Squadrons can be based here (lakes only for floatplanes).
    pub fn is_base(self) -> bool {
        matches!(
            self,
            NodeKind::Airbase | NodeKind::Boneyard | NodeKind::Capital
        )
    }
    /// Military targets are open from Night 1.
    pub fn is_military(self) -> bool {
        matches!(
            self,
            NodeKind::Airbase | NodeKind::Radar | NodeKind::AirDefense | NodeKind::Boneyard
        )
    }
    /// Part of the supply network (lakes, radar and air defense sites are not).
    pub fn on_network(self) -> bool {
        !matches!(
            self,
            NodeKind::Lake
                | NodeKind::Radar
                | NodeKind::AirDefense
                | NodeKind::TruckStop
                | NodeKind::MassDriver
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkKind {
    Highway,
    Rail,
    Pipeline,
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
pub struct NodeDef {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub owner: Option<FactionId>,
    pub zone: String,
    pub pos: P,
    #[serde(default)]
    pub objective: i32,
    #[serde(default)]
    pub tagline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkDef {
    pub a: String,
    pub b: String,
    pub kind: LinkKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapDef {
    pub width: i32,
    pub height: i32,
    pub zones: Vec<ZoneDef>,
    pub fringe: Vec<Vec<P>>,
    pub mississippi: Vec<P>,
    pub nodes: Vec<NodeDef>,
    pub links: Vec<LinkDef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    Fighter,
    Bomber,
    Scout,
    Jammer,
    Drone,
    Floatplane,
    Watchtower,
    Longhorn,
    Prototype,
    CropDuster,
    Gull,
}

impl UnitKind {
    pub const ALL: [UnitKind; 11] = [
        UnitKind::Fighter,
        UnitKind::Bomber,
        UnitKind::Scout,
        UnitKind::Jammer,
        UnitKind::Drone,
        UnitKind::Floatplane,
        UnitKind::Watchtower,
        UnitKind::Longhorn,
        UnitKind::Prototype,
        UnitKind::CropDuster,
        UnitKind::Gull,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Combat classes index the air-to-air and air defense tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatClass {
    Fighter,
    Bomber,
    Scout,
    Jammer,
    Drone,
}

impl CombatClass {
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderKind {
    Rest,
    Attack,
    Guard,
    Escort,
    Sweep,
    Scout,
    Jam,
    Rebase,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitDef {
    pub kind: UnitKind,
    pub name: String,
    pub faction: Option<FactionId>,
    pub class: CombatClass,
    pub cost: i32,
    /// Tenths of a map unit per tick (one tick is one game minute).
    pub speed: i32,
    /// Fuel per 1000 map units flown.
    pub fuel_rate: i32,
    /// Per-plane chance (per mille) to hit a ground target. 0 = cannot attack.
    pub strike: i32,
    /// Each successful strike roll counts this many hits.
    #[serde(default = "one")]
    pub strike_mult: i32,
    /// Per-plane, per-round hit chance (per mille) against each combat class.
    pub air: [i32; 5],
    /// Subtracted from enemy air defense hit chance.
    #[serde(default)]
    pub ad_evasion: i32,
    /// Chance (per mille) that an incoming hit is shrugged off.
    #[serde(default)]
    pub armor: i32,
    pub orders: Vec<OrderKind>,
    #[serde(default)]
    pub one_way: bool,
    #[serde(default)]
    pub stealth: bool,
    /// Airborne radar radius in map units (0 = none).
    #[serde(default)]
    pub radar: i32,
    #[serde(default = "yes")]
    pub buildable: bool,
    /// 0 = unlimited.
    #[serde(default)]
    pub max_owned: i32,
    pub desc: String,
}

fn one() -> i32 {
    1
}
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RulesCard {
    pub signature: (String, String),
    pub quirk: (String, String),
    pub unique: (String, String),
    pub jewel: (String, String),
    pub ace: (String, String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactionDef {
    pub id: FactionId,
    pub name: String,
    pub short: String,
    pub home: String,
    pub color: String,
    pub shape: String,
    pub emblem: String,
    pub leader: String,
    pub plays_like: String,
    pub ace: String,
    pub unique: UnitKind,
    pub jewel: String,
    pub card: RulesCard,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatRules {
    pub rounds: i32,
    pub vet_bonus: i32,
    pub ace_bonus: i32,
    pub shooter_cap: i32,
    pub scout_escape: i32,
    pub escort_screen: i32,
    pub escort_screen_max: i32,
    pub min_hit: i32,
    pub max_hit: i32,
    pub cooldown: i32,
    pub fights_per_star: i32,
    pub max_stars: i32,
    /// Target weight by combat class when picking whom to shoot.
    pub target_weight: [i32; 5],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AirDefenseRules {
    pub range: i32,
    pub shots_full: i32,
    pub shots_half: i32,
    pub hit: [i32; 5],
    pub grainbelt_range: i32,
    pub grainbelt_shots_bonus: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RadarRules {
    pub site: i32,
    pub frontrange_site: i32,
    pub grainbelt_site: i32,
    pub capital: i32,
    pub airbase: i32,
    pub visual: i32,
    pub stealth_visual: i32,
    pub guard_radar: i32,
    pub guard_visual: i32,
    pub sweep: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrikeRules {
    pub hits_per_step: i32,
    /// Crown jewels are hardened: they need more hits per step.
    pub jewel_hits_per_step: i32,
    pub overcast_penalty: i32,
    pub smoke_penalty: i32,
    pub swarm_bonus: i32,
    pub texas_refinery_mult: i32,
    pub pivot_strike: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomyRules {
    pub highway: i32,
    pub rail: i32,
    pub pipeline: i32,
    pub half_node_throughput: i32,
    pub capital_supply: i32,
    pub capital_fuel: i32,
    pub city_supply: i32,
    pub fuel_site: i32,
    pub brewery: i32,
    pub brewery_cap: i32,
    pub texas_fuel_mult: i32,
    pub strip_supply_bonus: i32,
    pub truck_stop_fuel: i32,
    pub factory_builds: i32,
    pub airbase_capacity: i32,
    pub capital_capacity: i32,
    pub stockpile_nights: i32,
    pub stockpile_floor: i32,
    pub start_supply: i32,
    pub start_fuel: i32,
    pub repair_cost: i32,
    pub free_guard_radius: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoringRules {
    pub jewel: i32,
    pub twine: i32,
    pub ace: i32,
    pub squadron: i32,
    pub city_step: i32,
    pub other_step: i32,
    pub night8_mult: i32,
    pub scrappy_pct: i32,
    pub casserole_br: i32,
    pub casserole_supply: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactionRules {
    pub bet_max: i32,
    pub bet_node_mult: i32,
    pub bet_faction_mult: i32,
    pub harvest_min: i32,
    pub harvest_max: i32,
    pub prototype_vanish: i32,
    pub longhorn_rebuild: i32,
    pub ace_return: i32,
    pub process_delay: i32,
    pub mass_driver_steps: i32,
    pub revenge_steps: i32,
    pub salvage_strength: i32,
    pub brisket_range: i32,
    pub brisket_bonus: i32,
    pub pivot_tick: i32,
    pub pivot_range: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RulesDef {
    pub nights: u8,
    pub open_season: u8,
    pub gloves_off: u8,
    pub stage_names: [String; 3],
    pub ticks: i32,
    pub economy: EconomyRules,
    pub combat: CombatRules,
    pub air_defense: AirDefenseRules,
    pub radar: RadarRules,
    pub strike: StrikeRules,
    pub scoring: ScoringRules,
    pub factions: FactionRules,
    pub start_forces: Vec<UnitKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchEffect {
    /// Strike accuracy, per mille.
    Accuracy(i32),
    /// Speed, tenths of a unit per tick.
    Speed(i32),
    /// Harder (positive) or easier (negative) to hit, per mille.
    Evasion(i32),
    /// Extra fuel per drone mission (negative = cheaper).
    Fuel(i32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchNote {
    pub version: String,
    pub text: String,
    pub effect: PatchEffect,
}

/// Raw content as parsed from the RON files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Content {
    pub map: MapDef,
    pub units: Vec<UnitDef>,
    pub factions: Vec<FactionDef>,
    pub rules: RulesDef,
    pub patch_notes: Vec<PatchNote>,
}

/// The RON source texts, keyed by file stem.
pub struct ContentFiles<'a> {
    pub map: &'a str,
    pub units: &'a str,
    pub factions: &'a str,
    pub rules: &'a str,
    pub patch_notes: &'a str,
}

fn parse<T: for<'de> Deserialize<'de>>(name: &str, src: &str) -> Result<T, String> {
    ron::de::from_str(src).map_err(|e| format!("{name}.ron: {e}"))
}

impl Content {
    pub fn parse(files: &ContentFiles) -> Result<Content, String> {
        Ok(Content {
            map: parse("map", files.map)?,
            units: parse("units", files.units)?,
            factions: parse("factions", files.factions)?,
            rules: parse("rules", files.rules)?,
            patch_notes: parse("patch_notes", files.patch_notes)?,
        })
    }
}

// --- Compiled world ----------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Node {
    pub idx: u16,
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub home: Option<FactionId>,
    pub zone: u16,
    pub pos: P,
    pub objective: i32,
    pub tagline: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Zone {
    pub idx: u16,
    pub id: String,
    pub name: String,
    pub region: Region,
    pub center: P,
    pub poly: Vec<P>,
    /// Rough radius used for patrol orbits.
    pub radius: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub idx: u16,
    pub a: u16,
    pub b: u16,
    pub kind: LinkKind,
}

/// Compiled, index-based content. Immutable during a game.
#[derive(Clone, Debug)]
pub struct World {
    pub content: Content,
    pub nodes: Vec<Node>,
    pub zones: Vec<Zone>,
    pub links: Vec<Link>,
    /// Per node: (neighbor, link index).
    pub adj: Vec<Vec<(u16, u16)>>,
    pub node_ids: BTreeMap<String, u16>,
    pub zone_ids: BTreeMap<String, u16>,
    pub jewels: [u16; 7],
    pub capitals: [u16; 7],
}

impl World {
    pub fn new(content: Content) -> Result<World, String> {
        let mut zone_ids = BTreeMap::new();
        let mut zones = Vec::new();
        for (i, z) in content.map.zones.iter().enumerate() {
            let mut r2 = 0i64;
            for p in &z.poly {
                let dx = (p.0 - z.center.0) as i64;
                let dy = (p.1 - z.center.1) as i64;
                r2 = r2.max(dx * dx + dy * dy);
            }
            let radius = (isqrt(r2) as i32 * 45 / 100).clamp(40, 110);
            zone_ids.insert(z.id.clone(), i as u16);
            zones.push(Zone {
                idx: i as u16,
                id: z.id.clone(),
                name: z.name.clone(),
                region: z.region,
                center: z.center,
                poly: z.poly.clone(),
                radius,
            });
        }
        let mut node_ids = BTreeMap::new();
        let mut nodes = Vec::new();
        for (i, n) in content.map.nodes.iter().enumerate() {
            let zone = *zone_ids
                .get(&n.zone)
                .ok_or_else(|| format!("node {} has unknown zone {}", n.id, n.zone))?;
            if node_ids.insert(n.id.clone(), i as u16).is_some() {
                return Err(format!("duplicate node id {}", n.id));
            }
            nodes.push(Node {
                idx: i as u16,
                id: n.id.clone(),
                name: n.name.clone(),
                kind: n.kind,
                home: n.owner,
                zone,
                pos: n.pos,
                objective: n.objective,
                tagline: n.tagline.clone(),
            });
        }
        let mut links = Vec::new();
        let mut adj = vec![Vec::new(); nodes.len()];
        for (i, l) in content.map.links.iter().enumerate() {
            let a = *node_ids
                .get(&l.a)
                .ok_or_else(|| format!("link to unknown node {}", l.a))?;
            let b = *node_ids
                .get(&l.b)
                .ok_or_else(|| format!("link to unknown node {}", l.b))?;
            links.push(Link {
                idx: i as u16,
                a,
                b,
                kind: l.kind,
            });
            adj[a as usize].push((b, i as u16));
            adj[b as usize].push((a, i as u16));
        }
        let mut jewels = [u16::MAX; 7];
        let mut capitals = [u16::MAX; 7];
        for n in &nodes {
            if let Some(f) = n.home {
                match n.kind {
                    NodeKind::CrownJewel => jewels[f.index()] = n.idx,
                    NodeKind::Capital => capitals[f.index()] = n.idx,
                    _ => {}
                }
            }
        }
        for f in FactionId::ALL {
            if jewels[f.index()] == u16::MAX || capitals[f.index()] == u16::MAX {
                return Err(format!("faction {f:?} needs a capital and a crown jewel"));
            }
        }
        // Units must be listed in UnitKind order so they can be indexed directly.
        if content.units.len() != UnitKind::ALL.len() {
            return Err("units.ron must define every unit kind".into());
        }
        for (i, u) in content.units.iter().enumerate() {
            if u.kind.index() != i {
                return Err(format!("units.ron: {:?} is out of order", u.kind));
            }
        }
        if content.factions.len() != 7 {
            return Err("factions.ron must define all seven factions".into());
        }
        for (i, f) in content.factions.iter().enumerate() {
            if f.id.index() != i {
                return Err(format!("factions.ron: {:?} is out of order", f.id));
            }
        }
        if content.patch_notes.is_empty() {
            return Err("patch_notes.ron is empty".into());
        }
        Ok(World {
            content,
            nodes,
            zones,
            links,
            adj,
            node_ids,
            zone_ids,
            jewels,
            capitals,
        })
    }

    pub fn unit(&self, k: UnitKind) -> &UnitDef {
        &self.content.units[k.index()]
    }
    pub fn faction(&self, f: FactionId) -> &FactionDef {
        &self.content.factions[f.index()]
    }
    pub fn rules(&self) -> &RulesDef {
        &self.content.rules
    }
    pub fn node(&self, id: &str) -> Option<u16> {
        self.node_ids.get(id).copied()
    }

    /// Which zone contains a point (falls back to the nearest center).
    pub fn zone_at(&self, p: P) -> u16 {
        for z in &self.zones {
            if point_in_poly(p, &z.poly) {
                return z.idx;
            }
        }
        let mut best = 0u16;
        let mut bd = i64::MAX;
        for z in &self.zones {
            let d = dist2(p, z.center);
            if d < bd {
                bd = d;
                best = z.idx;
            }
        }
        best
    }

    /// x coordinate of the Mississippi at height y; east of it is the Static.
    pub fn static_x(&self, y: i32) -> i32 {
        let m = &self.content.map.mississippi;
        for w in m.windows(2) {
            let (a, b) = (w[0], w[1]);
            if (a.1 <= y && y <= b.1) || (b.1 <= y && y <= a.1) {
                if a.1 == b.1 {
                    return a.0.max(b.0);
                }
                return a.0 + (b.0 - a.0) * (y - a.1) / (b.1 - a.1);
            }
        }
        m.last().map(|p| p.0).unwrap_or(i32::MAX)
    }

    pub fn in_static(&self, p: P) -> bool {
        p.0 > self.static_x(p.1)
    }
}

pub fn dist2(a: P, b: P) -> i64 {
    let dx = (a.0 - b.0) as i64;
    let dy = (a.1 - b.1) as i64;
    dx * dx + dy * dy
}

pub fn dist(a: P, b: P) -> i32 {
    isqrt(dist2(a, b)) as i32
}

pub fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        0
    } else {
        v.isqrt()
    }
}

pub fn point_in_poly(p: P, poly: &[P]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (poly[i].0 as i64, poly[i].1 as i64);
        let (xj, yj) = (poly[j].0 as i64, poly[j].1 as i64);
        let (px, py) = (p.0 as i64, p.1 as i64);
        if (yi > py) != (yj > py) {
            // px < (xj - xi) * (py - yi) / (yj - yi) + xi, without division
            let lhs = (px - xi) * (yj - yi);
            let rhs = (xj - xi) * (py - yi);
            if (yj - yi > 0 && lhs < rhs) || (yj - yi < 0 && lhs > rhs) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}
