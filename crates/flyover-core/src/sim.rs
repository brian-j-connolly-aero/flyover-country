//! `resolve_night(state, orders) -> (state, events)`: the whole night, resolved
//! at once on a fixed timestep (one tick is one game minute), written to an
//! ordered event log that the show plays back.

use crate::content::{dist, dist2, FactionId, NodeKind, PatchEffect, UnitKind, World, P};
use crate::economy;
use crate::rng::{mix, Rng};
use crate::route::{self, Threat};
use crate::rules;
use crate::setup;
use crate::state::*;
use crate::vision;

/// Fixed-point positions: map units times 16.
const FX: i32 = 16;
type Fp = (i32, i32);

fn to_fp(p: P) -> Fp {
    (p.0 * FX, p.1 * FX)
}
fn to_map(p: Fp) -> P {
    (p.0 / FX, p.1 / FX)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GKind {
    Strike(u16),
    Guard(P),
    Sweep(u16),
    Scout(u16),
    JamZone(u16),
    Rebase(u16),
    Smoke(u16),
    Watch(P),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Waiting,
    Outbound,
    OnStation,
    Returning,
    Done,
}

#[derive(Clone, Debug)]
struct Group {
    id: u32,
    seat: u8,
    members: Vec<usize>,
    payload: Vec<usize>,
    kind: GKind,
    tag: MissionTag,
    base: u16,
    path: Vec<Fp>,
    leg: usize,
    pos: Fp,
    speed: i32,
    phase: Phase,
    launch: i32,
    station_until: i32,
    orbit: Vec<Fp>,
    orbit_i: usize,
    guard_node: Option<u16>,
    public: bool,
    track: Vec<(i32, i32, i32)>,
    ad_in: Vec<u16>,
    cooldown: Vec<(u32, i32)>,
    engaged: bool,
    launch_payload: i32,
    launch_strength: i32,
    revealed: bool,
    vanish: bool,
    guard_radius: i32,
    guest_of: Option<u8>,
    arrived: bool,
    aborted: bool,
    launch_ev: usize,
    arrived_now: bool,
    pivoted: bool,
}

#[derive(Clone, Debug, Default)]
struct SqRt {
    group: Option<usize>,
    consumed: bool,
    lost: bool,
    grounded: bool,
    rested: bool,
    jiggled: bool,
    lucky: bool,
}

struct Plan {
    sq: usize,
    mission: Mission,
    path: Vec<P>,
}

struct Sim<'w> {
    w: &'w World,
    st: GameState,
    rng: Rng,
    ev: Vec<EventRec>,
    tick: i32,
    night: u8,
    groups: Vec<Group>,
    rt: Vec<SqRt>,
    blinded: Vec<bool>,
    hit_tonight: Vec<bool>,
    radar: Vec<Vec<(P, i32)>>,
    threats: Vec<Vec<Threat>>,
    next_engage: u32,
    bets: Vec<Option<(Bet, bool)>>,
    harvest: Option<(u8, i32)>,
    casseroles: Vec<(u8, Option<u8>)>,
    drone_targets: Vec<(u16, i32)>,
    orders: Vec<Orders>,
    guarded_by_beehive: Vec<(u16, u8)>,
    massdriver_holder: Option<u8>,
    scouted: Vec<(u8, u16)>,
    hit_texas: Vec<u8>,
}

pub fn resolve_night(world: &World, state: &GameState, orders: &[Orders]) -> (GameState, Vec<EventRec>) {
    let mut sim = Sim::new(world, state.clone(), orders);
    sim.run();
    (sim.st, sim.ev)
}

impl<'w> Sim<'w> {
    fn new(w: &'w World, st: GameState, orders: &[Orders]) -> Sim<'w> {
        let night = st.night;
        let rng = Rng::for_night(st.seed, night, 0);
        let n = st.squadrons.len();
        let nodes = w.nodes.len();
        let seats = st.seats.len();
        let mut ord: Vec<Orders> = (0..seats as u8).map(|s| Orders { seat: s, ..Default::default() }).collect();
        for o in orders {
            if (o.seat as usize) < seats {
                ord[o.seat as usize] = o.clone();
            }
        }
        Sim {
            w,
            st,
            rng,
            ev: Vec::new(),
            tick: 0,
            night,
            groups: Vec::new(),
            rt: vec![SqRt::default(); n],
            blinded: vec![false; nodes],
            hit_tonight: vec![false; nodes],
            radar: vec![Vec::new(); seats],
            threats: Vec::new(),
            next_engage: 1,
            bets: vec![None; seats],
            harvest: None,
            casseroles: Vec::new(),
            drone_targets: Vec::new(),
            orders: ord,
            guarded_by_beehive: Vec::new(),
            massdriver_holder: None,
            scouted: Vec::new(),
            hit_texas: Vec::new(),
        }
    }

    fn push(&mut self, public: bool, seats: Vec<u8>, e: Event) -> usize {
        self.ev.push(EventRec { tick: self.tick, public, seats, e });
        self.ev.len() - 1
    }
    fn public(&mut self, e: Event) {
        self.push(true, Vec::new(), e);
    }
    fn private(&mut self, seat: u8, e: Event) {
        self.push(false, vec![seat], e);
    }

    fn faction(&self, seat: u8) -> FactionId {
        self.st.seats[seat as usize].faction
    }

    fn sq_alive(&self, i: usize) -> bool {
        !self.rt[i].lost && !self.rt[i].consumed && self.st.squadrons[i].strength > 0
    }

    fn sq_snap(&self, i: usize) -> SqSnap {
        let s = &self.st.squadrons[i];
        SqSnap { sq: s.id, seat: s.seat, kind: s.kind, strength: s.strength, ace: s.ace, stars: s.stars }
    }

    fn sq_index(&self, id: u32) -> Option<usize> {
        self.st.squadrons.iter().position(|s| s.id == id)
    }

    fn is_ace_of(&self, i: usize, f: FactionId) -> bool {
        let s = &self.st.squadrons[i];
        s.ace && self.faction(s.seat) == f
    }

    fn sq_stealth(&self, i: usize, g: &Group) -> bool {
        let s = &self.st.squadrons[i];
        (self.w.unit(s.kind).stealth && !g.revealed) || self.is_ace_of(i, FactionId::Cascadia)
    }

    fn run(&mut self) {
        self.night_start();
        let ticks = self.w.rules().ticks;
        while self.tick < ticks {
            self.step();
            self.tick += 1;
        }
        self.night_end();
        self.dawn();
    }

    // --- Night start: orders, builds, repairs, faction actions ---------------

    fn night_start(&mut self) {
        let w = self.w;
        let night = self.night;
        // Threat pictures for routing come from what each seat knows at dusk.
        self.threats = (0..self.st.seats.len() as u8).map(|s| route::threats_for(w, &self.st, s)).collect();
        self.massdriver_holder = w
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::MassDriver)
            .and_then(|n| self.st.nodes[n.idx as usize].owner);

        for seat in 0..self.st.seats.len() as u8 {
            let o = self.orders[seat as usize].clone();
            // Paid repairs happen right away.
            let mut repaired: Vec<u16> = Vec::new();
            for &node in &o.repairs {
                if (node as usize) >= w.nodes.len() || repaired.contains(&node) {
                    continue;
                }
                let cost = w.rules().economy.repair_cost;
                let ns = &self.st.nodes[node as usize];
                if ns.owner == Some(seat) && ns.health < 2 && self.st.seats[seat as usize].supply >= cost {
                    self.st.seats[seat as usize].supply -= cost;
                    self.st.nodes[node as usize].health += 1;
                    repaired.push(node);
                    let h = self.st.nodes[node as usize].health;
                    self.private(seat, Event::Repaired { seat, node, health: h, paid: true });
                }
            }
            // Builds: pay now, arrive at dawn (Cascadia: a night later, as veterans).
            let mut cap = rules::build_capacity(w, &self.st, seat);
            for &kind in &o.builds {
                if let Err(r) = rules::buildable(w, &self.st, seat, kind) {
                    self.private(seat, Event::BuildFailed { seat, kind, reason: r.into() });
                    continue;
                }
                let cost = w.unit(kind).cost;
                if cap <= 0 {
                    self.private(seat, Event::BuildFailed { seat, kind, reason: "The factories are booked solid tonight.".into() });
                    continue;
                }
                if self.st.seats[seat as usize].supply < cost {
                    self.private(seat, Event::BuildFailed { seat, kind, reason: "Not enough Supply.".into() });
                    continue;
                }
                cap -= 1;
                self.st.seats[seat as usize].supply -= cost;
                let cascadia = self.faction(seat) == FactionId::Cascadia;
                let delay = if cascadia { 1 + w.rules().factions.process_delay as u8 } else { 1 };
                self.st.pending.push(PendingBuild { seat, kind, night: night + delay, stars: if cascadia { 1 } else { 0 } });
            }
            // Dreamland's secret bet.
            if self.faction(seat) == FactionId::Dreamland {
                if let Some(b) = o.bet.clone() {
                    let max = w.rules().factions.bet_max.min(self.st.seats[seat as usize].supply);
                    let stake = b.stake.clamp(0, max);
                    let valid_target = b.node.map(|n| (n as usize) < w.nodes.len()).unwrap_or(false)
                        || b.victim.map(|v| (v as usize) < self.st.seats.len()).unwrap_or(false);
                    if stake > 0 && b.on != seat && (b.on as usize) < self.st.seats.len() && valid_target {
                        self.st.seats[seat as usize].supply -= stake;
                        self.bets[seat as usize] = Some((Bet { stake, ..b }, false));
                    }
                }
            }
        }
        // The Harvest auction: highest secret bid wins tonight's shipment.
        let mut harvest_winner = None;
        let mut harvest_bid = 0;
        if self.st.harvest > 0 {
            if let Some(gb) = rules::seat_of(&self.st, FactionId::GrainBelt) {
                let mut best: Vec<(u8, i32)> = Vec::new();
                for seat in 0..self.st.seats.len() as u8 {
                    if seat == gb {
                        continue;
                    }
                    let bid = self.orders[seat as usize].harvest_bid.min(self.st.seats[seat as usize].supply);
                    if bid <= 0 {
                        continue;
                    }
                    if best.is_empty() || bid > best[0].1 {
                        best = vec![(seat, bid)];
                    } else if bid == best[0].1 {
                        best.push((seat, bid));
                    }
                }
                if !best.is_empty() {
                    let pick = best[self.rng.below(best.len() as u32) as usize];
                    self.st.seats[pick.0 as usize].supply -= pick.1;
                    self.harvest = Some(pick);
                    harvest_winner = Some(pick.0);
                    harvest_bid = pick.1;
                }
            }
        }
        let stage = rules::stage(w, night);
        let patch = self.st.patch;
        let harvest_value = self.st.harvest;
        self.public(Event::NightStart { night, stage, patch, harvest_winner, harvest_bid, harvest_value });

        self.plan_missions();
    }

