//! Operations research for the player: the Weak Spot view (which nodes cut a
//! network's income the most) and target values for Best Targets and bots.

use flyover_core::content::{FactionId, NodeKind, World};
use flyover_core::economy;
use flyover_core::rules;
use flyover_core::state::GameState;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct WeakSpot {
    pub seat: u8,
    pub nodes: Vec<u16>,
    pub supply_loss: i32,
    pub fuel_loss: i32,
    /// Share of the main resource these nodes carry, in percent.
    pub pct: i32,
    pub resource: String,
}

/// Health as `viewer` believes it.
pub fn believed_health(state: &GameState, viewer: u8) -> Vec<u8> {
    state
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if n.owner == Some(viewer) {
                n.health
            } else {
                state
                    .intel
                    .get(viewer as usize)
                    .and_then(|k| k.known.get(i).copied())
                    .unwrap_or(n.health)
            }
        })
        .collect()
}

/// The one or two nodes whose loss cuts `target`'s income the most, as `viewer` sees it.
pub fn weak_spot(world: &World, state: &GameState, viewer: u8, target: u8) -> WeakSpot {
    let base = believed_health(state, viewer);
    let night = state.night;
    let full = economy::income_with(world, state, target, &base, night);
    let candidates: Vec<u16> = world
        .nodes
        .iter()
        .filter(|n| {
            state.nodes[n.idx as usize].owner == Some(target)
                && n.kind.on_network()
                && n.kind != NodeKind::Capital
                && base[n.idx as usize] > 0
        })
        .map(|n| n.idx)
        .collect();
    let loss = |nodes: &[u16]| -> (i32, i32) {
        let mut h = base.clone();
        for &n in nodes {
            h[n as usize] = 0;
        }
        let inc = economy::income_with(world, state, target, &h, night);
        (full.supply - inc.supply, full.fuel - inc.fuel)
    };
    let weight = |l: (i32, i32)| l.0 * 3 + l.1 * 2;
    let mut best: (i32, Vec<u16>, (i32, i32)) = (0, Vec::new(), (0, 0));
    let singles: Vec<(u16, (i32, i32))> = candidates.iter().map(|&n| (n, loss(&[n]))).collect();
    for &(n, l) in &singles {
        if weight(l) > best.0 {
            best = (weight(l), vec![n], l);
        }
    }
    for i in 0..candidates.len() {
        for j in i + 1..candidates.len() {
            let pair = [candidates[i], candidates[j]];
            let l = loss(&pair);
            // A pair must beat the best single by a margin to be worth naming.
            if weight(l) > best.0 + 2 {
                best = (weight(l), pair.to_vec(), l);
            }
        }
    }
    let (s_loss, f_loss) = best.2;
    let s_pct = if full.supply > 0 {
        s_loss * 100 / full.supply
    } else {
        0
    };
    let f_pct = if full.fuel > 0 {
        f_loss * 100 / full.fuel
    } else {
        0
    };
    let (pct, resource) = if f_pct > s_pct {
        (f_pct, "Fuel")
    } else {
        (s_pct, "Supply")
    };
    WeakSpot {
        seat: target,
        nodes: best.1,
        supply_loss: s_loss,
        fuel_loss: f_loss,
        pct,
        resource: resource.into(),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TargetInfo {
    pub node: u16,
    pub value: i32,
    pub why: String,
}

/// How much knocking `node` down a step is worth to `seat` tonight (by value,
/// never by who's winning).
pub fn node_value(
    world: &World,
    state: &GameState,
    seat: u8,
    node: u16,
    health: &[u8],
) -> (i32, String) {
    let n = &world.nodes[node as usize];
    let ns = &state.nodes[node as usize];
    let sc = &world.rules().scoring;
    let Some(victim) = ns.owner else {
        return (0, "Nobody holds it.".into());
    };
    if victim == seat || health[node as usize] == 0 {
        return (0, String::new());
    }
    let mut v;
    let mut why;
    match n.kind {
        NodeKind::CrownJewel => {
            v = if n.home == Some(FactionId::GrainBelt) {
                sc.twine
            } else {
                sc.jewel
            } * 10
                + 30;
            why = "Crown jewel: big Bragging Rights and a nasty side effect.".to_string();
        }
        NodeKind::City | NodeKind::Factory => {
            v = sc.city_step * 10;
            why = if n.kind == NodeKind::City {
                "A city: Supply and Bragging Rights.".into()
            } else {
                "A factory: slows their builds.".into()
            };
        }
        _ => {
            v = sc.other_step * 10;
            why = match n.kind {
                NodeKind::AirDefense => "Air defense: clears the way for later strikes.".into(),
                NodeKind::Radar => "Radar: blinds them.".into(),
                NodeKind::Airbase | NodeKind::Boneyard => "Airbase: catch squadrons parked.".into(),
                NodeKind::Fuel => "Fuel: grounds their sorties.".into(),
                NodeKind::Junction => "Junction: cuts their supply lines.".into(),
                _ => "A target.".into(),
            };
        }
    }
    // Economic value: income the victim loses if it drops a step.
    if n.kind.on_network() && n.kind != NodeKind::Capital {
        let full = economy::income_with(world, state, victim, health, state.night);
        let mut h = health.to_vec();
        h[node as usize] = h[node as usize].saturating_sub(1);
        let hit = economy::income_with(world, state, victim, &h, state.night);
        let econ = (full.supply - hit.supply) * 4 + (full.fuel - hit.fuel) * 3;
        if econ >= 8 {
            why = format!("{} Cuts their income hard.", why);
        }
        v += econ;
    }
    if matches!(n.kind, NodeKind::Airbase | NodeKind::Boneyard) {
        let parked = state
            .squadrons
            .iter()
            .filter(|s| s.base == node && s.active())
            .count() as i32;
        v += parked * 4;
    }
    if n.kind == NodeKind::AirDefense {
        v += 6;
    }
    if rules::stage(world, state.night) == 2 && state.night >= world.rules().nights {
        v *= 2;
    }
    (v, why)
}

pub fn best_targets(world: &World, state: &GameState, seat: u8, limit: usize) -> Vec<TargetInfo> {
    let health = believed_health(state, seat);
    let mut out: Vec<TargetInfo> = world
        .nodes
        .iter()
        .filter(|n| rules::attack_check(world, state, seat, n.idx).is_ok())
        .map(|n| {
            let (value, why) = node_value(world, state, seat, n.idx, &health);
            TargetInfo {
                node: n.idx,
                value,
                why,
            }
        })
        .filter(|t| t.value > 0)
        .collect();
    out.sort_by(|a, b| b.value.cmp(&a.value).then(a.node.cmp(&b.node)));
    out.truncate(limit);
    out
}
