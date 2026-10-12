//! The planner behind bots and Suggest Plan. It reads only what a player can
//! see: the map, everyone's squadrons and last night's patrols and raids.

use crate::preview::{airfield, crossings, target_pos};
use flyover_core::geom::{circle_entry, dist, P};
use flyover_core::rng::{mix, Rng};
use flyover_core::rules::{
    build_cost, mission_check, patrol_ok, patrol_radius, site_open, site_up, stage,
};
use flyover_core::state::{GameState, Mission, MissionOrder, Orders, Target};
use flyover_core::{SiteKind, UnitKind, World};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    Easy,
    Normal,
}

struct Ctx<'a> {
    w: &'a World,
    st: &'a GameState,
    seat: u8,
    rng: Rng,
}

pub fn suggest(world: &World, state: &GameState, seat: u8, level: Level) -> Orders {
    let salt = mix(
        0xB075,
        seat as u64 + if level == Level::Easy { 100 } else { 0 },
    );
    let mut c = Ctx {
        w: world,
        st: state,
        seat,
        rng: Rng::for_night(state.seed, state.night, salt),
    };
    let me = &state.seats[seat as usize];
    let mut o = Orders::empty(seat);
    if me.doghouse {
        o.retreat = true;
        economy(&mut c, &mut o);
        return o;
    }
    if me.fetch {
        o.fetch = fetch_target(&c);
    }
    match level {
        Level::Normal => normal(&mut c, &mut o),
        Level::Easy => easy(&mut c, &mut o),
    }
    economy(&mut c, &mut o);
    o
}

/// Points for knocking a pip off this site.
fn pip_value(w: &World, site: u16) -> f64 {
    let s = &w.rules().score;
    match w.site(site).kind {
        SiteKind::HQ | SiteKind::Jewel => s.big_pip as f64,
        _ => s.pip as f64,
    }
}

/// Sites this seat can hit tonight that still have pips.
fn targets(c: &Ctx) -> Vec<u16> {
    (0..c.st.sites.len() as u16)
        .filter(|&s| site_open(c.w, c.st, c.seat, s).is_ok() && c.st.sites[s as usize].health > 0)
        .collect()
}

/// Rivals in the lead are better targets.
fn leader_bonus(c: &Ctx, victim: u8) -> f64 {
    let best = c.st.seats.iter().map(|s| s.score).max().unwrap_or(0);
    let v = c.st.seats[victim as usize].score;
    if v == best && best > c.st.seats[c.seat as usize].score {
        1.25
    } else {
        1.0
    }
}

fn bomb_hit(c: &Ctx, _victim: u8) -> f64 {
    c.w.rules().combat.bomb_hit as f64 / 1000.0
}

/// The site of each rival a Big Fetch should land on: an HQ or Crown Jewel of
/// whoever leads, once those are open; otherwise wait.
fn fetch_target(c: &Ctx) -> Option<u16> {
    let last = c.st.night >= c.w.rules().nights;
    if stage(c.w, c.st.night) < 2 && !last {
        return None;
    }
    targets(c).into_iter().max_by(|&a, &b| {
        let score = |s: u16| {
            let victim = c.st.site_owner(c.w, s).unwrap();
            pip_value(c.w, s)
                * leader_bonus(c, victim)
                * (c.st.sites[s as usize].health.min(2) as f64)
        };
        score(a).total_cmp(&score(b))
    })
}