    fn patch_effect(&self) -> Option<PatchEffect> {
        self.st.patch.and_then(|i| self.w.content.patch_notes.get(i as usize)).map(|p| p.effect)
    }

    fn is_patched_drone(&self, i: usize) -> bool {
        let s = &self.st.squadrons[i];
        s.kind == UnitKind::Drone && self.faction(s.seat) == FactionId::Pacific
    }

    fn unit_speed(&self, i: usize) -> i32 {
        let s = &self.st.squadrons[i];
        let mut v = self.w.unit(s.kind).speed;
        if self.is_patched_drone(i) {
            if let Some(PatchEffect::Speed(d)) = self.patch_effect() {
                v += d;
            }
        }
        v.max(10)
    }

    /// Validate orders, budget fuel and form flight groups.
    fn plan_missions(&mut self) {
        let w = self.w;
        let night = self.night;
        // 1. Standing orders, updated by tonight's orders.
        let mut order_rank: Vec<(u8, usize, usize)> = Vec::new(); // (seat, rank, sq index)
        for i in 0..self.st.squadrons.len() {
            let seat = self.st.squadrons[i].seat;
            let o = &self.orders[seat as usize];
            let id = self.st.squadrons[i].id;
            let rank = o.missions.iter().position(|m| m.sq == id);
            if let Some(r) = rank {
                let so = o.missions[r].clone();
                self.st.squadrons[i].order = so.mission;
                self.st.squadrons[i].waypoint = so.waypoint;
            }
            order_rank.push((seat, rank.unwrap_or(usize::MAX), i));
        }
        order_rank.sort();
        // 2. Validate and route.
        let mut plans: Vec<Plan> = Vec::new();
        let mut escorts: Vec<(usize, u32, bool)> = Vec::new(); // (sq, target id, jammer)
        for &(_, _, i) in &order_rank {
            let sq = self.st.squadrons[i].clone();
            if !sq.active() {
                continue;
            }
            let m = sq.order.clone();
            if m == Mission::Rest {
                self.rt[i].rested = true;
                continue;
            }
            if let Err(r) = rules::mission_check(w, &self.st, &sq, &m) {
                self.st.squadrons[i].order = Mission::Rest;
                self.rt[i].rested = true;
                self.rt[i].grounded = true;
                self.private(sq.seat, Event::Grounded { seat: sq.seat, sq: sq.id, reason: r });
                continue;
            }
            if sq.kind == UnitKind::Drone && rules::effect_active(&self.st, sq.seat, EffectKind::NoDrones, night) {
                self.rt[i].rested = true;
                self.rt[i].grounded = true;
                self.private(sq.seat, Event::Grounded { seat: sq.seat, sq: sq.id, reason: "Scheduled maintenance: the Server Farm is down.".into() });
                continue;
            }
            if self.st.nodes[sq.base as usize].health == 0 && !matches!(m, Mission::Rebase { .. }) && w.nodes[sq.base as usize].kind != NodeKind::Lake && w.nodes[sq.base as usize].kind != NodeKind::Capital {
                self.rt[i].rested = true;
                self.rt[i].grounded = true;
                self.private(sq.seat, Event::Grounded { seat: sq.seat, sq: sq.id, reason: "Runway's cratered. Rebase or wait for repairs.".into() });
                continue;
            }
            match m {
                Mission::Escort { sq: t } => escorts.push((i, t, false)),
                Mission::JamEscort { sq: t } => escorts.push((i, t, true)),
                _ => {
                    let from = w.nodes[sq.base as usize].pos;
                    let to = self.mission_point(&m);
                    let path = route::plan(w, &self.threats[sq.seat as usize], from, to, sq.waypoint);
                    plans.push(Plan { sq: i, mission: m, path });
                }
            }
        }
        // 3. Fuel, in priority order. Escorts pay for the route they fly.
        let mut flying: Vec<usize> = Vec::new(); // plan indices
        let mut flying_escorts: Vec<(usize, usize, bool)> = Vec::new(); // (sq, plan idx, jammer)
        let mut spent = vec![0i32; self.st.seats.len()];
        for &(seat, _, i) in &order_rank {
            if let Some(pi) = plans.iter().position(|p| p.sq == i) {
                let cost = self.fuel_cost(i, &plans[pi]);
                if self.st.seats[seat as usize].fuel - spent[seat as usize] >= cost {
                    spent[seat as usize] += cost;
                    flying.push(pi);
                } else {
                    self.rt[i].rested = true;
                    self.rt[i].grounded = true;
                    let id = self.st.squadrons[i].id;
                    self.private(seat, Event::Grounded { seat, sq: id, reason: "Not enough Fuel.".into() });
                }
            } else if let Some(&(_, t, jam)) = escorts.iter().find(|e| e.0 == i) {
                let target = self.sq_index(t).and_then(|ti| plans.iter().position(|p| p.sq == ti));
                match target {
                    Some(pi) if flying.contains(&pi) => {
                        let plan = &plans[pi];
                        let len = route::path_len(&plan.path) * 2;
                        let rate = w.unit(self.st.squadrons[i].kind).fuel_rate;
                        let cost = ((len * rate + 999) / 1000).max(1);
                        if self.st.seats[seat as usize].fuel - spent[seat as usize] >= cost {
                            spent[seat as usize] += cost;
                            flying_escorts.push((i, pi, jam));
                        } else {
                            self.rt[i].rested = true;
                            self.rt[i].grounded = true;
                            let id = self.st.squadrons[i].id;
                            self.private(seat, Event::Grounded { seat, sq: id, reason: "Not enough Fuel.".into() });
                        }
                    }
                    _ => {
                        self.rt[i].rested = true;
                        self.rt[i].grounded = true;
                        let id = self.st.squadrons[i].id;
                        self.private(seat, Event::Grounded { seat, sq: id, reason: "Nothing to fly with tonight.".into() });
                    }
                }
            }
        }
        for (s, v) in spent.iter().enumerate() {
            self.st.seats[s].fuel -= v;
        }
        // 4. Groups. Strikes on the same target from the same base fly together.
        let mut plan_group: Vec<Option<usize>> = vec![None; plans.len()];
        for &pi in &flying {
            let p = &plans[pi];
            let sq = self.st.squadrons[p.sq].clone();
            if let Mission::Attack { node } = p.mission {
                if let Some(gi) = self.groups.iter().position(|g| {
                    g.seat == sq.seat && g.base == sq.base && g.kind == GKind::Strike(node) && g.phase == Phase::Waiting
                }) {
                    self.groups[gi].members.push(p.sq);
                    self.groups[gi].payload.push(p.sq);
                    plan_group[pi] = Some(gi);
                    continue;
                }
            }
            let gi = self.new_group(p);
            plan_group[pi] = Some(gi);
        }
        for (i, pi, _) in flying_escorts {
            if let Some(gi) = plan_group[pi] {
                self.groups[gi].members.push(i);
            }
        }
        // Drones: count swarms per target for the swarm bonus.
        for g in &self.groups {
            if let GKind::Strike(node) = g.kind {
                let n = g.members.iter().filter(|&&i| self.st.squadrons[i].kind == UnitKind::Drone).count() as i32;
                if n > 0 {
                    match self.drone_targets.iter_mut().find(|d| d.0 == node) {
                        Some(d) => d.1 += n,
                        None => self.drone_targets.push((node, n)),
                    }
                }
            }
        }
        // 5. Finalize groups: speed, launch time, radar-aided guard radius.
        for gi in 0..self.groups.len() {
            self.finalize_group(gi);
        }
    }

    fn mission_point(&self, m: &Mission) -> P {
        let w = self.w;
        match *m {
            Mission::Attack { node } | Mission::Guard { node } | Mission::Rebase { node } => w.nodes[node as usize].pos,
            Mission::GuardZone { zone } | Mission::Sweep { zone } | Mission::Scout { zone } | Mission::JamZone { zone } => {
                w.zones[zone as usize].center
            }
            _ => (0, 0),
        }
    }

    fn fuel_cost(&self, i: usize, plan: &Plan) -> i32 {
        fuel_cost(self.w, &self.st, &self.st.squadrons[i], &plan.mission, &plan.path)
    }

    fn new_group(&mut self, p: &Plan) -> usize {
        let w = self.w;
        let sq = self.st.squadrons[p.sq].clone();
        let (kind, tag) = match p.mission {
            Mission::Attack { node } => (GKind::Strike(node), MissionTag::Attack),
            Mission::Guard { node } => match sq.kind {
                UnitKind::CropDuster => (GKind::Smoke(node), MissionTag::Smoke),
                UnitKind::Watchtower => (GKind::Watch(w.nodes[node as usize].pos), MissionTag::Watch),
                _ => (GKind::Guard(w.nodes[node as usize].pos), MissionTag::Guard),
            },
            Mission::GuardZone { zone } => match sq.kind {
                UnitKind::Watchtower => (GKind::Watch(w.zones[zone as usize].center), MissionTag::Watch),
                UnitKind::CropDuster => (GKind::Smoke(u16::MAX), MissionTag::Smoke),
                _ => (GKind::Guard(w.zones[zone as usize].center), MissionTag::Guard),
            },
            Mission::Sweep { zone } => (GKind::Sweep(zone), MissionTag::Sweep),
            Mission::Scout { zone } => (GKind::Scout(zone), MissionTag::Scout),
            Mission::JamZone { zone } => (GKind::JamZone(zone), MissionTag::Jam),
            Mission::Rebase { node } => (GKind::Rebase(node), MissionTag::Rebase),
            _ => (GKind::Guard(w.nodes[sq.base as usize].pos), MissionTag::Guard),
        };
        let guard_node = match p.mission {
            Mission::Guard { node } => Some(node),
            _ => None,
        };
        let guest_of = guard_node.and_then(|n| self.st.nodes[n as usize].owner).filter(|&o| o != sq.seat);
        let id = (self.groups.len() + 1) as u32;
        let path: Vec<Fp> = p.path.iter().map(|&q| to_fp(q)).collect();
        let pos = path[0];
        self.groups.push(Group {
            id,
            seat: sq.seat,
            members: vec![p.sq],
            payload: vec![p.sq],
            kind,
            tag,
            base: sq.base,
            path,
            leg: 1,
            pos,
            speed: 0,
            phase: Phase::Waiting,
            launch: 0,
            station_until: 300,
            orbit: Vec::new(),
            orbit_i: 0,
            guard_node,
            public: false,
            track: Vec::new(),
            ad_in: Vec::new(),
            cooldown: Vec::new(),
            engaged: false,
            launch_payload: 0,
            launch_strength: 0,
            revealed: false,
            vanish: false,
            guard_radius: 0,
            guest_of,
            arrived: false,
            aborted: false,
            launch_ev: usize::MAX,
            arrived_now: false,
            pivoted: false,
        });
        if let (Some(n), Some(_)) = (guard_node, guest_of) {
            if self.faction(sq.seat) == FactionId::Beehive {
                self.guarded_by_beehive.push((n, sq.seat));
            }
        }
        self.groups.len() - 1
    }

