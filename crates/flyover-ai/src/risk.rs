//! Risk colors: the advisor's estimate of the chance of losing a squadron on
//! a mission, with one plain sentence explaining it. Players see green, yellow
//! or red, never the formula.

use flyover_core::content::{dist, CombatClass, FactionId, World, P};
use flyover_core::route;
use flyover_core::rules;
use flyover_core::sim;
use flyover_core::state::{GameState, Mission, Orders, Squadron};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RiskColor {
    Green,
    Yellow,
    Red,
}

#[derive(Clone, Debug, Serialize)]
pub struct Preview {
    pub sq: u32,
    pub path: Vec<P>,
    pub fuel: i32,
    pub risk: i32,
    pub color: RiskColor,
    /// Lowercase fragment: "crosses 2 air defense sites with no jammer".
    pub reason: String,
    pub ad_sites: i32,
    pub escorted: bool,
    pub jammed: bool,
    pub in_static: bool,
}

pub fn color(risk: i32) -> RiskColor {
    if risk < 150 {
        RiskColor::Green
    } else if risk <= 400 {
        RiskColor::Yellow
    } else {
        RiskColor::Red
    }
}

/// Enemy fighter-capable squadrons this seat knows of within `range` of a point.
fn known_fighters_near(world: &World, state: &GameState, seat: u8, p: P, range: i32) -> i32 {
    let mut n = 0;
    let mut seen: Vec<u32> = Vec::new();
    for s in flyover_core::vision::visible_squadrons(world, state, seat) {
        if world.unit(s.kind).class == CombatClass::Fighter
            && dist(world.nodes[s.base as usize].pos, p) <= range
        {
            n += 1;
            seen.push(s.id);
        }
    }
    if let Some(intel) = state.intel.get(seat as usize) {
        for g in &intel.ghosts {
            if seen.contains(&g.sq) {
                continue;
            }
            if world.unit(g.kind).class == CombatClass::Fighter
                && dist(world.nodes[g.node as usize].pos, p) <= range
            {
                n += 1;
            }
        }
    }
    n
}

/// Preview one squadron's mission given the rest of tonight's draft orders.
pub fn preview(
    world: &World,
    state: &GameState,
    orders: &Orders,
    sq: &Squadron,
    m: &Mission,
    waypoint: Option<P>,
) -> Option<Preview> {
    let planner = route::Planner::for_seat(world, state, sq.seat);
    preview_with(world, state, orders, sq, m, waypoint, &planner)
}

