//! Rule checks shared by the simulation, the bots and the planning screen.

use crate::content::{FactionId, Region, SiteKind, UnitKind, World};
use crate::geom::P;
use crate::state::{GameState, Mission, MissionOrder, Orders, Squadron, Target};

/// 0 = Saber Rattling, 1 = Open Season, 2 = Gloves Off.
pub fn stage(world: &World, night: u8) -> u8 {
    let r = world.rules();
    if night >= r.open_hqs {
        2
    } else if night >= r.open_factories {
        1
    } else {
        0
    }
}

/// The first night a kind of site can be hit.
pub fn opens_on(world: &World, k: SiteKind) -> u8 {
    let r = world.rules();
    match k {
        SiteKind::Airfield => 1,
        SiteKind::Factory => r.open_factories,
        SiteKind::HQ | SiteKind::Jewel => r.open_hqs,
    }
}

impl GameState {
    pub fn seat_of(&self, f: FactionId) -> Option<u8> {
        self.seats
            .iter()
            .position(|s| s.faction == Some(f))
            .map(|i| i as u8)
    }
    pub fn faction(&self, seat: u8) -> FactionId {
        self.seats[seat as usize]
            .faction
            .expect("every seat has a faction")
    }
    pub fn squadron(&self, id: u32) -> Option<&Squadron> {
        self.squadrons.iter().find(|s| s.id == id)
    }
    pub fn squadrons_of(&self, seat: u8) -> impl Iterator<Item = &Squadron> {
        self.squadrons.iter().filter(move |s| s.seat == seat)
    }
    pub fn over(&self, world: &World) -> bool {
        self.night > world.rules().nights
    }
    /// The seat that owns a site (None for factions not in the game).
    pub fn site_owner(&self, world: &World, site: u16) -> Option<u8> {
        self.seat_of(world.site(site).faction)
    }
}

/// A faction's perk works while its Crown Jewel stands.
pub fn perk_on(world: &World, state: &GameState, seat: u8) -> bool {
    let f = state.faction(seat);
    state.sites[world.site_of(f, SiteKind::Jewel) as usize].health > 0
}

pub fn has_perk(world: &World, state: &GameState, seat: u8, f: FactionId) -> bool {
    world.rules().perks && state.faction(seat) == f && perk_on(world, state, seat)
}

pub fn site_up(world: &World, state: &GameState, seat: u8, k: SiteKind) -> bool {
    state.sites[world.site_of(state.faction(seat), k) as usize].health > 0
}

pub fn patrol_radius(world: &World, state: &GameState, seat: u8) -> i32 {
    let p = &world.rules().patrol;
    if has_perk(world, state, seat, FactionId::FrontRange) {
        p.mountain_radius
    } else {
        p.radius
    }
}

pub fn max_planes(world: &World, state: &GameState, seat: u8, kind: UnitKind) -> i32 {
    let r = world.rules();
    if kind == UnitKind::Bomber && has_perk(world, state, seat, FactionId::Texas) {
        r.big_bomber_planes
    } else {
        r.planes
    }
}

pub fn build_cost(world: &World, state: &GameState, seat: u8, kind: UnitKind) -> i32 {
    let c = &world.rules().costs;
    let base = match kind {
        UnitKind::Fighter => c.fighter,
        UnitKind::Bomber => c.bomber,
    };
    if has_perk(world, state, seat, FactionId::Pacific) {
        base.min(c.move_fast)
    } else {
        base
    }
}

/// Can `seat` hit this site tonight? Err explains why not, in plain words.
pub fn site_open(world: &World, state: &GameState, seat: u8, site: u16) -> Result<(), String> {
    let s = world.site(site);
    if !state.sites[site as usize].active {
        return Err("Nobody lives there this game.".into());
    }
    if s.faction == state.faction(seat) {
        return Err("That's yours.".into());
    }
    let opens = opens_on(world, s.kind);
    if state.night < opens {
        return Err(format!("Opens on Night {opens}."));
    }
    Ok(())
}

pub fn target_ok(world: &World, state: &GameState, seat: u8, t: Target) -> Result<(), String> {
    match t {
        Target::Site { site } => {
            if site as usize >= world.content.map.sites.len() {
                return Err("No such place.".into());
            }
            site_open(world, state, seat, site)
        }
        Target::Prize { prize } => {
            if prize as usize >= world.content.map.prizes.len() {
                return Err("No such place.".into());
            }
            Ok(())
        }
    }
}