    fn finalize_group(&mut self, gi: usize) {
        let w = self.w;
        let r = w.rules();
        let members = self.groups[gi].members.clone();
        let speed_tenths = members.iter().map(|&i| self.unit_speed(i)).min().unwrap_or(40);
        let speed = speed_tenths * FX / 10;
        for &i in &members {
            self.rt[i].group = Some(gi);
        }
        let g = &self.groups[gi];
        let len_fp: i32 = g.path.windows(2).map(|w2| dist(w2[0], w2[1])).sum();
        let flight = len_fp / speed.max(1);
        let salt = mix(self.st.seed, (self.night as u64) << 16 | g.id as u64);
        let launch = match g.kind {
            GKind::Strike(_) => {
                let tot = 110 + (salt % 110) as i32;
                (tot - flight).max(4 + (salt % 9) as i32)
            }
            GKind::Scout(_) => 8 + (salt % 12) as i32,
            GKind::Rebase(_) => 20 + (salt % 20) as i32,
            _ => 1 + (salt % 6) as i32,
        };
        let payload: i32 = g.payload.iter().map(|&i| self.st.squadrons[i].strength as i32).sum();
        let total: i32 = members.iter().map(|&i| self.st.squadrons[i].strength as i32).sum();
        let seat = g.seat;
        let kind = g.kind;
        let has_harvest_moon = members.iter().any(|&i| self.is_ace_of(i, FactionId::GrainBelt));
        let station = match kind {
            GKind::Guard(p) => Some(p),
            _ => None,
        };
        let src = vision::radar_sources(w, &self.st, seat, self.night, &[]);
        let mut radius = match station {
            Some(p) if vision::covered(&src, p) => r.radar.guard_radar,
            _ => r.radar.guard_visual,
        };
        if has_harvest_moon {
            radius *= 2;
        }
        // Sweeps and jammers orbit their zone.
        let orbit = match kind {
            GKind::Sweep(z) | GKind::JamZone(z) => {
                let zr = &w.zones[z as usize];
                let rad = if matches!(kind, GKind::JamZone(_)) { zr.radius / 3 } else { zr.radius * 2 / 3 };
                orbit_points(zr.center, rad, (salt % 16) as usize)
            }
            _ => Vec::new(),
        };
        let vanish = members.iter().any(|&i| self.st.squadrons[i].kind == UnitKind::Prototype)
            && self.rng.chance(r.factions.prototype_vanish);
        let g = &mut self.groups[gi];
        g.speed = speed.max(FX);
        g.launch = launch;
        g.launch_payload = payload;
        g.launch_strength = total;
        g.guard_radius = radius;
        g.orbit = orbit;
        g.vanish = vanish;
    }

    // --- The tick loop -----------------------------------------------------------

    fn step(&mut self) {
        let t = self.tick;
        // Radar pictures (radar sites can be hit or jammed mid-night).
        for s in 0..self.st.seats.len() as u8 {
            let mut src = vision::radar_sources(self.w, &self.st, s, self.night, &self.blinded);
            for g in &self.groups {
                if g.seat == s && airborne(g) {
                    for &i in &g.members {
                        if self.sq_alive(i) {
                            let rr = self.w.unit(self.st.squadrons[i].kind).radar;
                            if rr > 0 {
                                src.push((to_map(g.pos), rr));
                            }
                        }
                    }
                }
            }
            self.radar[s as usize] = src;
        }
        for gi in 0..self.groups.len() {
            self.move_group(gi);
        }
        self.update_jamming();
        self.air_defense();
        self.engagements();
        for gi in 0..self.groups.len() {
            if self.groups[gi].arrived_now {
                self.groups[gi].arrived_now = false;
                self.arrive(gi);
            }
        }
        if t == self.w.rules().factions.pivot_tick {
            self.pivot();
        }
        if t == 100 {
            self.mass_driver();
            self.revenge();
        }
        if t % 3 == 0 {
            for g in &mut self.groups {
                if airborne(g) {
                    let p = to_map(g.pos);
                    g.track.push((t, p.0, p.1));
                }
            }
        }
    }

    fn alive_members(&self, gi: usize) -> Vec<usize> {
        self.groups[gi].members.iter().copied().filter(|&i| self.sq_alive(i)).collect()
    }

    fn move_group(&mut self, gi: usize) {
        let t = self.tick;
        if self.groups[gi].phase != Phase::Done && self.alive_members(gi).is_empty() && self.groups[gi].phase != Phase::Waiting {
            self.groups[gi].phase = Phase::Done;
            return;
        }
        match self.groups[gi].phase {
            Phase::Waiting => {
                if t >= self.groups[gi].launch {
                    if self.alive_members(gi).is_empty() {
                        self.groups[gi].phase = Phase::Done;
                        return;
                    }
                    self.groups[gi].phase = Phase::Outbound;
                    let g = &self.groups[gi];
                    let squadrons = g.members.iter().map(|&i| self.sq_snap(i)).collect();
                    let (target_node, target_zone) = match g.kind {
                        GKind::Strike(n) | GKind::Rebase(n) | GKind::Smoke(n) => (Some(n).filter(|&n| n != u16::MAX), None),
                        GKind::Sweep(z) | GKind::Scout(z) | GKind::JamZone(z) => (None, Some(z)),
                        GKind::Guard(_) | GKind::Watch(_) => (g.guard_node, None),
                    };
                    let e = Event::Launch { group: g.id, seat: g.seat, mission: g.tag, squadrons, base: g.base, target_node, target_zone };
                    let seat = g.seat;
                    let p = to_map(g.pos);
                    let ix = self.push(false, vec![seat], e);
                    self.groups[gi].launch_ev = ix;
                    self.groups[gi].track.push((t, p.0, p.1));
                }
            }
            Phase::Outbound | Phase::Returning => {
                let g = &mut self.groups[gi];
                let mut budget = g.speed;
                while budget > 0 && g.leg < g.path.len() {
                    let target = g.path[g.leg];
                    let d = dist(g.pos, target);
                    if d <= budget {
                        g.pos = target;
                        budget -= d;
                        g.leg += 1;
                    } else {
                        g.pos = (g.pos.0 + (target.0 - g.pos.0) * budget / d, g.pos.1 + (target.1 - g.pos.1) * budget / d);
                        budget = 0;
                    }
                }
                if g.leg >= g.path.len() {
                    if g.phase == Phase::Outbound {
                        g.arrived_now = true;
                    } else {
                        g.phase = Phase::Done;
                        let p = to_map(g.pos);
                        g.track.push((t, p.0, p.1));
                    }
                }
                if self.w.in_static(to_map(self.groups[gi].pos)) {
                    self.lost_in_static(gi);
                }
            }
            Phase::OnStation => {
                if t >= self.groups[gi].station_until {
                    self.go_home(gi);
                    return;
                }
                let g = &mut self.groups[gi];
                if !g.orbit.is_empty() {
                    let mut budget = g.speed / 2;
                    while budget > 0 {
                        let target = g.orbit[g.orbit_i % g.orbit.len()];
                        let d = dist(g.pos, target);
                        if d <= budget {
                            g.pos = target;
                            budget -= d.max(1);
                            g.orbit_i += 1;
                        } else {
                            g.pos = (g.pos.0 + (target.0 - g.pos.0) * budget / d, g.pos.1 + (target.1 - g.pos.1) * budget / d);
                            budget = 0;
                        }
                    }
                }
            }
            Phase::Done => {}
        }
    }

    fn go_home(&mut self, gi: usize) {
        let w = self.w;
        let g = &mut self.groups[gi];
        let home = to_fp(w.nodes[g.base as usize].pos);
        // Strikes retrace their planned route; everyone else flies straight home.
        let path = if matches!(g.kind, GKind::Strike(_)) && !g.aborted && g.path.len() > 2 {
            let mut p: Vec<Fp> = g.path.iter().rev().copied().collect();
            p[0] = g.pos;
            p
        } else {
            vec![g.pos, home]
        };
        g.path = path;
        g.leg = 1;
        g.phase = Phase::Returning;
        if g.vanish {
            g.vanish = false;
            let members = g.members.clone();
            let seat = g.seat;
            let public = g.public;
            let p = to_map(g.pos);
            for i in members {
                if self.st.squadrons[i].kind == UnitKind::Prototype && self.sq_alive(i) {
                    self.rt[i].lost = true;
                    let id = self.st.squadrons[i].id;
                    self.push(public, vec![seat], Event::Vanished { sq: id, seat, x: p.0, y: p.1 });
                }
            }
        }
    }

    fn lost_in_static(&mut self, gi: usize) {
        let p = to_map(self.groups[gi].pos);
        let seat = self.groups[gi].seat;
        self.groups[gi].public = true;
        for i in self.alive_members(gi) {
            self.rt[i].lost = true;
            let id = self.st.squadrons[i].id;
            self.public(Event::LostInStatic { sq: id, seat, x: p.0, y: p.1 });
        }
        self.groups[gi].phase = Phase::Done;
    }

