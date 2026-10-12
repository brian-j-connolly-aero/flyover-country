//! What the planning screen draws: each raid's straight path, the patrols it
//! flew through last night, and which of your sites tonight's patrols cover.

use flyover_core::geom::{circle_entry, dist, P};
use flyover_core::rules::{normalize, patrol_radius, spend};
use flyover_core::state::{GameState, Mission, Orders, PatrolMark, Target};
use flyover_core::{SiteKind, UnitKind, World};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct RaidPreview {
    pub target: Target,
    pub from: P,
    pub to: P,
    pub sqs: Vec<u32>,
    pub fighters: i32,
    pub bombers: i32,
    /// Rival patrols from last night that this path flies through.
    pub crosses: Vec<PatrolMark>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Cover {
    pub site: u16,
    /// Tonight's patrols (squadron ids) whose circle contains the site.
    pub patrols: Vec<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Preview {
    pub orders: Orders,
    pub raids: Vec<RaidPreview>,
    pub cover: Vec<Cover>,
    pub spend: i32,
    pub supply_left: i32,
}

pub fn target_pos(world: &World, t: Target) -> P {
    match t {
        Target::Site { site } => world.site(site).pos,
        Target::Prize { prize } => world.content.map.prizes[prize as usize].pos,
    }
}

pub fn airfield(world: &World, state: &GameState, seat: u8) -> P {
    world
        .site(world.site_of(state.faction(seat), SiteKind::Airfield))
        .pos
}

/// Rival patrols from last night that a path from `from` to `to` crosses.
pub fn crossings(state: &GameState, seat: u8, from: P, to: P) -> Vec<PatrolMark> {
    let len = dist(from, to);
    state
        .last
        .patrols
        .iter()
        .filter(|p| p.seat != seat)
        .filter(|p| matches!(circle_entry(from, to, p.at, p.r), Some(s) if s < len))
        .cloned()
        .collect()
}

pub fn preview(world: &World, state: &GameState, orders: &Orders) -> Preview {
    let o = normalize(world, state, orders);
    let seat = o.seat;
    let from = airfield(world, state, seat);
    let mut raids: Vec<RaidPreview> = Vec::new();
    for m in &o.missions {
        let Mission::Fly { target } = m.mission else {
            continue;
        };
        let Some(sq) = state.squadron(m.sq) else {
            continue;
        };
        let r = match raids.iter_mut().position(|r| r.target == target) {
            Some(i) => &mut raids[i],
            None => {
                let to = target_pos(world, target);
                raids.push(RaidPreview {
                    target,
                    from,
                    to,
                    sqs: Vec::new(),
                    fighters: 0,
                    bombers: 0,
                    crosses: crossings(state, seat, from, to),
                });
                raids.last_mut().unwrap()
            }
        };
        r.sqs.push(sq.id);
        match sq.kind {
            UnitKind::Fighter => r.fighters += sq.planes,
            UnitKind::Bomber => r.bombers += sq.planes,
        }
    }
    let radius = patrol_radius(world, state, seat);
    let patrols: Vec<(u32, P)> = o
        .missions
        .iter()
        .filter_map(|m| match m.mission {
            Mission::Patrol { at } => Some((m.sq, at)),
            _ => None,
        })
        .collect();
    let f = state.faction(seat);
    let cover = [
        SiteKind::HQ,
        SiteKind::Factory,
        SiteKind::Airfield,
        SiteKind::Jewel,
    ]
    .iter()
    .map(|&k| {
        let site = world.site_of(f, k);
        let pos = world.site(site).pos;
        let patrols = patrols
            .iter()
            .filter(|(_, at)| dist(*at, pos) <= radius)
            .map(|(id, _)| *id)
            .collect();
        Cover { site, patrols }
    })
    .collect();
    let spend = spend(world, state, &o);
    let supply_left = state.seats[seat as usize].supply - spend;
    Preview {
        orders: o,
        raids,
        cover,
        spend,
        supply_left,
    }
}