/// Patrols fly over your own land or the Big Empty (which includes the land
/// of factions not in this game), never over a rival's home.
pub fn patrol_ok(world: &World, state: &GameState, seat: u8, at: P) -> Result<(), String> {
    match world.region_at(at) {
        None => Err("Patrols fly over land.".into()),
        Some(Region::BigEmpty) => Ok(()),
        Some(r) => {
            let f = r.faction().expect("home regions belong to factions");
            if f == state.faction(seat) || state.seat_of(f).is_none() {
                Ok(())
            } else {
                Err("Patrol over your own land or the Big Empty.".into())
            }
        }
    }
}

pub fn mission_check(
    world: &World,
    state: &GameState,
    sq: &Squadron,
    m: &Mission,
) -> Result<(), String> {
    match *m {
        Mission::Rest => Ok(()),
        Mission::Patrol { at } => {
            if sq.kind != UnitKind::Fighter {
                return Err("Only fighters patrol.".into());
            }
            patrol_ok(world, state, sq.seat, at)
        }
        Mission::Fly { target } => target_ok(world, state, sq.seat, target),
    }
}

/// Can this seat go to Project Bark Park, given the scores now?
pub fn doghouse_eligible(world: &World, state: &GameState, seat: u8, night: u8) -> bool {
    let b = &world.rules().bark_park;
    let me = &state.seats[seat as usize];
    if me.been_to_park || night < b.first_night || night > b.last_night {
        return false;
    }
    let best = state.seats.iter().map(|s| s.score).max().unwrap_or(0);
    let worst = state.seats.iter().map(|s| s.score).min().unwrap_or(0);
    let alive = state.squadrons_of(seat).count();
    (me.score == worst && best - me.score >= b.gap) || alive <= b.low_squadrons
}

/// Orders cleaned up the way the simulation will read them: unknown or
/// illegal missions become Rest, unaffordable builds and repairs are dropped,
/// and the park and the Big Fetch need to be available.
pub fn normalize(world: &World, state: &GameState, orders: &Orders) -> Orders {
    let seat = orders.seat;
    let me = &state.seats[seat as usize];
    let retreat = orders.retreat && me.doghouse;
    let mut missions = Vec::new();
    for sq in state.squadrons_of(seat) {
        let m = if retreat {
            Mission::Rest
        } else {
            orders.mission(sq.id)
        };
        let m = if mission_check(world, state, sq, &m).is_ok() {
            m
        } else {
            Mission::Rest
        };
        if m != Mission::Rest {
            missions.push(MissionOrder {
                sq: sq.id,
                mission: m,
            });
        }
    }
    let mut supply = me.supply;
    let mut repairs = Vec::new();
    let mut fixed = vec![0; state.sites.len()];
    let repair = world.rules().costs.repair;
    for &site in &orders.repairs {
        let i = site as usize;
        if i >= state.sites.len() || world.site(site).faction != state.faction(seat) {
            continue;
        }
        let s = &state.sites[i];
        if s.health + fixed[i] < s.max && supply >= repair {
            supply -= repair;
            fixed[i] += 1;
            repairs.push(site);
        }
    }
    let mut builds = Vec::new();
    let mut count = state.squadrons_of(seat).count();
    if site_up(world, state, seat, SiteKind::Airfield) {
        for &k in &orders.builds {
            let cost = build_cost(world, state, seat, k);
            if count < world.rules().max_squadrons && supply >= cost {
                supply -= cost;
                count += 1;
                builds.push(k);
            }
        }
    }
    let fetch = if me.fetch && !retreat {
        orders.fetch.filter(|&s| {
            (s as usize) < state.sites.len() && site_open(world, state, seat, s).is_ok()
        })
    } else {
        None
    };
    Orders {
        seat,
        missions,
        builds,
        repairs,
        retreat,
        fetch,
    }
}

/// What normalized orders cost.
pub fn spend(world: &World, state: &GameState, orders: &Orders) -> i32 {
    let repairs = orders.repairs.len() as i32 * world.rules().costs.repair;
    let builds: i32 = orders
        .builds
        .iter()
        .map(|&k| build_cost(world, state, orders.seat, k))
        .sum();
    repairs + builds
}