    fn arrive(&mut self, gi: usize) {
        let w = self.w;
        if self.alive_members(gi).is_empty() {
            self.groups[gi].phase = Phase::Done;
            return;
        }
        self.groups[gi].arrived = true;
        match self.groups[gi].kind {
            GKind::Strike(node) => {
                self.strike(gi, node);
                let all_one_way = self.alive_members(gi).iter().all(|&i| w.unit(self.st.squadrons[i].kind).one_way);
                for i in self.alive_members(gi) {
                    if w.unit(self.st.squadrons[i].kind).one_way {
                        self.rt[i].consumed = true;
                    }
                }
                if all_one_way {
                    self.groups[gi].phase = Phase::Done;
                } else {
                    self.go_home(gi);
                }
            }
            GKind::Guard(_) | GKind::Watch(_) | GKind::Sweep(_) | GKind::JamZone(_) => {
                self.groups[gi].phase = Phase::OnStation;
                if let GKind::JamZone(z) = self.groups[gi].kind {
                    let seat = self.groups[gi].seat;
                    let id = self.groups[gi].id;
                    self.private(seat, Event::JamOn { group: id, seat, zone: z });
                }
            }
            GKind::Scout(z) => {
                let seat = self.groups[gi].seat;
                self.scouted.push((seat, z));
                self.groups[gi].phase = Phase::OnStation;
                self.groups[gi].station_until = self.tick + 15;
            }
            GKind::Rebase(node) => {
                for i in self.alive_members(gi) {
                    self.st.squadrons[i].base = node;
                    if self.st.squadrons[i].order == (Mission::Rebase { node }) {
                        self.st.squadrons[i].order = Mission::Rest;
                    }
                }
                self.groups[gi].base = node;
                self.groups[gi].phase = Phase::Done;
            }
            GKind::Smoke(node) => {
                let seat = self.groups[gi].seat;
                let id = self.groups[gi].id;
                if node != u16::MAX {
                    self.st.smoke.retain(|s| s.node != node || s.seat != seat);
                    self.st.smoke.push(Smoke { node, seat, until: self.night + 1 });
                    let public = self.groups[gi].public;
                    self.push(public, vec![seat], Event::Smoke { group: id, seat, node });
                }
                self.go_home(gi);
            }
        }
    }

    // --- Jamming, air defense, detection ----------------------------------------

    fn update_jamming(&mut self) {
        let w = self.w;
        for b in self.blinded.iter_mut() {
            *b = false;
        }
        for g in &self.groups {
            if g.phase != Phase::OnStation {
                continue;
            }
            if let GKind::JamZone(z) = g.kind {
                let jammer_alive = g.members.iter().any(|&i| self.sq_alive(i) && w.unit(self.st.squadrons[i].kind).class == crate::content::CombatClass::Jammer);
                if !jammer_alive {
                    continue;
                }
                for n in &w.nodes {
                    if n.zone == z && matches!(n.kind, NodeKind::AirDefense | NodeKind::Radar) && self.st.nodes[n.idx as usize].owner != Some(g.seat) {
                        self.blinded[n.idx as usize] = true;
                    }
                }
            }
        }
    }

    fn group_jammed(&self, gi: usize) -> bool {
        self.groups[gi].members.iter().any(|&i| {
            self.sq_alive(i) && self.w.unit(self.st.squadrons[i].kind).class == crate::content::CombatClass::Jammer
        })
    }

    fn group_stealthy(&self, gi: usize) -> bool {
        let g = &self.groups[gi];
        let alive = self.alive_members(gi);
        !alive.is_empty() && alive.iter().all(|&i| self.sq_stealth(i, g))
    }

    /// Does `seat` see group `gi` on radar right now?
    fn on_radar(&self, seat: u8, gi: usize) -> bool {
        if self.group_stealthy(gi) {
            return false;
        }
        let p = to_map(self.groups[gi].pos);
        let jammed = self.group_jammed(gi);
        self.radar[seat as usize].iter().any(|&(c, r)| {
            let r = if jammed { r / 2 } else { r };
            dist2(c, p) <= (r as i64) * (r as i64)
        }) || vision::always_visible_zone(self.w, &self.st, self.w.zone_at(p))
    }

    fn friendly(&self, a: &Group, b: &Group) -> bool {
        a.seat == b.seat || a.guest_of == Some(b.seat) || b.guest_of == Some(a.seat)
    }

    fn air_defense(&mut self) {
        let w = self.w;
        let r = w.rules();
        for n in &w.nodes {
            if n.kind != NodeKind::AirDefense {
                continue;
            }
            let ns = self.st.nodes[n.idx as usize].clone();
            let Some(owner) = ns.owner else { continue };
            if ns.health == 0 || self.blinded[n.idx as usize] {
                continue;
            }
            let gb = self.faction(owner) == FactionId::GrainBelt;
            let range = if gb { r.air_defense.grainbelt_range } else { r.air_defense.range };
            for gi in 0..self.groups.len() {
                let g = &self.groups[gi];
                if !airborne(g) || g.seat == owner || g.guest_of == Some(owner) {
                    continue;
                }
                let inside = dist2(to_map(g.pos), n.pos) <= (range as i64) * (range as i64);
                let already = g.ad_in.contains(&n.idx);
                if !inside {
                    if already {
                        self.groups[gi].ad_in.retain(|&x| x != n.idx);
                    }
                    continue;
                }
                if already {
                    continue;
                }
                self.groups[gi].ad_in.push(n.idx);
                if self.group_jammed(gi) {
                    continue;
                }
                let targets: Vec<usize> =
                    self.alive_members(gi).into_iter().filter(|&i| !self.sq_stealth(i, &self.groups[gi])).collect();
                if targets.is_empty() {
                    continue;
                }
                let mut shots_n = if ns.health == 2 { r.air_defense.shots_full } else { r.air_defense.shots_half };
                if gb {
                    shots_n += r.air_defense.grainbelt_shots_bonus;
                }
                let mut shots = Vec::new();
                let mut hits: Vec<usize> = Vec::new();
                for _ in 0..shots_n {
                    let live: Vec<usize> = targets.iter().copied().filter(|&i| self.st.squadrons[i].strength > 0).collect();
                    let weights: Vec<i32> = live.iter().map(|&i| self.st.squadrons[i].strength as i32).collect();
                    let Some(k) = self.rng.weighted(&weights) else { break };
                    let t = live[k];
                    let ts = &self.st.squadrons[t];
                    let tu = w.unit(ts.kind);
                    let mut p = r.air_defense.hit[tu.class.index()] - tu.ad_evasion;
                    if self.is_patched_drone(t) {
                        if let Some(PatchEffect::Evasion(d)) = self.patch_effect() {
                            p -= d;
                        }
                    }
                    let hit = self.rng.chance(p.clamp(r.combat.min_hit, r.combat.max_hit));
                    shots.push(Shot { from: 0, to: ts.id, hit });
                    if hit {
                        hits.push(t);
                    }
                }
                let seat = self.groups[gi].seat;
                let gid = self.groups[gi].id;
                let p = to_map(self.groups[gi].pos);
                self.groups[gi].public = true;
                self.public(Event::Flak { site: n.idx, site_seat: owner, group: gid, seat, shots, x: p.0, y: p.1 });
                for t in hits {
                    self.apply_hit(t, Some(owner), None, Some(n.idx), None);
                }
                self.after_damage(gi);
            }
        }
    }

    // --- Air combat ---------------------------------------------------------------

    fn is_hunter(&self, gi: usize) -> bool {
        let g = &self.groups[gi];
        if g.phase != Phase::OnStation {
            return false;
        }
        if !matches!(g.kind, GKind::Guard(_) | GKind::Sweep(_)) {
            return false;
        }
        self.alive_members(gi).iter().any(|&i| self.w.unit(self.st.squadrons[i].kind).air.iter().any(|&a| a > 0))
    }

    fn engagements(&mut self) {
        let w = self.w;
        let r = w.rules();
        let t = self.tick;
        for hi in 0..self.groups.len() {
            if !self.is_hunter(hi) {
                continue;
            }
            let mut best: Option<(i64, usize)> = None;
            for gi in 0..self.groups.len() {
                if gi == hi || !airborne(&self.groups[gi]) {
                    continue;
                }
                let (h, g) = (&self.groups[hi], &self.groups[gi]);
                if self.friendly(h, g) {
                    continue;
                }
                if h.cooldown.iter().any(|&(o, until)| o == g.id && until > t) {
                    continue;
                }
                if self.alive_members(gi).is_empty() {
                    continue;
                }
                // Guards don't pick fights with other guards next door, only over the same place.
                if matches!(h.kind, GKind::Guard(_))
                    && g.phase == Phase::OnStation
                    && matches!(g.kind, GKind::Guard(_) | GKind::Watch(_))
                    && (h.guard_node.is_none() || h.guard_node != g.guard_node)
                {
                    continue;
                }
                let gp = to_map(g.pos);
                let hp = to_map(h.pos);
                let anchor = match h.kind {
                    GKind::Guard(p) => p,
                    _ => hp,
                };
                let stealthy = self.group_stealthy(gi);
                let radar = !stealthy && self.on_radar(h.seat, gi);
                let reach = if stealthy {
                    r.radar.stealth_visual
                } else if radar {
                    match h.kind {
                        GKind::Guard(_) => h.guard_radius,
                        _ => r.radar.sweep,
                    }
                } else {
                    match h.kind {
                        GKind::Guard(_) => h.guard_radius.min(r.radar.guard_visual),
                        _ => r.radar.visual,
                    }
                };
                let d = if stealthy { dist2(gp, hp) } else { dist2(gp, anchor) };
                if d <= (reach as i64) * (reach as i64) && best.map(|b| d < b.0).unwrap_or(true) {
                    best = Some((d, gi));
                }
            }
            if let Some((_, gi)) = best {
                self.fight(hi, gi);
            }
        }
    }

    fn first_shot_side(&self, a: &[usize], b: &[usize], a_gi: usize, b_gi: usize, at: P) -> Option<u8> {
        let w = self.w;
        let has_edge = |side: &[usize], gi: usize| -> bool {
            if side.is_empty() {
                return false;
            }
            let seat = self.st.squadrons[side[0]].seat;
            // SWITCHBACK: altitude advantage.
            if side.iter().any(|&i| self.is_ace_of(i, FactionId::FrontRange)) {
                return true;
            }
            // Oh My Heck: Utah's guards always get the first shot.
            let g = &self.groups[gi];
            if self.faction(seat) == FactionId::Beehive
                && matches!(g.kind, GKind::Guard(_))
                && !rules::effect_active(&self.st, seat, EffectKind::NoFirstShot, self.night)
            {
                return true;
            }
            // Watchtower: nearby friendly fighters get the first shot.
            self.groups.iter().any(|og| {
                og.seat == seat
                    && airborne(og)
                    && og.members.iter().any(|&i| {
                        self.sq_alive(i) && {
                            let rr = w.unit(self.st.squadrons[i].kind).radar;
                            rr > 0 && dist2(to_map(og.pos), at) <= (rr as i64) * (rr as i64)
                        }
                    })
            })
        };
        let ea = has_edge(a, a_gi);
        let eb = has_edge(b, b_gi);
        match (ea, eb) {
            (true, false) => Some(self.st.squadrons[a[0]].seat),
            (false, true) => Some(self.st.squadrons[b[0]].seat),
            _ => None,
        }
    }

