//! The Advisor: each faction's dog leader flags problems while you plan. At
//! most three warnings show at once, worst first. Text lives in flavor.ron;
//! this module only decides what to say and fills in the blanks.

use crate::analysis;
use crate::risk::{self, RiskColor};
use flyover_core::content::{CombatClass, FactionId, NodeKind, World};
use flyover_core::economy;
use flyover_core::route::Planner;
use flyover_core::rules;
use flyover_core::state::*;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Warning {
    pub key: String,
    /// 3 = will go wrong, 2 = risky, 1 = worth a look, 0 = informational.
    pub severity: u8,
    pub params: BTreeMap<String, String>,
    pub sq: Option<u32>,
    pub node: Option<u16>,
}

fn warn(key: &str, severity: u8) -> Warning {
    Warning {
        key: key.into(),
        severity,
        params: BTreeMap::new(),
        sq: None,
        node: None,
    }
}

impl Warning {
    fn p(mut self, k: &str, v: impl ToString) -> Warning {
        self.params.insert(k.into(), v.to_string());
        self
    }
}

pub fn advise(world: &World, state: &GameState, seat: u8, orders: &Orders) -> Vec<Warning> {
    let w = world;
    let st = state;
    let f = rules::faction(st, seat);
    let stage = rules::stage(w, st.night);
    let planner = Planner::for_seat(w, st, seat);
    let mine: Vec<&Squadron> = st
        .squadrons
        .iter()
        .filter(|s| s.seat == seat && s.active())
        .collect();
    let mission_of = |id: u32| -> (Mission, Option<flyover_core::content::P>) {
        orders
            .missions
            .iter()
            .find(|o| o.sq == id)
            .map(|o| (o.mission.clone(), o.waypoint))
            .unwrap_or_else(|| {
                mine.iter()
                    .find(|s| s.id == id)
                    .map(|s| (s.order.clone(), s.waypoint))
                    .unwrap_or((Mission::Rest, None))
            })
    };
    // Project Bark Park: tonight is a night off, so there's nothing else to flag.
    if orders.retreat && rules::retreat_check(w, st, seat).is_ok() {
        return vec![warn("retreating", 0)];
    }
    let mut out: Vec<Warning> = Vec::new();
    if rules::retreat_check(w, st, seat).is_ok() {
        out.push(warn("doghouse", 2));
    }
    if st.seats[seat as usize].fetch && orders.fetch.is_none() {
        out.push(warn("fetch_ready", 1));
    }
    let mut fuel_needed = 0;
    let mut flying = 0;
    let mut grounded_by_fuel = 0;
    let fuel = st.seats[seat as usize].fuel;
    let mut idle = 0;
    let mut scouts_out = false;
    let mut guards_home = false;
    let mut jewel_covered = false;
    let jewel = w.jewels[f.index()];
    let mut reds = 0;
    for s in &mine {
        let (m, wp) = mission_of(s.id);
        if m == Mission::Rest {
            if s.strength >= MAX_STRENGTH && s.kind != flyover_core::UnitKind::Longhorn {
                idle += 1;
            }
            continue;
        }
        if let Err(r) = rules::mission_check(w, st, s, &m) {
            out.push(
                warn("invalid", 3)
                    .p("unit", &w.unit(s.kind).name)
                    .p("reason", r),
            );
            continue;
        }
        match m {
            Mission::Scout { .. } => scouts_out = true,
            Mission::Guard { node } => {
                let n = &w.nodes[node as usize];
                if st.nodes[node as usize].owner == Some(seat) && n.home == Some(f) {
                    guards_home = true;
                }
                if node == jewel {
                    jewel_covered = true;
                }
            }
            Mission::GuardZone { zone } if w.zones[zone as usize].region.faction() == Some(f) => {
                guards_home = true;
            }
            _ => {}
        }
        let Some(p) = risk::preview_with(w, st, orders, s, &m, wp, &planner) else {
            continue;
        };
        flying += 1;
        fuel_needed += p.fuel;
        if fuel_needed > fuel {
            grounded_by_fuel += 1;
        }
        let unit = unit_label(w, st, s);
        if p.color == RiskColor::Red && reds < 2 {
            reds += 1;
            let sev = if matches!(
                w.unit(s.kind).class,
                CombatClass::Bomber | CombatClass::Drone
            ) {
                3
            } else {
                2
            };
            let mut wn = warn("red_route", sev)
                .p("unit", &unit)
                .p("reason", &p.reason);
            wn.sq = Some(s.id);
            out.push(wn);
        } else if matches!(m, Mission::Attack { .. })
            && !p.escorted
            && p.risk >= 150
            && p.reason.contains("escort")
            && matches!(
                w.unit(s.kind).class,
                CombatClass::Bomber | CombatClass::Drone
            )
        {
            let mut wn = warn("no_escort", 2).p("unit", &unit);
            wn.sq = Some(s.id);
            out.push(wn);
        }
    }
    let _ = flying;
    if grounded_by_fuel > 0 {
        out.push(warn("fuel_short", 3).p("count", grounded_by_fuel));
    }
    if !guards_home && st.night >= 2 {
        out.push(warn("empty_sky", 2));
    }
    if stage == 2 && !jewel_covered && st.nodes[jewel as usize].active {
        let smoked = st
            .smoke
            .iter()
            .any(|s| s.node == jewel && s.seat == seat && s.until >= st.night);
        if !smoked {
            let mut wn = warn("jewel_open", 2).p("jewel", &w.nodes[jewel as usize].name);
            wn.node = Some(jewel);
            out.push(wn);
        }
    }
    if idle > 0 {
        out.push(warn("idle", 1).p("count", idle));
    }
    if !scouts_out
        && st.night >= 2
        && mine
            .iter()
            .any(|s| rules::can_order(w, s.kind, flyover_core::content::OrderKind::Scout))
    {
        out.push(warn("no_scout", 1));
    }
    // Spending check.
    let mut supply = st.seats[seat as usize].supply;
    supply -= orders.builds.iter().map(|&k| w.unit(k).cost).sum::<i32>();
    supply -= orders.repairs.len() as i32 * w.rules().economy.repair_cost;
    supply -= orders.harvest_bid.max(0);
    supply -= orders.bet.as_ref().map(|b| b.stake).unwrap_or(0);
    let cap = economy::cap(w, st.seats[seat as usize].income_supply);
    if supply + st.seats[seat as usize].income_supply > cap {
        out.push(warn("supply_cap", 1));
    }
    for n in &w.nodes {
        let ns = &st.nodes[n.idx as usize];
        if ns.owner == Some(seat)
            && ns.health == 0
            && n.kind != NodeKind::Lake
            && !orders.repairs.contains(&n.idx)
        {
            let mut wn = warn("repair", 1).p("node", &n.name);
            wn.node = Some(n.idx);
            out.push(wn);
            break;
        }
    }
    // Faction reminders.
    let special = match f {
        FactionId::Pacific => st.patch.is_some(),
        FactionId::Cascadia => !orders.builds.is_empty(),
        FactionId::FrontRange => orders.intel_sale.is_none(),
        FactionId::Texas => !st.texas_grudge.is_empty(),
        FactionId::Dreamland => orders.bet.is_none(),
        FactionId::GrainBelt => st.harvest > 0,
        FactionId::Beehive => stage < 2,
    };
    if special {
        out.push(warn("special", 1));
    }
    let ws = analysis::weak_spot(w, st, seat, seat);
    if ws.pct >= 30 && !ws.nodes.is_empty() {
        let names: Vec<String> = ws
            .nodes
            .iter()
            .map(|&n| w.nodes[n as usize].name.clone())
            .collect();
        out.push(
            warn("weak_spot", 0)
                .p("nodes", names.join(" and "))
                .p("pct", ws.pct)
                .p("resource", &ws.resource),
        );
    }
    out.sort_by_key(|w| std::cmp::Reverse(w.severity));
    if out.iter().all(|w| w.severity == 0) {
        out.insert(0, warn("all_good", 0));
    }
    out
}

pub fn unit_label(w: &World, st: &GameState, s: &Squadron) -> String {
    if s.ace {
        rules::ace_name(w, st, s.seat)
    } else {
        format!("{} #{}", w.unit(s.kind).name, s.id)
    }
}