fn normal(c: &mut Ctx, o: &mut Orders) {
    let mut fighters: Vec<(u32, bool, i32)> = Vec::new();
    let mut bombers: Vec<(u32, i32)> = Vec::new();
    for s in c.st.squadrons_of(c.seat) {
        match s.kind {
            UnitKind::Fighter => fighters.push((s.id, s.ace, s.planes)),
            UnitKind::Bomber => bombers.push((s.id, s.planes)),
        }
    }
    // Battered squadrons rest and come back stronger.
    let tired = |planes: i32| planes <= 1;
    bombers.sort_by_key(|&(id, p)| (-p, id));
    fighters.sort_by_key(|&(id, ace, p)| (ace, -p, id));

    // Bombers: pile onto the best targets until each is likely flattened.
    let from = airfield(c.w, c.st, c.seat);
    let mut planned: Vec<(u16, f64)> = Vec::new(); // site, expected hits so far
    let mut raids: Vec<(Target, usize)> = Vec::new(); // target, crossings
    for &(id, planes) in &bombers {
        if tired(planes) {
            continue;
        }
        let mut best: Option<(f64, u16)> = None;
        for site in targets(c) {
            let victim = c.st.site_owner(c.w, site).unwrap();
            let to = c.w.site(site).pos;
            let cross = crossings(c.st, c.seat, from, to).len() as f64;
            let survive = (planes as f64 - 1.1 * cross - 0.4).max(0.3);
            let hits = survive * bomb_hit(c, victim);
            let health = c.st.sites[site as usize].health as f64;
            let already = planned
                .iter()
                .find(|p| p.0 == site)
                .map(|p| p.1)
                .unwrap_or(0.0);
            let useful = (health - already).max(0.0).min(hits);
            let kind = match c.w.site(site).kind {
                SiteKind::Airfield => 1.1,
                SiteKind::Factory => 1.15,
                SiteKind::Jewel => 1.1,
                SiteKind::HQ => 1.0,
            };
            let jitter = 0.85 + c.rng.below(300) as f64 / 1000.0;
            let score = useful * pip_value(c.w, site) * kind * leader_bonus(c, victim) * jitter
                - 0.6 * cross;
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, site));
            }
        }
        if let Some((score, site)) = best {
            if score > 0.2 {
                let target = Target::Site { site };
                o.missions.push(MissionOrder {
                    sq: id,
                    mission: Mission::Fly { target },
                });
                let to = c.w.site(site).pos;
                let cross = crossings(c.st, c.seat, from, to).len();
                match planned.iter_mut().find(|p| p.0 == site) {
                    Some(p) => p.1 += planes as f64 * 0.5,
                    None => planned.push((site, planes as f64 * 0.5)),
                }
                if !raids.iter().any(|r| r.0 == target) {
                    raids.push((target, cross));
                }
            }
        }
    }

    // Fighters: one patrol at home first, then escort a risky raid, then
    // chase prizes, then more patrols.
    let mut covered: Vec<P> = Vec::new();
    let mut escorted = false;
    let mut claimed: Vec<u16> = Vec::new();
    for (i, &(id, ace, planes)) in fighters.iter().enumerate() {
        if tired(planes) && !ace {
            continue;
        }
        let first_patrol = covered.is_empty() && !ace;
        if !first_patrol && !escorted {
            if let Some(&(target, cross)) = raids.iter().max_by_key(|r| r.1) {
                if cross > 0 || (ace && c.rng.below(2) == 0) {
                    o.missions.push(MissionOrder {
                        sq: id,
                        mission: Mission::Fly { target },
                    });
                    escorted = true;
                    continue;
                }
            }
        }
        if !first_patrol && i > 0 {
            if let Some(prize) = prize_to_claim(c, &claimed) {
                claimed.push(prize);
                let target = Target::Prize { prize };
                o.missions.push(MissionOrder {
                    sq: id,
                    mission: Mission::Fly { target },
                });
                continue;
            }
        }
        if let Some(at) = best_patrol(c, &covered) {
            covered.push(at);
            o.missions.push(MissionOrder {
                sq: id,
                mission: Mission::Patrol { at },
            });
        }
    }
    // A spare bomber with nothing worth hitting goes for a prize nobody holds.
    for &(id, planes) in &bombers {
        if tired(planes) || o.mission(id) != Mission::Rest {
            continue;
        }
        if let Some(prize) = prize_to_claim(c, &claimed) {
            if c.st.prizes[prize as usize].owner.is_none() {
                claimed.push(prize);
                o.missions.push(MissionOrder {
                    sq: id,
                    mission: Mission::Fly {
                        target: Target::Prize { prize },
                    },
                });
            }
        }
    }
}