    fn hit_chance(&self, shooter: usize, target: usize, at: P) -> i32 {
        let w = self.w;
        let r = w.rules();
        let s = &self.st.squadrons[shooter];
        let tsq = &self.st.squadrons[target];
        let tu = w.unit(tsq.kind);
        let mut p = w.unit(s.kind).air[tu.class.index()];
        if p <= 0 {
            return 0;
        }
        p += s.stars as i32 * r.combat.vet_bonus;
        if s.ace {
            p += r.combat.ace_bonus;
            // BRISKET: "Y'all are a long way from home."
            if self.faction(s.seat) == FactionId::Texas {
                let their_cap = w.nodes[w.capitals[self.faction(tsq.seat).index()] as usize].pos;
                if dist(at, their_cap) >= r.factions.brisket_range {
                    p += r.factions.brisket_bonus;
                }
            }
        }
        if self.is_patched_drone(target) {
            if let Some(PatchEffect::Evasion(d)) = self.patch_effect() {
                p -= d;
            }
        }
        p.clamp(r.combat.min_hit, r.combat.max_hit)
    }

    /// One side fires at the other. Returns the shot log and the hits.
    fn volley(&mut self, shooters: &[usize], targets: &[usize], escorts: &[usize], at: P) -> (Vec<Shot>, Vec<(usize, usize)>) {
        let w = self.w;
        let r = w.rules();
        let mut planes: Vec<usize> = Vec::new();
        for &s in shooters {
            if !self.sq_alive(s) {
                continue;
            }
            if w.unit(self.st.squadrons[s].kind).air.iter().all(|&a| a <= 0) {
                continue;
            }
            for _ in 0..self.st.squadrons[s].strength {
                planes.push(s);
            }
        }
        // Bigger groups win, with diminishing returns (Lanchester, gently).
        let cap = r.combat.shooter_cap as usize;
        let planes: Vec<usize> =
            planes.iter().enumerate().filter(|(k, _)| *k < cap || (k - cap) % 2 == 0).map(|(_, &s)| s).collect();
        let escort_live: Vec<usize> = escorts.iter().copied().filter(|&i| self.sq_alive(i)).collect();
        let screen = (r.combat.escort_screen * escort_live.len() as i32).min(r.combat.escort_screen_max);
        let mut shots = Vec::new();
        let mut hits = Vec::new();
        for s in planes {
            let live: Vec<usize> = targets.iter().copied().filter(|&i| self.sq_alive(i)).collect();
            if live.is_empty() {
                break;
            }
            let pool: Vec<usize> = if !escort_live.is_empty() && self.rng.chance(screen) {
                escort_live.iter().copied().filter(|&i| self.sq_alive(i)).collect()
            } else {
                live.clone()
            };
            let pool = if pool.is_empty() { live } else { pool };
            let weights: Vec<i32> = pool
                .iter()
                .map(|&i| {
                    let t = &self.st.squadrons[i];
                    let wgt = r.combat.target_weight[w.unit(t.kind).class.index()];
                    if self.hit_chance(s, i, at) > 0 {
                        wgt * t.strength as i32
                    } else {
                        0
                    }
                })
                .collect();
            let Some(k) = self.rng.weighted(&weights) else { continue };
            let t = pool[k];
            let p = self.hit_chance(s, t, at);
            let hit = self.rng.chance(p);
            shots.push(Shot { from: self.st.squadrons[s].id, to: self.st.squadrons[t].id, hit });
            if hit {
                hits.push((s, t));
            }
        }
        (shots, hits)
    }

    fn fight(&mut self, hi: usize, gi: usize) {
        let w = self.w;
        let r = w.rules();
        let at = to_map(self.groups[gi].pos);
        let a = self.alive_members(hi);
        let b = self.alive_members(gi);
        let a_seat = self.groups[hi].seat;
        let b_seat = self.groups[gi].seat;
        let a_snap: Vec<SqSnap> = a.iter().map(|&i| self.sq_snap(i)).collect();
        let b_snap: Vec<SqSnap> = b.iter().map(|&i| self.sq_snap(i)).collect();
        let ace_duel = a.iter().any(|&i| self.st.squadrons[i].ace) && b.iter().any(|&i| self.st.squadrons[i].ace);
        let first = self.first_shot_side(&a, &b, hi, gi, at);
        // Escorts screen the strike; fighters in a strike group are its escorts.
        let b_escorts: Vec<usize> = if matches!(self.groups[gi].kind, GKind::Strike(_) | GKind::Scout(_) | GKind::Rebase(_) | GKind::Smoke(_)) {
            b.iter().copied().filter(|i| !self.groups[gi].payload.contains(i)).filter(|&i| w.unit(self.st.squadrons[i].kind).class == crate::content::CombatClass::Fighter).collect()
        } else {
            Vec::new()
        };
        let mut rounds: Vec<Vec<Shot>> = Vec::new();
        let mut escaped: Vec<u32> = Vec::new();
        let mut a_side = a.clone();
        let mut b_side = b.clone();
        let total_rounds = r.combat.rounds + if first.is_some() { 1 } else { 0 };
        for round in 0..total_rounds {
            // Scouts usually outrun trouble.
            for side in [&mut a_side, &mut b_side] {
                let mut keep = Vec::new();
                for &i in side.iter() {
                    if self.sq_alive(i) && w.unit(self.st.squadrons[i].kind).class == crate::content::CombatClass::Scout && self.rng.chance(r.combat.scout_escape) {
                        escaped.push(self.st.squadrons[i].id);
                    } else {
                        keep.push(i);
                    }
                }
                *side = keep;
            }
            let free = first.is_some() && round == 0;
            let a_fires = !free || first == Some(a_seat);
            let b_fires = !free || first == Some(b_seat);
            let mut shots = Vec::new();
            let mut all_hits: Vec<(usize, usize, u8)> = Vec::new();
            if a_fires {
                let (s, h) = self.volley(&a_side, &b_side, &b_escorts, at);
                shots.extend(s);
                all_hits.extend(h.into_iter().map(|(x, y)| (x, y, a_seat)));
            }
            if b_fires {
                let (s, h) = self.volley(&b_side, &a_side, &[], at);
                shots.extend(s);
                all_hits.extend(h.into_iter().map(|(x, y)| (x, y, b_seat)));
            }
            rounds.push(shots);
            for (s, t, seat) in all_hits {
                let by_sq = self.st.squadrons[s].id;
                let shooter_group = if seat == a_seat { hi } else { gi };
                self.apply_hit(t, Some(seat), Some(by_sq), None, Some(shooter_group));
            }
            let a_alive = a_side.iter().any(|&i| self.sq_alive(i));
            let b_alive = b_side.iter().any(|&i| self.sq_alive(i));
            if !a_alive || !b_alive {
                break;
            }
        }
        let after: Vec<SqSnap> = a.iter().chain(b.iter()).map(|&i| self.sq_snap(i)).collect();
        let id = self.next_engage;
        self.next_engage += 1;
        let (hid, gid) = (self.groups[hi].id, self.groups[gi].id);
        self.public(Event::Engage {
            id,
            x: at.0,
            y: at.1,
            a_seat,
            b_seat,
            a_group: hid,
            b_group: gid,
            a: a_snap,
            b: b_snap,
            first_shot: first,
            ace_duel,
            rounds,
            escaped,
            after,
        });
        // Veterancy: survive two engagements, earn a star.
        for &i in a.iter().chain(b.iter()) {
            if self.sq_alive(i) {
                let s = &mut self.st.squadrons[i];
                s.fights += 1;
                if s.fights as i32 >= r.combat.fights_per_star {
                    s.fights = 0;
                    s.stars = (s.stars + 1).min(r.combat.max_stars as u8);
                }
            }
        }
        let t = self.tick;
        let cd = r.combat.cooldown;
        for (x, y) in [(hi, gid), (gi, hid)] {
            let g = &mut self.groups[x];
            g.public = true;
            g.engaged = true;
            g.cooldown.push((y, t + cd));
        }
        self.after_damage(hi);
        self.after_damage(gi);
    }

    /// Strikes abort when they've lost half their payload; others go home at half strength.
    fn after_damage(&mut self, gi: usize) {
        let g = &self.groups[gi];
        if !airborne(g) || g.phase == Phase::Returning {
            return;
        }
        let alive = self.alive_members(gi);
        if alive.is_empty() {
            self.groups[gi].phase = Phase::Done;
            return;
        }
        let abort = match g.kind {
            GKind::Strike(_) => {
                let payload: i32 = g.payload.iter().filter(|&&i| self.sq_alive(i)).map(|&i| self.st.squadrons[i].strength as i32).sum();
                payload * 2 <= g.launch_payload
            }
            _ => {
                let total: i32 = alive.iter().map(|&i| self.st.squadrons[i].strength as i32).sum();
                total * 2 <= g.launch_strength && g.launch_strength > 1
            }
        };
        // LIME JELLO wiggles but doesn't break: never aborts.
        let jello = alive.iter().any(|&i| self.is_ace_of(i, FactionId::Beehive));
        if abort && !jello {
            self.groups[gi].aborted = true;
            let (id, seat, public) = (self.groups[gi].id, self.groups[gi].seat, self.groups[gi].public);
            self.push(public, vec![seat], Event::Abort { group: id, seat });
            self.go_home(gi);
        }
    }

    fn apply_hit(&mut self, t: usize, by_seat: Option<u8>, by_sq: Option<u32>, by_site: Option<u16>, shooter_group: Option<usize>) {
        if !self.sq_alive(t) {
            return;
        }
        let w = self.w;
        let armor = w.unit(self.st.squadrons[t].kind).armor;
        if armor > 0 && self.rng.chance(armor) {
            return;
        }
        // JACKPOT: the first hit each night just... misses.
        if self.is_ace_of(t, FactionId::Dreamland) && !self.rt[t].lucky {
            self.rt[t].lucky = true;
            return;
        }
        self.st.squadrons[t].strength -= 1;
        let seat = self.st.squadrons[t].seat;
        self.st.seats[seat as usize].planes_lost += 1;
        if self.st.squadrons[t].strength > 0 {
            return;
        }
        if self.is_ace_of(t, FactionId::Beehive) && !self.rt[t].jiggled {
            self.rt[t].jiggled = true;
            self.st.squadrons[t].strength = 1;
            let id = self.st.squadrons[t].id;
            self.public(Event::Jiggle { sq: id, seat });
            return;
        }
        self.splash(t, by_seat, by_sq, by_site, false, shooter_group);
    }

