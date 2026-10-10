//! Rule checks shared by the simulation, the advisor and the user interface:
//! escalation stages, legal targets, legal orders and base capacity.

use crate::content::{FactionId, NodeKind, OrderKind, UnitKind, World};
use crate::state::{EffectKind, GameState, Mission, Squadron};

/// 0 = Saber Rattling, 1 = Open Season, 2 = Gloves Off.
pub fn stage(world: &World, night: u8) -> u8 {
    let r = world.rules();
    if night >= r.gloves_off {
        2
    } else if night >= r.open_season {
        1
    } else {
        0
    }
}

pub fn seat_of(state: &GameState, f: FactionId) -> Option<u8> {
    state.seats.iter().position(|s| s.faction == f).map(|i| i as u8)
}

pub fn faction(state: &GameState, seat: u8) -> FactionId {
    state.seats[seat as usize].faction
}

pub fn is_seated(state: &GameState, f: FactionId) -> bool {
    seat_of(state, f).is_some()
}

pub fn effect_active(state: &GameState, seat: u8, kind: EffectKind, night: u8) -> bool {
    state
        .effects
        .iter()
        .any(|e| e.seat == seat && e.kind == kind && e.from <= night && night <= e.until)
}

/// Neutral ground: the Big Empty and fallen home regions.
pub fn capturable(world: &World, state: &GameState, node: u16) -> bool {
    let n = &world.nodes[node as usize];
    if matches!(n.kind, NodeKind::Lake | NodeKind::CrownJewel) {
        return false;
    }
    match n.home {
        None => true,
        Some(f) => !is_seated(state, f),
    }
}

/// Objective value (Bragging Rights per night held) of a node.
pub fn objective(world: &World, state: &GameState, node: u16) -> i32 {
    let n = &world.nodes[node as usize];
    if n.objective > 0 {
        return n.objective;
    }
    match (n.kind, n.home) {
        (NodeKind::Capital, Some(f)) if !is_seated(state, f) => 1,
        _ => 0,
    }
}

pub fn owner_of(state: &GameState, node: u16) -> Option<u8> {
    state.nodes[node as usize].owner
}

/// Can `seat` order an attack on `node` tonight?
pub fn attack_check(world: &World, state: &GameState, seat: u8, node: u16) -> Result<(), &'static str> {
    let n = &world.nodes[node as usize];
    let ns = &state.nodes[node as usize];
    let st = stage(world, state.night);
    match n.kind {
        NodeKind::Lake => return Err("You can't bomb a lake. The floatplanes know it."),
        NodeKind::Capital if ns.owner.is_some() && n.home.map(|f| is_seated(state, f)).unwrap_or(false) => {
            return Err("Capitals can't be destroyed.")
        }
        _ => {}
    }
    if ns.owner == Some(seat) {
        return Err("That's ours.");
    }
    if !ns.active {
        return Err("Just a landmark now. Nobody's home.");
    }
    if n.kind == NodeKind::MassDriver && st < 2 {
        return Err("The Mass Driver opens up at Gloves Off (Night 6).");
    }
    let home_seated = n.home.map(|f| is_seated(state, f)).unwrap_or(false);
    if home_seated {
        let ok = match n.kind {
            k if k.is_military() => true,
            NodeKind::CrownJewel => st >= 2,
            _ => st >= 1,
        };
        if !ok {
            return Err(if n.kind == NodeKind::CrownJewel {
                "Crown jewels open up at Gloves Off (Night 6)."
            } else {
                "Cities, factories, fuel and junctions open up at Open Season (Night 3)."
            });
        }
    }
    if faction(state, seat) == FactionId::Beehive && st < 2 {
        if let Some(o) = ns.owner {
            if !state.beehive_grudge.contains(&o) {
                return Err("Oh my heck: Utah never throws the first punch.");
            }
        }
    }
    Ok(())
}

/// Can `seat` guard `node`?
pub fn guard_check(world: &World, state: &GameState, seat: u8, node: u16) -> Result<(), &'static str> {
    let n = &world.nodes[node as usize];
    if n.kind == NodeKind::Lake {
        return Err("Guard a place on land.");
    }
    let ns = &state.nodes[node as usize];
    if ns.owner == Some(seat) || capturable(world, state, node) {
        return Ok(());
    }
    if faction(state, seat) == FactionId::Beehive {
        return Ok(()); // Service Project: Utah guards anyone's nodes.
    }
    Err("You can only guard your own or neutral ground.")
}

pub fn guard_zone_check(world: &World, state: &GameState, seat: u8, zone: u16) -> Result<(), &'static str> {
    let z = &world.zones[zone as usize];
    match z.region.faction() {
        None => Ok(()),
        Some(f) if f == faction(state, seat) || !is_seated(state, f) => Ok(()),
        _ if faction(state, seat) == FactionId::Beehive => Ok(()),
        _ => Err("Guard your own or neutral skies. Use Sweep to hunt elsewhere."),
    }
}

