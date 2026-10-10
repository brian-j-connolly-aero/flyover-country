//! What each seat can see: radar coverage, revealed zones, the always-visible
//! Grain Belt and Texas refineries, and the ground truth they unlock.

use crate::content::{dist2, FactionId, NodeKind, World, P};
use crate::rules;
use crate::state::{EffectKind, GameState, Squadron};

/// Static radar sources for a seat on a given night: (center, radius).
pub fn radar_sources(world: &World, state: &GameState, seat: u8, night: u8, blinded: &[bool]) -> Vec<(P, i32)> {
    let r = &world.rules().radar;
    let f = rules::faction(state, seat);
    let sealed = rules::effect_active(state, seat, EffectKind::RadarSealed, night);
    let mut out = Vec::new();
    for n in &world.nodes {
        let ns = &state.nodes[n.idx as usize];
        if ns.owner != Some(seat) {
            continue;
        }
        match n.kind {
            NodeKind::Radar if !sealed && ns.health > 0 && !blinded.get(n.idx as usize).copied().unwrap_or(false) => {
                let base = match f {
                    FactionId::FrontRange => r.frontrange_site,
                    FactionId::GrainBelt => r.grainbelt_site,
                    _ => r.site,
                };
                let radius = if ns.health == 2 { base } else { base * 2 / 3 };
                out.push((n.pos, radius));
            }
            NodeKind::Capital => out.push((n.pos, r.capital)),
            NodeKind::Airbase | NodeKind::Boneyard if ns.health > 0 => out.push((n.pos, r.airbase)),
            _ => {}
        }
    }
    out
}

pub fn covered(sources: &[(P, i32)], p: P) -> bool {
    sources.iter().any(|&(c, r)| dist2(c, p) <= (r as i64) * (r as i64))
}

/// Grain Belt territory is flat as a pancake: everyone sees everything there.
pub fn always_visible_zone(world: &World, state: &GameState, zone: u16) -> bool {
    matches!(world.zones[zone as usize].region.faction(), Some(FactionId::GrainBelt))
        && rules::is_seated(state, FactionId::GrainBelt)
}

/// Is this node's true health visible to `seat` right now?
pub fn node_visible(world: &World, state: &GameState, seat: u8, node: u16) -> bool {
    let n = &world.nodes[node as usize];
    let ns = &state.nodes[node as usize];
    if ns.owner == Some(seat) {
        return true;
    }
    if always_visible_zone(world, state, n.zone) {
        return true;
    }
    // Big Oil, Big Targets: Texan refineries are always visible.
    if n.kind == NodeKind::Fuel && n.home == Some(FactionId::Texas) {
        return true;
    }
    revealed(world, state, seat, n.pos, n.zone, node as u64)
}

fn revealed(world: &World, state: &GameState, seat: u8, pos: P, zone: u16, salt: u64) -> bool {
    let Some(intel) = state.intel.get(seat as usize) else { return false };
    for r in &intel.revealed {
        if r.zone != zone {
            continue;
        }
        if r.half && (crate::rng::mix(salt, state.night as u64) & 1) == 1 {
            continue;
        }
        if let Some(seller) = r.radar_of {
            let src = radar_sources(world, state, seller, state.night, &[]);
            if !covered(&src, pos) {
                continue;
            }
        }
        return true;
    }
    false
}

/// Enemy squadrons this seat can see at dawn.
pub fn visible_squadrons<'a>(world: &World, state: &'a GameState, seat: u8) -> Vec<&'a Squadron> {
    let src = radar_sources(world, state, seat, state.night, &[]);
    let mut out = Vec::new();
    for s in &state.squadrons {
        if s.seat == seat || !s.active() {
            continue;
        }
        let n = &world.nodes[s.base as usize];
        // Floatplanes on lakes are hidden from radar; SASQUATCH never shows.
        let radar_ok = n.kind != NodeKind::Lake && !(s.ace && rules::faction(state, s.seat) == FactionId::Cascadia);
        let seen = (radar_ok && covered(&src, n.pos))
            || always_visible_zone(world, state, n.zone)
            || revealed(world, state, seat, n.pos, n.zone, s.id as u64 + 7919);
        if seen {
            out.push(s);
        }
    }
    out
}