    fn splash(&mut self, t: usize, by_seat: Option<u8>, by_sq: Option<u32>, by_site: Option<u16>, ground: bool, shooter_group: Option<usize>) {
        let w = self.w;
        let sc = &w.rules().scoring;
        self.rt[t].lost = true;
        let s = self.st.squadrons[t].clone();
        let pos = match self.rt[t].group {
            Some(gi) if !ground => to_map(self.groups[gi].pos),
            _ => w.nodes[s.base as usize].pos,
        };
        self.public(Event::Splash {
            sq: s.id,
            seat: s.seat,
            kind: s.kind,
            ace: s.ace,
            by_seat,
            by_sq,
            by_site,
            ground,
            x: pos.0,
            y: pos.1,
        });
        let Some(by) = by_seat else { return };
        if by == s.seat {
            return;
        }
        let mut pts = if s.ace { sc.ace } else { sc.squadron };
        // Ace kills count double.
        if let Some(bid) = by_sq {
            if self.st.squadrons.iter().any(|x| x.id == bid && x.ace) {
                pts *= 2;
            }
        }
        self.st.seats[by as usize].kills += 1;
        self.add_score(by, pts, if s.ace { ScoreReason::Ace } else if ground { ScoreReason::Ground } else { ScoreReason::Splash }, None, Some(s.seat));
        self.note_hit(by, s.seat, None);
        // Service Project: a thank-you casserole for defending the neighbors.
        if let Some(gi) = shooter_group {
            let g = &self.groups[gi];
            if self.faction(g.seat) == FactionId::Beehive && g.guest_of.is_some() && g.seat == by {
                self.casseroles.push((by, g.guest_of));
                self.add_score(by, sc.casserole_br, ScoreReason::Casserole, None, None);
            }
        }
    }

    fn add_score(&mut self, seat: u8, pts: i32, reason: ScoreReason, node: Option<u16>, victim: Option<u8>) {
        if pts == 0 {
            return;
        }
        self.st.seats[seat as usize].score += pts;
        if let Some(v) = victim {
            if v != seat {
                self.st.seats[v as usize].pride_lost += pts;
            }
        }
        self.public(Event::Score { seat, delta: pts, reason, node, victim });
    }

    /// Track grudges: Texas remembers, Utah keeps a list.
    fn note_hit(&mut self, by: u8, victim: u8, node: Option<u16>) {
        let _ = node;
        if self.faction(victim) == FactionId::Beehive && !self.st.beehive_grudge.contains(&by) {
            self.st.beehive_grudge.push(by);
        }
    }

    // --- Strikes -----------------------------------------------------------------

    fn strike_accuracy(&self, i: usize, striker: u8, node: u16) -> i32 {
        let w = self.w;
        let r = w.rules();
        let s = &self.st.squadrons[i];
        let u = w.unit(s.kind);
        let mut acc = u.strike;
        if s.ace && self.faction(s.seat) == FactionId::Pacific {
            acc = acc.max(r.strike.pivot_strike); // PIVOT is multirole.
        }
        if acc <= 0 {
            return 0;
        }
        acc += s.stars as i32 * r.combat.vet_bonus / 2;
        if self.is_patched_drone(i) {
            if let Some(PatchEffect::Accuracy(d)) = self.patch_effect() {
                acc += d;
            }
            let swarms = self.drone_targets.iter().find(|d| d.0 == node).map(|d| d.1).unwrap_or(1);
            acc += (swarms - 1).max(0) * r.strike.swarm_bonus;
        }
        let n = &w.nodes[node as usize];
        // Eternal Overcast over Cascadia.
        if let Some(cas) = rules::seat_of(&self.st, FactionId::Cascadia) {
            if striker != cas && w.zones[n.zone as usize].region.faction() == Some(FactionId::Cascadia) {
                acc -= r.strike.overcast_penalty;
            }
        }
        if self.st.smoke.iter().any(|sm| sm.node == node && sm.seat != striker && sm.until >= self.night) {
            acc -= r.strike.smoke_penalty;
        }
        acc.clamp(50, 950)
    }

    fn strike(&mut self, gi: usize, node: u16) {
        let w = self.w;
        let r = w.rules();
        let seat = self.groups[gi].seat;
        let attackers: Vec<usize> = self.alive_members(gi).into_iter().filter(|i| self.groups[gi].payload.contains(i)).collect();
        if rules::attack_check(w, &self.st, seat, node).is_err() {
            return; // captured by us mid-night, or otherwise no longer a target
        }
        let mut hits = 0;
        for &i in &attackers {
            let acc = self.strike_accuracy(i, seat, node);
            let mult = w.unit(self.st.squadrons[i].kind).strike_mult;
            for _ in 0..self.st.squadrons[i].strength {
                if self.rng.chance(acc) {
                    hits += mult;
                }
            }
        }
        let n = &w.nodes[node as usize];
        if n.kind == NodeKind::Fuel && n.home == Some(FactionId::Texas) {
            hits *= r.strike.texas_refinery_mult;
        }
        // Prototype reveals itself when it attacks.
        self.groups[gi].revealed = true;
        self.groups[gi].public = true;
        let kinds = attackers.iter().map(|&i| self.st.squadrons[i].kind).collect();
        let steps = hits / r.strike.hits_per_step;
        let gid = self.groups[gi].id;
        self.apply_steps(seat, node, steps, hits, ScoreReason::Strike, Some((gid, kinds)));
    }

    /// Knock a node down `steps` and handle everything that follows.
    fn apply_steps(&mut self, seat: u8, node: u16, steps: i32, hits: i32, reason: ScoreReason, strike: Option<(u32, Vec<UnitKind>)>) {
        let w = self.w;
        let n = &w.nodes[node as usize];
        let victim = self.st.nodes[node as usize].owner;
        let jewel = n.kind == NodeKind::CrownJewel;
        let cheyenne = jewel && n.home == Some(FactionId::FrontRange);
        let health = self.st.nodes[node as usize].health as i32;
        let steps = steps.clamp(0, if cheyenne { 1 } else { health });
        if steps > 0 {
            if !cheyenne {
                self.st.nodes[node as usize].health = (health - steps) as u8;
            }
            self.st.nodes[node as usize].hit_night = self.night;
            self.hit_tonight[node as usize] = true;
        }
        let health = self.st.nodes[node as usize].health;
        match (reason, strike) {
            (ScoreReason::Strike, Some((group, kinds))) => {
                self.public(Event::Strike { group, seat, node, victim, kinds, hits, steps, health, jewel });
            }
            (ScoreReason::MassDriver, _) => self.public(Event::MassDriver { seat, node, victim, steps, health }),
            _ => self.public(Event::Revenge { seat, node, victim, steps, health }),
        }
        if steps <= 0 {
            return;
        }
        // Parked squadrons get caught on the ground.
        if matches!(n.kind, NodeKind::Airbase | NodeKind::Boneyard) {
            for i in 0..self.st.squadrons.len() {
                let s = &self.st.squadrons[i];
                if s.base != node || !self.sq_alive(i) || s.down_until != 0 {
                    continue;
                }
                let airborne_now = self.rt[i].group.map(|g| airborne(&self.groups[g])).unwrap_or(false);
                if airborne_now {
                    continue;
                }
                let loss = (steps as u8).min(self.st.squadrons[i].strength);
                self.st.squadrons[i].strength -= loss;
                let owner = self.st.squadrons[i].seat;
                self.st.seats[owner as usize].planes_lost += loss as i32;
                if self.st.squadrons[i].strength == 0 {
                    self.splash(i, Some(seat), None, None, true, None);
                }
            }
        }
        let Some(victim) = victim else { return };
        if victim == seat {
            return;
        }
        self.st.seats[seat as usize].steps_dealt += steps;
        self.st.seats[victim as usize].steps_taken += steps;
        let sc = w.rules().scoring.clone();
        let mut pts = if jewel {
            if n.home == Some(FactionId::GrainBelt) {
                sc.twine
            } else {
                sc.jewel
            }
        } else {
            let per = match n.kind {
                NodeKind::City | NodeKind::Factory => sc.city_step,
                _ => sc.other_step,
            };
            per * steps
        };
        if self.night >= w.rules().nights {
            pts *= sc.night8_mult;
        }
        if self.st.scrappy.contains(&seat) {
            pts = (pts * (100 + sc.scrappy_pct) + 99) / 100;
        }
        let reason = if jewel && reason == ScoreReason::Strike { ScoreReason::Jewel } else { reason };
        self.add_score(seat, pts, reason, Some(node), Some(victim));
        self.note_hit(seat, victim, Some(node));
        if self.faction(victim) == FactionId::Texas && !self.hit_texas.contains(&seat) {
            self.hit_texas.push(seat);
        }
        if let Some(&(_, bee)) = self.guarded_by_beehive.iter().find(|(gn, _)| *gn == node) {
            if !self.st.beehive_grudge.contains(&seat) && seat != bee {
                self.st.beehive_grudge.push(seat);
            }
        }
        if jewel {
            self.jewel_effect(seat, victim);
        }
        // Dreamland's bet.
        for b in 0..self.bets.len() {
            if let Some((bet, won)) = self.bets[b].clone() {
                if won || bet.on != seat {
                    continue;
                }
                let node_match = bet.node == Some(node);
                let victim_match = bet.victim == Some(victim) && bet.node.is_none();
                if node_match || victim_match {
                    let fr = &w.rules().factions;
                    let payout = bet.stake * if node_match { fr.bet_node_mult } else { fr.bet_faction_mult };
                    self.bets[b] = Some((bet.clone(), true));
                    self.public(Event::BetWin { seat: b as u8, on: bet.on, node: bet.node, victim: bet.victim, stake: bet.stake, payout });
                }
            }
        }
    }

    fn jewel_effect(&mut self, striker: u8, victim: u8) {
        let night = self.night;
        let f = self.faction(victim);
        let (kind, nights) = match f {
            FactionId::Pacific => (Some(EffectKind::NoDrones), 1),
            FactionId::Cascadia => (Some(EffectKind::FactoriesHalf), 2),
            FactionId::FrontRange => (Some(EffectKind::RadarSealed), 1),
            FactionId::Texas => (Some(EffectKind::FuelHalf), 2),
            FactionId::Dreamland => (Some(EffectKind::StripDark), 1),
            FactionId::Beehive => (Some(EffectKind::NoFirstShot), 2),
            FactionId::GrainBelt => {
                self.st.revenge_against = Some(striker);
                (None, 1)
            }
        };
        if let Some(k) = kind {
            if self.st.effects.iter().any(|e| e.seat == victim && e.kind == k && e.from == night + 1) {
                return;
            }
            self.st.effects.push(Effect { seat: victim, kind: k, from: night + 1, until: night + nights });
            self.public(Event::JewelEffect { seat: victim, kind: k, nights });
        }
    }

    fn mass_driver(&mut self) {
        let w = self.w;
        let Some(holder) = self.massdriver_holder else { return };
        if rules::stage(w, self.night) < 2 {
            return;
        }
        let Some(node) = self.orders[holder as usize].mass_driver else { return };
        if (node as usize) >= w.nodes.len() || rules::attack_check(w, &self.st, holder, node).is_err() {
            return;
        }
        let steps = w.rules().factions.mass_driver_steps;
        self.apply_steps(holder, node, steps, steps * 2, ScoreReason::MassDriver, None);
    }