pub fn preview_with(
    world: &World,
    state: &GameState,
    orders: &Orders,
    sq: &Squadron,
    m: &Mission,
    waypoint: Option<P>,
    planner: &route::Planner,
) -> Option<Preview> {
    let seat = sq.seat;
    let threats = &planner.threats;
    // Escorts fly their lead's route.
    let (lead, lead_m, lead_wp) = match *m {
        Mission::Escort { sq: t } | Mission::JamEscort { sq: t } => {
            let l = state.squadrons.iter().find(|s| s.id == t)?;
            let lm = orders
                .missions
                .iter()
                .find(|o| o.sq == t)
                .map(|o| (o.mission.clone(), o.waypoint))
                .unwrap_or((l.order.clone(), l.waypoint));
            (l, lm.0, lm.1)
        }
        _ => (sq, m.clone(), waypoint),
    };
    if matches!(
        lead_m,
        Mission::Rest | Mission::Escort { .. } | Mission::JamEscort { .. }
    ) {
        return None;
    }
    let to = sim::mission_point(world, state, lead, &lead_m)?;
    let from = world.nodes[lead.base as usize].pos;
    let path = planner.plan(from, to, lead_wp);
    let fuel = sim::fuel_cost(world, state, sq, m, &path);
    let in_static = route::path_in_static(world, &path);
    let u = world.unit(sq.kind);
    let class = u.class;
    let r = world.rules();
    let lead_id = lead.id;
    let escorted = orders
        .missions
        .iter()
        .any(|o| matches!(o.mission, Mission::Escort { sq: t } if t == lead_id))
        || matches!(m, Mission::Escort { .. });
    let jam_escort = orders
        .missions
        .iter()
        .any(|o| matches!(o.mission, Mission::JamEscort { sq: t } if t == lead_id))
        || class == CombatClass::Jammer;
    let crossed = route::threats_crossed(threats, &path);
    // A jammed zone covers the AD sites inside it.
    let jammed_zones: Vec<u16> = orders
        .missions
        .iter()
        .filter_map(|o| {
            if let Mission::JamZone { zone } = o.mission {
                Some(zone)
            } else {
                None
            }
        })
        .collect();
    let mut ad_live = 0;
    for &ti in &crossed {
        let t = &threats[ti];
        let z = world.zone_at(t.pos);
        if !jam_escort && !jammed_zones.contains(&z) {
            ad_live += 1;
        }
    }
    let stealthy = u.stealth || (sq.ace && rules::faction(state, seat) == FactionId::Cascadia);
    let passes = if u.one_way || matches!(lead_m, Mission::Rebase { .. }) {
        1
    } else {
        2
    };
    let ad_hit = r.air_defense.hit[class.index()] - u.ad_evasion;
    let expected_hits_x1000 = if stealthy {
        0
    } else {
        ad_live * passes * r.air_defense.shots_full * ad_hit
    };
    // Enemy fighters near where we're going.
    let target_zone = world.zone_at(to);
    let region = world.zones[target_zone as usize].region.faction();
    let enemy_land = match region {
        Some(f) => f != rules::faction(state, seat) && rules::is_seated(state, f),
        None => false,
    };
    let near = known_fighters_near(world, state, seat, to, 260);
    let mut p_guard = if enemy_land { 350 } else { 100 };
    if near > 0 {
        p_guard = (450 + 150 * near).min(900);
    }
    let own_land = region == Some(rules::faction(state, seat));
    if own_land
        && matches!(
            lead_m,
            Mission::Guard { .. } | Mission::GuardZone { .. } | Mission::Rebase { .. }
        )
    {
        p_guard = 60;
    }
    let mut loss_if_met = match class {
        CombatClass::Bomber | CombatClass::Drone | CombatClass::Jammer => {
            if escorted {
                380
            } else {
                800
            }
        }
        CombatClass::Fighter => 300,
        CombatClass::Scout => 120,
    };
    if stealthy {
        loss_if_met /= 4;
    }
    if sq.ace {
        loss_if_met = loss_if_met * 2 / 3;
    }
    let fighter_risk = p_guard * loss_if_met / 1000;
    // Expected AD hits on a four-plane squadron: one expected hit ~ 250 per mille.
    let ad_risk = (expected_hits_x1000.max(0) / 4).min(900);
    let mut risk = 1000 - (1000 - ad_risk) * (1000 - fighter_risk) / 1000;
    if sq.kind == flyover_core::UnitKind::Prototype {
        risk = 1000 - (1000 - risk) * (1000 - r.factions.prototype_vanish) / 1000;
    }
    if in_static {
        risk = 1000;
    }
    if matches!(m, Mission::Rest) {
        risk = 0;
    }
    let reason = if in_static {
        "flies into the Static east of the Mississippi".to_string()
    } else if ad_live > 0 && ad_risk >= fighter_risk {
        format!(
            "crosses {} air defense site{} with no jammer",
            ad_live,
            if ad_live == 1 { "" } else { "s" }
        )
    } else if fighter_risk >= 150
        && !escorted
        && matches!(
            class,
            CombatClass::Bomber | CombatClass::Drone | CombatClass::Jammer
        )
    {
        if near > 0 {
            format!(
                "runs into {} enemy fighter squadron{} we know of, with no escort",
                near,
                if near == 1 { "" } else { "s" }
            )
        } else {
            "goes deep into enemy skies with no escort".to_string()
        }
    } else if fighter_risk >= 150 {
        "will probably meet enemy fighters".to_string()
    } else if stealthy {
        "slips past radar".to_string()
    } else if !crossed.is_empty() {
        "has its air defense covered".to_string()
    } else {
        "looks clear, as far as we know".to_string()
    };
    Some(Preview {
        sq: sq.id,
        path,
        fuel,
        risk,
        color: color(risk),
        reason,
        ad_sites: ad_live,
        escorted,
        jammed: jam_escort,
        in_static,
    })
}
