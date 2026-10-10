//! Starting a war: seat the factions, turn unpicked home regions into Fallen
//! zones and hand out the starting forces.

use crate::content::{FactionId, NodeKind, UnitKind, World};
use crate::economy;
use crate::rng::Rng;
use crate::rules;
use crate::state::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameConfig {
    pub seed: u64,
    /// Four distinct factions in seat order.
    pub factions: Vec<FactionId>,
}

pub fn new_game(world: &World, cfg: &GameConfig) -> Result<GameState, String> {
    if cfg.factions.len() < 2 || cfg.factions.len() > 4 {
        return Err("Pick two to four factions.".into());
    }
    for (i, f) in cfg.factions.iter().enumerate() {
        if cfg.factions[..i].contains(f) {
            return Err(format!("{f:?} picked twice."));
        }
    }
    let e = &world.rules().economy;
    let seats: Vec<SeatState> = cfg
        .factions
        .iter()
        .map(|&f| SeatState {
            faction: f,
            supply: e.start_supply,
            fuel: e.start_fuel,
            score: 0,
            pride_lost: 0,
            income_supply: 0,
            income_fuel: 0,
            planes_lost: 0,
            kills: 0,
            steps_dealt: 0,
            steps_taken: 0,
        })
        .collect();
    let seat_of = |f: FactionId| cfg.factions.iter().position(|&x| x == f).map(|i| i as u8);
    let nodes: Vec<NodeState> = world
        .nodes
        .iter()
        .map(|n| {
            let owner = n.home.and_then(seat_of);
            let fallen = n.home.is_some() && owner.is_none();
            NodeState {
                owner: if n.kind == NodeKind::Lake { None } else { owner },
                health: 2,
                hit_night: 0,
                active: !(fallen && n.kind == NodeKind::CrownJewel),
            }
        })
        .collect();
    let mut state = GameState {
        version: STATE_VERSION,
        seed: cfg.seed,
        night: 1,
        seats,
        nodes,
        squadrons: Vec::new(),
        next_sq: 1,
        effects: Vec::new(),
        intel: Vec::new(),
        patch: None,
        harvest: 0,
        pending: Vec::new(),
        smoke: Vec::new(),
        scrappy: Vec::new(),
        texas_grudge: Vec::new(),
        beehive_grudge: Vec::new(),
        revenge_against: None,
        history: vec![NightRecord { night: 0, scores: vec![0; cfg.factions.len()] }],
    };
    for seat in 0..state.seats.len() as u8 {
        let f = state.seats[seat as usize].faction;
        let fd = world.faction(f);
        let mut kinds: Vec<(UnitKind, bool)> = world.rules().start_forces.iter().map(|&k| (k, false)).collect();
        kinds.push((fd.unique, false));
        kinds.push((UnitKind::Fighter, true));
        for (kind, ace) in kinds {
            let base = place(world, &state, seat, kind).ok_or_else(|| format!("{f:?} has no room for its {kind:?}"))?;
            let id = state.next_sq;
            state.next_sq += 1;
            state.squadrons.push(Squadron {
                id,
                seat,
                kind,
                strength: MAX_STRENGTH,
                base,
                stars: 0,
                fights: 0,
                ace,
                order: Mission::Rest,
                waypoint: None,
                down_until: 0,
            });
        }
    }
    state.intel = (0..state.seats.len()).map(|_| Intel { known: vec![2; world.nodes.len()], ..Default::default() }).collect();
    let mut rng = Rng::for_night(cfg.seed, 0, 1);
    roll_dawn_offers(world, &mut state, &mut rng);
    for seat in 0..state.seats.len() as u8 {
        let inc = economy::income(world, &state, seat, 1);
        state.seats[seat as usize].income_supply = inc.supply;
        state.seats[seat as usize].income_fuel = inc.fuel;
    }
    Ok(state)
}

/// Pick a base for a new squadron: the faction's own bases with the most room
/// (floatplanes prefer lakes). Returns None when everything is full.
pub fn place(world: &World, state: &GameState, seat: u8, kind: UnitKind) -> Option<u16> {
    let f = rules::faction(state, seat);
    let mut best: Option<(i32, u16)> = None;
    for n in &world.nodes {
        let ns = &state.nodes[n.idx as usize];
        let ours = match n.kind {
            NodeKind::Lake => kind == UnitKind::Floatplane && n.home == Some(f),
            _ => ns.owner == Some(seat) && n.kind.is_base(),
        };
        if !ours {
            continue;
        }
        let room = rules::base_capacity(world, state, n.idx, kind) - rules::base_occupancy(state, n.idx, None);
        if room <= 0 {
            continue;
        }
        // Prefer lakes for floatplanes, airbases over the capital, then room.
        let pref = match n.kind {
            NodeKind::Lake => 100,
            NodeKind::Capital => 0,
            _ => 50,
        } + room;
        if best.map(|(b, _)| pref > b).unwrap_or(true) {
            best = Some((pref, n.idx));
        }
    }
    best.map(|(_, i)| i)
}

/// Patch Notes and the Harvest Shipment for the coming night.
pub fn roll_dawn_offers(world: &World, state: &mut GameState, rng: &mut Rng) {
    state.patch = if rules::is_seated(state, FactionId::Pacific) {
        Some(rng.below(world.content.patch_notes.len() as u32) as u16)
    } else {
        None
    };
    let fr = &world.rules().factions;
    state.harvest = if rules::is_seated(state, FactionId::GrainBelt) {
        fr.harvest_min + rng.below((fr.harvest_max - fr.harvest_min + 1) as u32) as i32
    } else {
        0
    };
}