    fn revenge(&mut self) {
        let w = self.w;
        let Some(gb) = rules::seat_of(&self.st, FactionId::GrainBelt) else { return };
        let Some(target_seat) = self.st.revenge_against else { return };
        let Some(node) = self.orders[gb as usize].revenge else { return };
        if (node as usize) >= w.nodes.len() || self.st.nodes[node as usize].owner != Some(target_seat) {
            return;
        }
        if rules::attack_check(w, &self.st, gb, node).is_err() {
            return;
        }
        let steps = w.rules().factions.revenge_steps;
        self.apply_steps(gb, node, steps, steps * 2, ScoreReason::Revenge, None);
    }

    /// PIVOT may switch role mid-mission once per night.
    fn pivot(&mut self) {
        let w = self.w;
        let range = w.rules().factions.pivot_range;
        for gi in 0..self.groups.len() {
            let g = &self.groups[gi];
            if g.phase != Phase::OnStation || g.engaged || g.pivoted || !matches!(g.kind, GKind::Guard(_) | GKind::Sweep(_)) {
                continue;
            }
            if !self.alive_members(gi).iter().any(|&i| self.is_ace_of(i, FactionId::Pacific)) {
                continue;
            }
            let here = to_map(g.pos);
            let seat = g.seat;
            let mut best: Option<(i32, u16)> = None;
            for n in &w.nodes {
                if !n.kind.is_military() || rules::attack_check(w, &self.st, seat, n.idx).is_err() {
                    continue;
                }
                if self.st.nodes[n.idx as usize].owner.is_none() || self.st.nodes[n.idx as usize].health == 0 {
                    continue;
                }
                let d = dist(here, n.pos);
                if d <= range && best.map(|b| d < b.0).unwrap_or(true) {
                    best = Some((d, n.idx));
                }
            }
            if let Some((_, node)) = best {
                let g = &mut self.groups[gi];
                g.pivoted = true;
                g.kind = GKind::Strike(node);
                g.tag = MissionTag::Attack;
                g.payload = g.members.clone();
                g.launch_payload = g.members.iter().map(|&i| self.st.squadrons[i].strength as i32).sum();
                g.path = vec![g.pos, to_fp(w.nodes[node as usize].pos)];
                g.leg = 1;
                g.phase = Phase::Outbound;
                g.orbit.clear();
                let id = g.id;
                self.public(Event::Pivot { group: id, seat, node });
            }
        }
    }

    // --- End of night and dawn ---------------------------------------------------

    fn night_end(&mut self) {
        // Everyone still airborne makes it home before dawn.
        let t = self.tick;
        for g in &mut self.groups {
            if g.phase != Phase::Done && g.phase != Phase::Waiting {
                let p = to_map(g.pos);
                g.track.push((t, p.0, p.1));
            }
            if g.phase != Phase::Waiting {
                g.phase = Phase::Done;
            }
        }
        // Publish flight records; tracks of squadrons that never met anyone stay private.
        for gi in 0..self.groups.len() {
            let g = &self.groups[gi];
            if g.launch_ev == usize::MAX {
                continue;
            }
            let public = g.public;
            if public {
                self.ev[g.launch_ev].public = true;
            }
            let e = Event::Track { group: g.id, seat: g.seat, pts: g.track.clone() };
            let seat = g.seat;
            self.push(public, vec![seat], e);
        }
    }

    fn dawn(&mut self) {
        let w = self.w;
        let night = self.night;
        let next = night + 1;
        let r = w.rules().clone();
        // Casualties: aces and the Longhorn come back later; everything else is gone.
        let mut keep = Vec::new();
        for (i, mut s) in std::mem::take(&mut self.st.squadrons).into_iter().enumerate() {
            let rt = &self.rt[i];
            if rt.consumed {
                continue;
            }
            if rt.lost || s.strength == 0 {
                if s.ace {
                    s.down_until = night + r.factions.ace_return as u8;
                    s.strength = 0;
                    s.order = Mission::Rest;
                    keep.push(s);
                } else if s.kind == UnitKind::Longhorn {
                    s.down_until = night + r.factions.longhorn_rebuild as u8;
                    s.strength = 0;
                    s.order = Mission::Rest;
                    keep.push(s);
                }
                continue;
            }
            // Squadron repairs at base: +1 a night, +2 resting.
            let base_ok = self.st.nodes[s.base as usize].health > 0 || w.nodes[s.base as usize].kind == NodeKind::Lake;
            if base_ok && s.down_until == 0 {
                let add = if rt.rested && !rt.grounded { 2 } else { 1 };
                s.strength = (s.strength + add).min(MAX_STRENGTH);
            }
            keep.push(s);
        }
        self.st.squadrons = keep;

        // Captures: guard it and hold the sky at dawn.
        let mut holders: Vec<(u16, Vec<u8>)> = Vec::new();
        for g in &self.groups {
            let Some(node) = g.guard_node else { continue };
            if !g.arrived || g.aborted || !matches!(g.kind, GKind::Guard(_)) {
                continue;
            }
            let alive = g.members.iter().any(|&i| !self.rt[i].lost && !self.rt[i].consumed);
            if !alive {
                continue;
            }
            match holders.iter_mut().find(|h| h.0 == node) {
                Some(h) => {
                    if !h.1.contains(&g.seat) {
                        h.1.push(g.seat)
                    }
                }
                None => holders.push((node, vec![g.seat])),
            }
        }
        holders.sort();
        for (node, seats) in holders {
            if seats.len() != 1 || !rules::capturable(w, &self.st, node) {
                continue;
            }
            let seat = seats[0];
            let prev = self.st.nodes[node as usize].owner;
            if prev != Some(seat) {
                self.st.nodes[node as usize].owner = Some(seat);
                self.public(Event::Capture { node, seat, prev });
            }
        }
        // Neutral objectives held at dawn.
        for n in &w.nodes {
            let obj = rules::objective(w, &self.st, n.idx);
            if obj <= 0 {
                continue;
            }
            if let Some(o) = self.st.nodes[n.idx as usize].owner {
                if rules::capturable(w, &self.st, n.idx) {
                    self.add_score(o, obj, ScoreReason::Objective, Some(n.idx), None);
                }
            }
        }
        // Income for the coming night (jewel effects already scheduled).
        for seat in 0..self.st.seats.len() as u8 {
            let inc = economy::income(w, &self.st, seat, next);
            let s = &mut self.st.seats[seat as usize];
            s.income_supply = inc.supply;
            s.income_fuel = inc.fuel;
            // Income can't push a stockpile past two nights' worth; nothing already banked is lost.
            s.supply = (s.supply + inc.supply).min(economy::cap(w, inc.supply).max(s.supply));
            s.fuel = (s.fuel + inc.fuel).min(economy::cap(w, inc.fuel).max(s.fuel));
            let (sup, fu) = (inc.supply, inc.fuel);
            self.private(seat, Event::Income { seat, supply: sup, fuel: fu });
        }
        // Windfalls land after the cap: bets, the Harvest, casseroles.
        for b in 0..self.bets.len() {
            if let Some((bet, won)) = self.bets[b].clone() {
                let fr = &r.factions;
                if won {
                    let mult = if bet.node.is_some() { fr.bet_node_mult } else { fr.bet_faction_mult };
                    self.st.seats[b].supply += bet.stake * mult;
                } else {
                    self.private(b as u8, Event::BetLost { seat: b as u8, stake: bet.stake });
                }
            }
        }
        if let Some((winner, bid)) = self.harvest {
            self.st.seats[winner as usize].supply += self.st.harvest;
            if let Some(gb) = rules::seat_of(&self.st, FactionId::GrainBelt) {
                self.st.seats[gb as usize].supply += bid;
            }
        }
        for (seat, host) in std::mem::take(&mut self.casseroles) {
            self.st.seats[seat as usize].supply += r.scoring.casserole_supply;
            self.public(Event::Casserole { seat, host, supply: r.scoring.casserole_supply });
        }
        // Colorado sells one radar picture.
        self.intel_sale(next);
        // Free repairs: one step a night, starting the night after a hit.
        for i in 0..self.st.nodes.len() {
            let ns = &mut self.st.nodes[i];
            if ns.health < 2 && !self.hit_tonight[i] && ns.owner.is_some() {
                ns.health += 1;
                let (seat, h) = (ns.owner.unwrap(), ns.health);
                self.private(seat, Event::Repaired { seat, node: i as u16, health: h, paid: false });
            }
        }
        // New squadrons, salvage and returning aces.
        let pend = std::mem::take(&mut self.st.pending);
        for p in pend {
            if p.night > next {
                self.st.pending.push(p);
                continue;
            }
            match setup::place(w, &self.st, p.seat, p.kind) {
                Some(base) => {
                    let id = self.new_squadron(p.seat, p.kind, base, MAX_STRENGTH, p.stars);
                    self.private(p.seat, Event::Built { seat: p.seat, sq: id, kind: p.kind, node: base });
                }
                None => {
                    self.st.seats[p.seat as usize].supply += w.unit(p.kind).cost;
                    self.private(p.seat, Event::BuildFailed { seat: p.seat, kind: p.kind, reason: "No room at any base. Refunded.".into() });
                }
            }
        }
        if night % 2 == 0 {
            if let Some(by) = w.nodes.iter().find(|n| n.kind == NodeKind::Boneyard) {
                if let Some(holder) = self.st.nodes[by.idx as usize].owner {
                    let kind = if (night / 2) % 2 == 1 { UnitKind::Fighter } else { UnitKind::Bomber };
                    let base = if rules::base_occupancy(&self.st, by.idx, None) < rules::base_capacity(w, &self.st, by.idx, kind) {
                        Some(by.idx)
                    } else {
                        setup::place(w, &self.st, holder, kind)
                    };
                    if let Some(base) = base {
                        let id = self.new_squadron(holder, kind, base, r.factions.salvage_strength as u8, 0);
                        self.private(holder, Event::Salvage { seat: holder, sq: id, kind });
                    }
                }
            }
        }
        for i in 0..self.st.squadrons.len() {
            if self.st.squadrons[i].down_until != 0 && self.st.squadrons[i].down_until <= next {
                let seat = self.st.squadrons[i].seat;
                let kind = self.st.squadrons[i].kind;
                let base_ok = {
                    let b = self.st.squadrons[i].base;
                    self.st.nodes[b as usize].owner == Some(seat)
                        && rules::base_occupancy(&self.st, b, None) < rules::base_capacity(w, &self.st, b, kind)
                };
                let base = if base_ok { Some(self.st.squadrons[i].base) } else { setup::place(w, &self.st, seat, kind) };
                if let Some(b) = base {
                    let s = &mut self.st.squadrons[i];
                    s.down_until = 0;
                    s.strength = MAX_STRENGTH;
                    s.base = b;
                    let id = s.id;
                    self.public(Event::AceReturns { seat, sq: id, kind });
                }
            }
        }
        // Grudges for tomorrow. Texas remembers one night; a revenge strike is good for one night.
        let mut hit_texas = std::mem::take(&mut self.hit_texas);
        hit_texas.sort();
        self.st.texas_grudge = hit_texas;
        if !self.twine_hit_tonight() {
            self.st.revenge_against = None;
        }
        // Intel and scouting.
        self.update_intel(next);
        self.st.effects.retain(|e| e.until >= next);
        self.st.smoke.retain(|s| s.until >= next);
        // Scrappy: whoever is last at dawn hits harder tomorrow.
        let scores: Vec<i32> = self.st.seats.iter().map(|s| s.score).collect();
        let min = *scores.iter().min().unwrap_or(&0);
        let all_tied = scores.iter().all(|&s| s == min);
        self.st.scrappy = if all_tied { Vec::new() } else { (0..scores.len() as u8).filter(|&i| scores[i as usize] == min).collect() };
        let mut rng = Rng::for_night(self.st.seed, night, 2);
        setup::roll_dawn_offers(w, &mut self.st, &mut rng);
        self.st.history.push(NightRecord { night, scores: scores.clone() });
        self.st.night = next;
        let (scrappy, patch, harvest) = (self.st.scrappy.clone(), self.st.patch, self.st.harvest);
        self.tick = r.ticks;
        self.public(Event::Dawn { night, scores, scrappy, patch, harvest });
    }