/// How many squadrons a base can hold (0 when knocked out).
pub fn base_capacity(world: &World, state: &GameState, node: u16, kind: UnitKind) -> i32 {
    let n = &world.nodes[node as usize];
    let ns = &state.nodes[node as usize];
    let e = &world.rules().economy;
    match n.kind {
        NodeKind::Lake => {
            if kind == UnitKind::Floatplane {
                e.airbase_capacity
            } else {
                0
            }
        }
        NodeKind::Capital => e.capital_capacity,
        NodeKind::Airbase | NodeKind::Boneyard => match ns.health {
            0 => 0,
            1 => e.airbase_capacity / 2,
            _ => e.airbase_capacity,
        },
        _ => 0,
    }
}

pub fn base_occupancy(state: &GameState, node: u16, except: Option<u32>) -> i32 {
    state
        .squadrons
        .iter()
        .filter(|s| s.base == node && s.down_until == 0 && Some(s.id) != except)
        .count() as i32
}

/// Can `seat` base `sq` at `node`?
pub fn rebase_check(world: &World, state: &GameState, sq: &Squadron, node: u16) -> Result<(), &'static str> {
    let n = &world.nodes[node as usize];
    let ns = &state.nodes[node as usize];
    if node == sq.base {
        return Err("Already based there.");
    }
    let lake_ok = n.kind == NodeKind::Lake && sq.kind == UnitKind::Floatplane;
    if !lake_ok {
        if !n.kind.is_base() {
            return Err("Squadrons can only base at airbases (floatplanes: lakes too).");
        }
        if ns.owner != Some(sq.seat) {
            return Err("Capture that airbase first.");
        }
    }
    if base_occupancy(state, node, Some(sq.id)) >= base_capacity(world, state, node, sq.kind) {
        return Err("No room at that base.");
    }
    Ok(())
}

pub fn can_order(world: &World, kind: UnitKind, ok: OrderKind) -> bool {
    world.unit(kind).orders.contains(&ok)
}

/// Full legality check for a squadron's mission tonight.
pub fn mission_check(world: &World, state: &GameState, sq: &Squadron, m: &Mission) -> Result<(), String> {
    if !sq.active() {
        return Err("That squadron is out of action.".into());
    }
    if !can_order(world, sq.kind, m.kind()) {
        return Err(format!("{} can't do that.", world.unit(sq.kind).name));
    }
    let nn = world.nodes.len() as u16;
    let nz = world.zones.len() as u16;
    let find = |id: u32| state.squadrons.iter().find(|s| s.id == id);
    match *m {
        Mission::Rest => Ok(()),
        Mission::Attack { node } => {
            if node >= nn {
                return Err("No such target.".into());
            }
            attack_check(world, state, sq.seat, node).map_err(String::from)
        }
        Mission::Guard { node } => {
            if node >= nn {
                return Err("No such place.".into());
            }
            guard_check(world, state, sq.seat, node).map_err(String::from)
        }
        Mission::GuardZone { zone } => {
            if zone >= nz {
                return Err("No such zone.".into());
            }
            guard_zone_check(world, state, sq.seat, zone).map_err(String::from)
        }
        Mission::Sweep { zone } | Mission::Scout { zone } | Mission::JamZone { zone } => {
            if zone >= nz {
                Err("No such zone.".into())
            } else {
                Ok(())
            }
        }
        Mission::Escort { sq: other } | Mission::JamEscort { sq: other } => match find(other) {
            Some(o) if o.seat == sq.seat && o.id != sq.id && o.active() => Ok(()),
            _ => Err("Pick one of your own squadrons to fly with.".into()),
        },
        Mission::Rebase { node } => {
            if node >= nn {
                return Err("No such base.".into());
            }
            rebase_check(world, state, sq, node).map_err(String::from)
        }
    }
}

/// Factory build capacity for a seat tonight.
pub fn build_capacity(world: &World, state: &GameState, seat: u8) -> i32 {
    let e = &world.rules().economy;
    let mut cap = 0;
    for n in &world.nodes {
        if n.kind == NodeKind::Factory && state.nodes[n.idx as usize].owner == Some(seat) {
            cap += match state.nodes[n.idx as usize].health {
                2 => e.factory_builds,
                1 => e.factory_builds / 2,
                _ => 0,
            };
        }
    }
    if effect_active(state, seat, EffectKind::FactoriesHalf, state.night) {
        cap /= 2;
    }
    cap
}

/// Can this seat build this unit at all?
pub fn buildable(world: &World, state: &GameState, seat: u8, kind: UnitKind) -> Result<(), &'static str> {
    let u = world.unit(kind);
    if !u.buildable {
        return Err("Can't be built; it comes back on its own.");
    }
    if let Some(f) = u.faction {
        if f != faction(state, seat) {
            return Err("Another faction's unique unit.");
        }
    }
    if u.max_owned > 0 {
        let owned = state.squadrons.iter().filter(|s| s.seat == seat && s.kind == kind).count() as i32
            + state.pending.iter().filter(|p| p.seat == seat && p.kind == kind).count() as i32;
        if owned >= u.max_owned {
            return Err("You already have as many of those as you can keep.");
        }
    }
    Ok(())
}

pub fn ace_name(world: &World, state: &GameState, seat: u8) -> String {
    world.faction(faction(state, seat)).ace.clone()
}