/// The prize most worth flying to: unheld or a rival's, close to home.
fn prize_to_claim(c: &mut Ctx, taken: &[u16]) -> Option<u16> {
    let from = airfield(c.w, c.st, c.seat);
    let mut best: Option<(f64, u16)> = None;
    for (i, p) in c.w.content.map.prizes.iter().enumerate() {
        let i = i as u16;
        if taken.contains(&i) || c.st.prizes[i as usize].owner == Some(c.seat) {
            continue;
        }
        let d = dist(from, p.pos) as f64;
        let cross = crossings(c.st, c.seat, from, p.pos).len() as f64;
        let held_by_rival = c.st.prizes[i as usize].owner.is_some();
        let score = 1000.0 / (d + 200.0) - cross * 0.8 + if held_by_rival { 0.2 } else { 0.6 };
        if score > 0.9 && best.is_none_or(|(b, _)| score > b) {
            best = Some((score, i));
        }
    }
    best.map(|b| b.1)
}

/// Where a patrol protects the most: over sites rivals can hit, weighted by
/// what they're worth, and across the lanes from rival Airfields.
fn best_patrol(c: &mut Ctx, covered: &[P]) -> Option<P> {
    let r = patrol_radius(c.w, c.st, c.seat);
    let f = c.st.faction(c.seat);
    let mine: Vec<u16> = [
        SiteKind::HQ,
        SiteKind::Factory,
        SiteKind::Airfield,
        SiteKind::Jewel,
    ]
    .iter()
    .map(|&k| c.w.site_of(f, k))
    .collect();
    let night = c.st.night;
    let weight = |s: u16| -> f64 {
        let k = c.w.site(s).kind;
        let open = night >= flyover_core::rules::opens_on(c.w, k);
        let soon = night + 1 >= flyover_core::rules::opens_on(c.w, k);
        let base = pip_value(c.w, s) * (c.st.sites[s as usize].health.max(1) as f64);
        if open {
            base
        } else if soon {
            base * 0.25
        } else {
            0.05
        }
    };
    let mut cands: Vec<P> = mine.iter().map(|&s| c.w.site(s).pos).collect();
    for i in 0..mine.len() {
        for j in i + 1..mine.len() {
            let (a, b) = (c.w.site(mine[i]).pos, c.w.site(mine[j]).pos);
            cands.push(((a.0 + b.0) / 2, (a.1 + b.1) / 2));
        }
    }
    let rivals: Vec<P> = (0..c.st.seats.len() as u8)
        .filter(|&s| s != c.seat)
        .map(|s| airfield(c.w, c.st, s))
        .collect();
    let mut best: Option<(f64, P)> = None;
    for at in cands {
        if patrol_ok(c.w, c.st, c.seat, at).is_err() {
            continue;
        }
        let mut score = 0.0;
        for &s in &mine {
            let pos = c.w.site(s).pos;
            let inside = dist(at, pos) <= r;
            let already = covered.iter().any(|&q| dist(q, pos) <= r);
            let w = weight(s) * if already { 0.35 } else { 1.0 };
            if inside {
                score += w;
            } else {
                // Lanes from rival Airfields to this site that pass through the circle.
                let lanes = rivals
                    .iter()
                    .filter(|&&from| circle_entry(from, pos, at, r).is_some())
                    .count();
                score += w * 0.25 * lanes as f64;
            }
        }
        score *= 0.9 + c.rng.below(200) as f64 / 1000.0;
        if best.is_none_or(|(b, _)| score > b) {
            best = Some((score, at));
        }
    }
    best.map(|b| b.1)
}

