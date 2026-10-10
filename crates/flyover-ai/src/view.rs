//! What each window may see. The planning window gets one seat's private view;
//! the Theater window only ever gets the public view, so it can never leak.

use crate::analysis;
use flyover_core::content::{FactionId, NodeKind, UnitKind, World};
use flyover_core::economy::{self, Income};
use flyover_core::rules;
use flyover_core::state::*;
use flyover_core::vision;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct PublicSeat {
    pub seat: u8,
    pub faction: FactionId,
    pub score: i32,
    pub pride_lost: i32,
    pub planes_lost: i32,
    pub kills: i32,
    pub steps_dealt: i32,
    pub steps_taken: i32,
    pub scrappy: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct NodeView {
    pub owner: Option<u8>,
    pub health: u8,
    /// True when this health is current; false when it's a memory.
    pub exact: bool,
    pub smoke: bool,
    pub capturable: bool,
    pub active: bool,
    /// None = can be attacked tonight; otherwise why not.
    pub attack: Option<String>,
    pub guard: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnemyView {
    pub sq: u32,
    pub seat: u8,
    pub kind: UnitKind,
    pub strength: u8,
    pub base: u16,
    pub ace: bool,
    /// Night last seen (== tonight when visible now).
    pub night: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct BuildOption {
    pub kind: UnitKind,
    pub cost: i32,
    pub ok: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeatView {
    pub seat: u8,
    pub faction: FactionId,
    pub night: u8,
    pub nights: u8,
    pub stage: u8,
    pub over: bool,
    pub supply: i32,
    pub fuel: i32,
    pub income_supply: i32,
    pub income_fuel: i32,
    pub cap_supply: i32,
    pub cap_fuel: i32,
    pub build_capacity: i32,
    pub seats: Vec<PublicSeat>,
    pub nodes: Vec<NodeView>,
    pub squadrons: Vec<Squadron>,
    pub enemies: Vec<EnemyView>,
    pub revealed: Vec<Reveal>,
    pub flows: Income,
    pub effects: Vec<Effect>,
    pub patch: Option<u16>,
    pub harvest: i32,
    pub texas_grudge: Vec<u8>,
    pub beehive_grudge: Vec<u8>,
    pub revenge_against: Option<u8>,
    pub mass_driver_holder: Option<u8>,
    pub mass_driver_fired: bool,
    pub pending: Vec<PendingBuild>,
    pub builds: Vec<BuildOption>,
    pub carry_over: Orders,
    pub history: Vec<NightRecord>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicNode {
    pub owner: Option<u8>,
    pub health: u8,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicView {
    pub night: u8,
    pub nights: u8,
    pub stage: u8,
    pub over: bool,
    pub seats: Vec<PublicSeat>,
    pub nodes: Vec<PublicNode>,
    pub effects: Vec<Effect>,
    pub patch: Option<u16>,
    pub harvest: i32,
    pub mass_driver_holder: Option<u8>,
    pub mass_driver_fired: bool,
    pub history: Vec<NightRecord>,
}

fn public_seats(st: &GameState) -> Vec<PublicSeat> {
    st.seats
        .iter()
        .enumerate()
        .map(|(i, s)| PublicSeat {
            seat: i as u8,
            faction: s.faction,
            score: s.score,
            pride_lost: s.pride_lost,
            planes_lost: s.planes_lost,
            kills: s.kills,
            steps_dealt: s.steps_dealt,
            steps_taken: s.steps_taken,
            scrappy: st.scrappy.contains(&(i as u8)),
        })
        .collect()
}

fn mass_driver_holder(w: &World, st: &GameState) -> Option<u8> {
    w.nodes
        .iter()
        .find(|n| n.kind == NodeKind::MassDriver)
        .and_then(|n| st.nodes[n.idx as usize].owner)
}

pub fn public_view(w: &World, st: &GameState) -> PublicView {
    PublicView {
        night: st.night,
        nights: w.rules().nights,
        stage: rules::stage(w, st.night.min(w.rules().nights)),
        over: st.night > w.rules().nights,
        seats: public_seats(st),
        nodes: st
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| PublicNode {
                owner: n.owner,
                health: st.public_health.get(i).copied().unwrap_or(2),
                active: n.active,
            })
            .collect(),
        effects: st.effects.clone(),
        patch: st.patch,
        harvest: st.harvest,
        mass_driver_holder: mass_driver_holder(w, st),
        mass_driver_fired: st.mass_driver_fired,
        history: st.history.clone(),
    }
}

pub fn seat_view(w: &World, st: &GameState, seat: u8) -> SeatView {
    let s = &st.seats[seat as usize];
    let believed = analysis::believed_health(st, seat);
    let nodes = w
        .nodes
        .iter()
        .map(|n| {
            let ns = &st.nodes[n.idx as usize];
            let exact = vision::node_visible(w, st, seat, n.idx);
            NodeView {
                owner: ns.owner,
                health: if exact {
                    ns.health
                } else {
                    believed[n.idx as usize]
                },
                exact,
                smoke: st.smoke.iter().any(|sm| {
                    sm.node == n.idx && (sm.seat == seat || exact) && sm.until >= st.night
                }),
                capturable: rules::capturable(w, st, n.idx),
                active: ns.active,
                attack: rules::attack_check(w, st, seat, n.idx)
                    .err()
                    .map(String::from),
                guard: rules::guard_check(w, st, seat, n.idx)
                    .err()
                    .map(String::from),
            }
        })
        .collect();
    let mut enemies: Vec<EnemyView> = vision::visible_squadrons(w, st, seat)
        .into_iter()
        .map(|q| EnemyView {
            sq: q.id,
            seat: q.seat,
            kind: q.kind,
            strength: q.strength,
            base: q.base,
            ace: q.ace,
            night: st.night,
        })
        .collect();
    for g in &st.intel[seat as usize].ghosts {
        if !enemies.iter().any(|e| e.sq == g.sq) {
            enemies.push(EnemyView {
                sq: g.sq,
                seat: g.seat,
                kind: g.kind,
                strength: g.strength,
                base: g.node,
                ace: g.ace,
                night: g.night,
            });
        }
    }
    enemies.sort_by_key(|e| e.sq);
    let builds = w
        .content
        .units
        .iter()
        .filter(|u| u.faction.is_none() || u.faction == Some(s.faction))
        .map(|u| {
            let r = rules::buildable(w, st, seat, u.kind).err();
            BuildOption {
                kind: u.kind,
                cost: u.cost,
                ok: r.is_none(),
                reason: r.map(String::from),
            }
        })
        .collect();
    let night = st.night.min(w.rules().nights);
    SeatView {
        seat,
        faction: s.faction,
        night: st.night,
        nights: w.rules().nights,
        stage: rules::stage(w, night),
        over: st.night > w.rules().nights,
        supply: s.supply,
        fuel: s.fuel,
        income_supply: s.income_supply,
        income_fuel: s.income_fuel,
        cap_supply: economy::cap(w, s.income_supply),
        cap_fuel: economy::cap(w, s.income_fuel),
        build_capacity: rules::build_capacity(w, st, seat),
        seats: public_seats(st),
        nodes,
        squadrons: st
            .squadrons
            .iter()
            .filter(|q| q.seat == seat)
            .cloned()
            .collect(),
        enemies,
        revealed: st.intel[seat as usize].revealed.clone(),
        flows: economy::income(w, st, seat, night),
        effects: st.effects.clone(),
        patch: st.patch,
        harvest: st.harvest,
        texas_grudge: st.texas_grudge.clone(),
        beehive_grudge: st.beehive_grudge.clone(),
        revenge_against: st.revenge_against,
        mass_driver_holder: mass_driver_holder(w, st),
        mass_driver_fired: st.mass_driver_fired,
        pending: st
            .pending
            .iter()
            .filter(|p| p.seat == seat)
            .cloned()
            .collect(),
        builds,
        carry_over: crate::plan::carry_over(w, st, seat),
        history: st.history.clone(),
    }
}