    fn twine_hit_tonight(&self) -> bool {
        rules::is_seated(&self.st, FactionId::GrainBelt) && self.hit_tonight[self.w.jewels[FactionId::GrainBelt.index()] as usize]
    }

    fn new_squadron(&mut self, seat: u8, kind: UnitKind, base: u16, strength: u8, stars: u8) -> u32 {
        let id = self.st.next_sq;
        self.st.next_sq += 1;
        self.st.squadrons.push(Squadron {
            id,
            seat,
            kind,
            strength,
            base,
            stars,
            fights: 0,
            ace: false,
            order: Mission::Rest,
            waypoint: None,
            down_until: 0,
        });
        id
    }

    fn intel_sale(&mut self, next: u8) {
        let Some(co) = rules::seat_of(&self.st, FactionId::FrontRange) else { return };
        let Some(sale) = self.orders[co as usize].intel_sale.clone() else { return };
        if rules::effect_active(&self.st, co, EffectKind::RadarSealed, next) || sale.price < 0 {
            return;
        }
        let mut best: Vec<(i32, u8, u16)> = Vec::new();
        for seat in 0..self.st.seats.len() as u8 {
            if seat == co || sale.only.map(|o| o != seat).unwrap_or(false) {
                continue;
            }
            if let Some(req) = &self.orders[seat as usize].intel_request {
                if req.max_price >= sale.price && self.st.seats[seat as usize].supply >= sale.price && (req.zone as usize) < self.w.zones.len() {
                    if best.is_empty() || req.max_price > best[0].0 {
                        best = vec![(req.max_price, seat, req.zone)];
                    } else if req.max_price == best[0].0 {
                        best.push((req.max_price, seat, req.zone));
                    }
                }
            }
        }
        if best.is_empty() {
            return;
        }
        let (_, buyer, zone) = best[self.rng.below(best.len() as u32) as usize];
        self.st.seats[buyer as usize].supply -= sale.price;
        self.st.seats[co as usize].supply += sale.price;
        self.st.intel[buyer as usize].revealed.push(Reveal { zone, radar_of: Some(co), half: false });
        self.push(false, vec![co, buyer], Event::IntelSold { seller: co, buyer, zone, price: sale.price });
    }

    fn update_intel(&mut self, next: u8) {
        let w = self.w;
        let seats = self.st.seats.len();
        // Scouting reports for the coming planning phase (bought pictures were added already).
        for i in 0..seats {
            self.st.intel[i].revealed.retain(|r| r.radar_of.is_some());
        }
        for (seat, zone) in std::mem::take(&mut self.scouted) {
            let half = w.zones[zone as usize].region.faction() == Some(FactionId::Cascadia)
                && self.faction(seat) != FactionId::Cascadia
                && rules::is_seated(&self.st, FactionId::Cascadia);
            if !self.st.intel[seat as usize].revealed.iter().any(|r| r.zone == zone && r.radar_of.is_none()) {
                self.st.intel[seat as usize].revealed.push(Reveal { zone, radar_of: None, half });
            }
            self.private(seat, Event::ScoutReport { seat, zone, half });
        }
        // Everyone saw tonight's strikes.
        let mut public_hits: Vec<(u16, u8)> = Vec::new();
        let mut seen_flying: Vec<u32> = Vec::new();
        let mut splashed: Vec<u32> = Vec::new();
        for e in &self.ev {
            if !e.public {
                continue;
            }
            match &e.e {
                Event::Strike { node, health, .. } | Event::MassDriver { node, health, .. } | Event::Revenge { node, health, .. } => {
                    public_hits.push((*node, *health))
                }
                Event::Launch { squadrons, .. } => seen_flying.extend(squadrons.iter().map(|s| s.sq)),
                Event::Splash { sq, .. } | Event::Vanished { sq, .. } | Event::LostInStatic { sq, .. } => splashed.push(*sq),
                _ => {}
            }
        }
        // Make the reveal checks see the new night.
        let saved_night = self.st.night;
        self.st.night = next;
        for seat in 0..seats as u8 {
            for &(node, h) in &public_hits {
                self.st.intel[seat as usize].known[node as usize] = h;
            }
            for n in 0..w.nodes.len() as u16 {
                if vision::node_visible(w, &self.st, seat, n) {
                    let h = self.st.nodes[n as usize].health;
                    self.st.intel[seat as usize].known[n as usize] = h;
                }
            }
            let visible: Vec<Ghost> = vision::visible_squadrons(w, &self.st, seat)
                .into_iter()
                .chain(self.st.squadrons.iter().filter(|s| seen_flying.contains(&s.id) && s.seat != seat && s.active()))
                .map(|s| Ghost { sq: s.id, seat: s.seat, kind: s.kind, strength: s.strength, node: s.base, night: next, ace: s.ace })
                .collect();
            let intel = &mut self.st.intel[seat as usize];
            intel.ghosts.retain(|g| !splashed.contains(&g.sq) && !visible.iter().any(|v| v.sq == g.sq));
            intel.ghosts.extend(visible);
            intel.ghosts.retain(|g| next.saturating_sub(g.night) <= 3);
            intel.ghosts.sort_by_key(|g| g.sq);
            intel.ghosts.dedup_by_key(|g| g.sq);
        }
        self.st.night = saved_night;
    }
}

fn airborne(g: &Group) -> bool {
    matches!(g.phase, Phase::Outbound | Phase::OnStation | Phase::Returning)
}

/// A 16-gon patrol orbit, starting at a varied angle. Integer trig table.
fn orbit_points(center: P, radius: i32, start: usize) -> Vec<Fp> {
    const COS: [i32; 16] = [1000, 924, 707, 383, 0, -383, -707, -924, -1000, -924, -707, -383, 0, 383, 707, 924];
    let r = radius.max(10) * FX;
    let c = to_fp(center);
    (0..16)
        .map(|k| {
            let a = (k + start) % 16;
            let s = (a + 12) % 16; // sin(a) = cos(a - 90 deg)
            (c.0 + r * COS[a] / 1000, c.1 + r * COS[s] / 1000)
        })
        .collect()
}

/// Squadrons whose standing orders make sense to carry over into the next night.
pub fn carry_over(world: &World, state: &GameState, seat: u8) -> Orders {
    let mut o = Orders { seat, ..Default::default() };
    for s in state.squadrons.iter().filter(|s| s.seat == seat && s.active()) {
        let m = if rules::mission_check(world, state, s, &s.order).is_ok() { s.order.clone() } else { Mission::Rest };
        o.missions.push(SqOrder { sq: s.id, mission: m, waypoint: s.waypoint });
    }
    o
}

/// Fuel a mission costs: by distance flown, with free local guards and
/// Remember the Alamo. `path` is the outbound route (escorts: their lead's).
pub fn fuel_cost(world: &World, state: &GameState, sq: &Squadron, mission: &Mission, path: &[P]) -> i32 {
    let w = world;
    let one_way = w.unit(sq.kind).one_way || matches!(mission, Mission::Rebase { .. });
    let len = route::path_len(path) * if one_way { 1 } else { 2 };
    let base = w.nodes[sq.base as usize].pos;
    let free = w.rules().economy.free_guard_radius;
    match *mission {
        Mission::Rest => return 0,
        Mission::Guard { node } | Mission::Rebase { node } if dist(base, w.nodes[node as usize].pos) <= free => return 0,
        Mission::GuardZone { zone } if dist(base, w.zones[zone as usize].center) <= free => return 0,
        Mission::Attack { node } if state.seats[sq.seat as usize].faction == FactionId::Texas => {
            // Remember the Alamo: hitting last night's attackers flies free.
            if let Some(o) = state.nodes[node as usize].owner {
                if state.texas_grudge.contains(&o) {
                    return 0;
                }
            }
        }
        _ => {}
    }
    let mut cost = (len * w.unit(sq.kind).fuel_rate + 999) / 1000;
    if sq.kind == UnitKind::Drone && state.seats[sq.seat as usize].faction == FactionId::Pacific {
        if let Some(PatchEffect::Fuel(d)) = state.patch.and_then(|i| w.content.patch_notes.get(i as usize)).map(|p| p.effect) {
            cost += d;
        }
    }
    cost.max(1)
}

/// The point a mission flies to.
pub fn mission_point(world: &World, state: &GameState, sq: &Squadron, m: &Mission) -> Option<P> {
    let w = world;
    Some(match *m {
        Mission::Attack { node } | Mission::Guard { node } | Mission::Rebase { node } => w.nodes[node as usize].pos,
        Mission::GuardZone { zone } | Mission::Sweep { zone } | Mission::Scout { zone } | Mission::JamZone { zone } => w.zones[zone as usize].center,
        Mission::Rest => return None,
        Mission::Escort { sq: t } | Mission::JamEscort { sq: t } => {
            let lead = state.squadrons.iter().find(|s| s.id == t)?;
            if lead.id == sq.id {
                return None;
            }
            return mission_point(world, state, lead, &lead.order);
        }
    })
}