/// Easy bots: patrol the HQ, send bombers at a random open target, no escorts.
fn easy(c: &mut Ctx, o: &mut Orders) {
    let f = c.st.faction(c.seat);
    let hq = c.w.site(c.w.site_of(f, SiteKind::HQ)).pos;
    let options = targets(c);
    let mut patrolled = false;
    let squads: Vec<(u32, UnitKind)> = c.st.squadrons_of(c.seat).map(|s| (s.id, s.kind)).collect();
    for (id, kind) in squads {
        match kind {
            UnitKind::Fighter if !patrolled => {
                o.missions.push(MissionOrder {
                    sq: id,
                    mission: Mission::Patrol { at: hq },
                });
                patrolled = true;
            }
            UnitKind::Fighter => {
                if c.rng.below(2) == 0 {
                    let prize = c.rng.below(c.w.content.map.prizes.len() as u32) as u16;
                    o.missions.push(MissionOrder {
                        sq: id,
                        mission: Mission::Fly {
                            target: Target::Prize { prize },
                        },
                    });
                }
            }
            UnitKind::Bomber if !options.is_empty() => {
                let site = options[c.rng.below(options.len() as u32) as usize];
                o.missions.push(MissionOrder {
                    sq: id,
                    mission: Mission::Fly {
                        target: Target::Site { site },
                    },
                });
            }
            UnitKind::Bomber => {}
        }
    }
}

/// Repairs and builds, in order of what hurts most.
fn economy(c: &mut Ctx, o: &mut Orders) {
    let f = c.st.faction(c.seat);
    let me = &c.st.seats[c.seat as usize];
    let mut supply = me.supply;
    let repair = c.w.rules().costs.repair;
    let mut health: Vec<(SiteKind, u16, i32, i32)> = [
        SiteKind::Airfield,
        SiteKind::Factory,
        SiteKind::Jewel,
        SiteKind::HQ,
    ]
    .iter()
    .map(|&k| {
        let s = c.w.site_of(f, k);
        (
            k,
            s,
            c.st.sites[s as usize].health,
            c.st.sites[s as usize].max,
        )
    })
    .collect();
    let mut fix = |o: &mut Orders, supply: &mut i32, kind: SiteKind, upto: i32| {
        if let Some(h) = health.iter_mut().find(|h| h.0 == kind) {
            while h.2 < upto.min(h.3) && *supply >= repair {
                o.repairs.push(h.1);
                h.2 += 1;
                *supply -= repair;
            }
        }
    };
    fix(o, &mut supply, SiteKind::Airfield, 1);
    let airfield_up = site_up(c.w, c.st, c.seat, SiteKind::Airfield)
        || o.repairs
            .iter()
            .any(|&s| s == c.w.site_of(f, SiteKind::Airfield));
    let mut count = c.st.squadrons_of(c.seat).count();
    let (mut nf, mut nb) = (
        c.st.squadrons_of(c.seat)
            .filter(|s| s.kind == UnitKind::Fighter)
            .count(),
        c.st.squadrons_of(c.seat)
            .filter(|s| s.kind == UnitKind::Bomber)
            .count(),
    );
    let max = c.w.rules().max_squadrons;
    let mut build = |o: &mut Orders, supply: &mut i32, count: &mut usize| {
        if !airfield_up || *count >= max {
            return false;
        }
        let kind = if nb <= nf {
            UnitKind::Bomber
        } else {
            UnitKind::Fighter
        };
        let cost = build_cost(c.w, c.st, c.seat, kind);
        if *supply < cost {
            return false;
        }
        *supply -= cost;
        *count += 1;
        if kind == UnitKind::Bomber {
            nb += 1;
        } else {
            nf += 1;
        }
        o.builds.push(kind);
        true
    };
    if count < 4 {
        build(o, &mut supply, &mut count);
    }
    fix(o, &mut supply, SiteKind::Factory, 99);
    fix(o, &mut supply, SiteKind::Jewel, 1);
    while build(o, &mut supply, &mut count) {}
    fix(o, &mut supply, SiteKind::HQ, 1);
    fix(o, &mut supply, SiteKind::Jewel, 99);
    fix(o, &mut supply, SiteKind::Airfield, 99);
    fix(o, &mut supply, SiteKind::HQ, 99);
    // Drop anything the rules would refuse.
    o.missions.retain(|m| {
        c.st.squadron(m.sq)
            .is_some_and(|sq| mission_check(c.w, c.st, sq, &m.mission).is_ok())
    });
}

/// Where a mission's squadron would fly, for previews (None for Rest).
pub fn mission_point(world: &World, m: &Mission) -> Option<P> {
    match *m {
        Mission::Rest => None,
        Mission::Patrol { at } => Some(at),
        Mission::Fly { target } => Some(target_pos(world, target)),
    }
}
