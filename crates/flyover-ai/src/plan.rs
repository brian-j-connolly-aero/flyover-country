//! Suggest Plan and the bots. One button fills in a complete, decent plan in
//! well under half a second; bots use the same planner. Bots pick targets by
//! value, never by who's winning.

use crate::analysis;
use crate::risk;
use flyover_core::content::{dist, CombatClass, FactionId, NodeKind, OrderKind, UnitKind, World, P};
use flyover_core::rng::mix;
use flyover_core::route::Planner;
use flyover_core::rules;
use flyover_core::sim;
use flyover_core::state::*;
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
    f: FactionId,
    level: Level,
    salt: u64,
    health: Vec<u8>,
    planner: Planner<'a>,
    orders: Orders,
}

impl<'a> Ctx<'a> {
    fn roll(&mut self, n: u64) -> u64 {
        self.salt = mix(self.salt, 0x5151);
        if n == 0 {
            0
        } else {
            self.salt % n
        }
    }

    fn set(&mut self, sq: u32, m: Mission) {
        self.orders.missions.retain(|o| o.sq != sq);
        self.orders.missions.push(SqOrder { sq, mission: m, waypoint: None });
    }

    fn mission(&self, sq: u32) -> Mission {
        self.orders.missions.iter().find(|o| o.sq == sq).map(|o| o.mission.clone()).unwrap_or(Mission::Rest)
    }

    fn free(&self, sq: u32) -> bool {
        self.mission(sq) == Mission::Rest
    }

    fn ok(&self, s: &Squadron, m: &Mission) -> bool {
        rules::mission_check(self.w, self.st, s, m).is_ok()
    }

    fn base_pos(&self, s: &Squadron) -> P {
        self.w.nodes[s.base as usize].pos
    }
}

pub fn suggest(world: &World, state: &GameState, seat: u8, level: Level) -> Orders {
    let mut c = Ctx {
        w: world,
        st: state,
        seat,
        f: rules::faction(state, seat),
        level,
        salt: mix(state.seed ^ 0xB07, (state.night as u64) << 8 | seat as u64),
        health: analysis::believed_health(state, seat),
        planner: Planner::for_seat(world, state, seat),
        orders: Orders { seat, ..Default::default() },
    };
    let mine: Vec<Squadron> = state.squadrons.iter().filter(|s| s.seat == seat && s.active()).cloned().collect();
    for s in &mine {
        c.set(s.id, Mission::Rest);
    }
    let stage = rules::stage(world, state.night);

    // 1. Defense: guard what matters most.
    let mut guard_spots: Vec<u16> = Vec::new();
    let jewel = world.jewels[c.f.index()];
    if stage >= 2 && state.nodes[jewel as usize].active {
        guard_spots.push(jewel);
    }
    let ws = analysis::weak_spot(world, state, seat, seat);
    for &n in &ws.nodes {
        if !guard_spots.contains(&n) {
            guard_spots.push(n);
        }
    }
    let capital = world.capitals[c.f.index()];
    if !guard_spots.contains(&capital) {
        guard_spots.push(capital);
    }
    let want_guards = match (c.level, c.f) {
        (Level::Easy, _) => 1,
        (_, FactionId::Beehive) => 2,
        _ if stage == 2 => 2,
        _ => 1,
    };
    let interceptors: Vec<Squadron> = mine
        .iter()
        .filter(|s| matches!(s.kind, UnitKind::Fighter | UnitKind::Gull))
        .cloned()
        .collect();
    let mut guards = 0;
    // Gulls guard first; fighters prefer escort work.
    let mut by_pref = interceptors.clone();
    by_pref.sort_by_key(|s| (s.kind != UnitKind::Gull, s.ace, s.id));
    for spot in guard_spots.iter().copied() {
        if guards >= want_guards {
            break;
        }
        if let Some(s) = by_pref.iter().find(|s| c.free(s.id) && c.ok(s, &Mission::Guard { node: spot })).cloned() {
            c.set(s.id, Mission::Guard { node: spot });
            guards += 1;
        }
    }
    // Crop dusters smoke the jewel (Gloves Off) or the weakest spot.
    for s in mine.iter().filter(|s| s.kind == UnitKind::CropDuster) {
        let spot = if stage == 2 { jewel } else { *ws.nodes.first().unwrap_or(&capital) };
        if c.ok(s, &Mission::Guard { node: spot }) {
            c.set(s.id, Mission::Guard { node: spot });
        }
    }

    // 2. The race for the Big Empty: guard a neutral objective to capture it.
    if state.night <= world.rules().nights - 1 {
        let mut best: Option<(i32, u16)> = None;
        for n in &world.nodes {
            let obj = rules::objective(world, state, n.idx);
            if obj <= 0 || state.nodes[n.idx as usize].owner == Some(seat) || !rules::capturable(world, state, n.idx) {
                continue;
            }
            let d = mine.iter().map(|s| dist(c.base_pos(s), n.pos)).min().unwrap_or(9999);
            if d > 420 {
                continue;
            }
            let score = obj * 100 - d / 3 + (c.roll(20) as i32);
            if best.map(|b| score > b.0).unwrap_or(true) {
                best = Some((score, n.idx));
            }
        }
        if let Some((_, node)) = best {
            let pos = world.nodes[node as usize].pos;
            let mut cands: Vec<Squadron> = interceptors.iter().filter(|s| c.free(s.id)).cloned().collect();
            cands.sort_by_key(|s| (dist(c.base_pos(s), pos), s.id));
            if let Some(s) = cands.first().cloned() {
                if c.ok(&s, &Mission::Guard { node }) {
                    c.set(s.id, Mission::Guard { node });
                }
            }
        }
    }

    // 3. Strikes: bombers and strike uniques at the best value for the risk.
    let targets = analysis::best_targets(world, state, seat, if c.level == Level::Easy { 6 } else { 14 });
    let mut strikers: Vec<Squadron> = mine
        .iter()
        .filter(|s| c.free(s.id) && world.unit(s.kind).strike > 0 && world.unit(s.kind).class != CombatClass::Fighter)
        .cloned()
        .collect();
    strikers.sort_by_key(|s| (-(world.unit(s.kind).strike * world.unit(s.kind).strike_mult), s.id));
    let mut strike_count: Vec<(u16, i32)> = Vec::new();
    let mut leads: Vec<(u32, u16, i32)> = Vec::new(); // (lead sq, target, risk)
    let drone_target = if targets.is_empty() {
        None
    } else {
        // Swarms go together: pick one target for every drone tonight.
        let pick = if c.level == Level::Easy { c.roll(targets.len() as u64) as usize } else { c.roll(2.min(targets.len() as u64)) as usize };
        Some(targets[pick].node)
    };
    for s in &strikers {
        if s.kind == UnitKind::Drone {
            if let Some(t) = drone_target {
                let m = Mission::Attack { node: t };
                if c.ok(s, &m) {
                    c.set(s.id, m);
                    match strike_count.iter_mut().find(|x| x.0 == t) {
                        Some(x) => x.1 += 1,
                        None => strike_count.push((t, 1)),
                    }
                }
            }
            continue;
        }
        let mut best: Option<(i32, u16, i32)> = None;
        for t in &targets {
            let m = Mission::Attack { node: t.node };
            if !c.ok(s, &m) {
                continue;
            }
            let Some(p) = risk::preview_with(world, state, &c.orders, s, &m, None, &c.planner) else { continue };
            if p.in_static {
                continue;
            }
            if p.risk >= 950 {
                continue;
            }
            let already = strike_count.iter().find(|x| x.0 == t.node).map(|x| x.1).unwrap_or(0);
            let h = c.health[t.node as usize] as i32;
            // Stack two squadrons on a healthy target; more is overkill.
            let stack = if already >= h.max(1) { -40 } else if already == 1 { 12 } else { 0 };
            let distance = dist(c.base_pos(s), world.nodes[t.node as usize].pos);
            let jitter = c.roll(if c.level == Level::Easy { 40 } else { 12 }) as i32;
            let score = t.value * 3 - p.risk / 15 - distance / 20 + stack + jitter - p.fuel * 2;
            if best.map(|b| score > b.0).unwrap_or(true) {
                best = Some((score, t.node, p.risk));
            }
        }
        if let Some((_, node, rsk)) = best {
            {
                c.set(s.id, Mission::Attack { node });
                match strike_count.iter_mut().find(|x| x.0 == node) {
                    Some(x) => x.1 += 1,
                    None => strike_count.push((node, 1)),
                }
                if !leads.iter().any(|l| l.1 == node && l.0 != s.id && mine.iter().any(|m| m.id == l.0 && m.base == s.base)) {
                    leads.push((s.id, node, rsk));
                }
            }
        }
    }
    leads.sort_by_key(|l| (-l.2, l.0));

    // 4. Escorts, jammers, the Watchtower.
    if c.level == Level::Normal {
        let mut escorts: Vec<Squadron> = interceptors.iter().filter(|s| c.free(s.id) && s.kind == UnitKind::Fighter).cloned().collect();
        escorts.sort_by_key(|s| (!s.ace, s.id));
        for (lead, _, rsk) in leads.clone() {
            if rsk < 150 {
                continue;
            }
            if let Some(e) = escorts.iter().find(|e| c.free(e.id)).cloned() {
                let m = Mission::Escort { sq: lead };
                if c.ok(&e, &m) {
                    c.set(e.id, m);
                }
            }
        }
        for j in mine.iter().filter(|s| s.kind == UnitKind::Jammer && c.free(s.id)).cloned().collect::<Vec<_>>() {
            // Jam the strike that crosses the most air defense; otherwise blind
            // the radar over the main target so its guards see us late.
            let mut best: Option<(i32, u32)> = None;
            for &(lead, _, _) in &leads {
                let Some(ls) = mine.iter().find(|m| m.id == lead) else { continue };
                let lm = c.mission(lead);
                if let Some(p) = risk::preview_with(world, state, &c.orders, ls, &lm, None, &c.planner) {
                    if p.ad_sites > 0 && best.map(|b| p.ad_sites > b.0).unwrap_or(true) {
                        best = Some((p.ad_sites, lead));
                    }
                }
            }
            if let Some((_, lead)) = best {
                c.set(j.id, Mission::JamEscort { sq: lead });
            } else if let Some(&(_, node, _)) = leads.first() {
                let m = Mission::JamZone { zone: world.nodes[node as usize].zone };
                if c.ok(&j, &m) {
                    c.set(j.id, m);
                }
            }
        }
        for wt in mine.iter().filter(|s| s.kind == UnitKind::Watchtower && c.free(s.id)).cloned().collect::<Vec<_>>() {
            let m = match leads.first() {
                Some(&(lead, _, rsk)) if rsk >= 150 => Mission::Escort { sq: lead },
                _ => Mission::Guard { node: capital },
            };
            if c.ok(&wt, &m) {
                c.set(wt.id, m);
            }
        }
    }
    // Leftover fighters: escort an unescorted strike, sweep ahead of the main
    // strike, or stand guard over a spot nobody's covering yet.
    let mut spare_spot = 0usize;
    for s in interceptors.iter().filter(|s| c.free(s.id)).cloned().collect::<Vec<_>>() {
        let unescorted = leads.iter().find(|l| {
            !c.orders.missions.iter().any(|o| matches!(o.mission, Mission::Escort { sq } if sq == l.0))
        });
        let m = match (c.level, unescorted, leads.first()) {
            (Level::Normal, Some(&(lead, _, _)), _) if s.kind == UnitKind::Fighter => Mission::Escort { sq: lead },
            (Level::Normal, None, Some(&(_, node, _))) if s.kind == UnitKind::Fighter && c.roll(2) == 0 => {
                Mission::Sweep { zone: world.nodes[node as usize].zone }
            }
            _ => {
                let guarded: Vec<u16> = c
                    .orders
                    .missions
                    .iter()
                    .filter_map(|o| if let Mission::Guard { node } = o.mission { Some(node) } else { None })
                    .collect();
                let open: Vec<u16> = guard_spots.iter().copied().filter(|n| !guarded.contains(n)).collect();
                let spot = if open.is_empty() {
                    spare_spot += 1;
                    guard_spots[spare_spot % guard_spots.len()]
                } else {
                    open[0]
                };
                Mission::Guard { node: spot }
            }
        };
        if c.ok(&s, &m) {
            c.set(s.id, m);
        }
    }
    // Floatplanes with nothing to hit guard a lake-side node; fine to rest otherwise.

    // 5. Scouts: look where we plan to hit next.
    for s in mine.iter().filter(|s| c.free(s.id) && rules::can_order(world, s.kind, OrderKind::Scout)).cloned().collect::<Vec<_>>() {
        let zone = match targets.get(c.roll(3.min(targets.len().max(1) as u64)) as usize) {
            Some(t) => world.nodes[t.node as usize].zone,
            None => {
                let zs: Vec<u16> = world.zones.iter().filter(|z| z.region.faction() != Some(c.f)).map(|z| z.idx).collect();
                zs[c.roll(zs.len() as u64) as usize]
            }
        };
        let m = Mission::Scout { zone };
        if c.ok(&s, &m) {
            c.set(s.id, m);
        }
    }

    // 6. Fuel: drop the least important missions until the plan is affordable.
    budget_fuel(&mut c, &mine);

    // 7. Repairs, then builds.
    let mut supply = state.seats[seat as usize].supply;
    let e = &world.rules().economy;
    if c.level == Level::Normal {
        let mut damaged: Vec<(i32, u16)> = world
            .nodes
            .iter()
            .filter(|n| state.nodes[n.idx as usize].owner == Some(seat) && state.nodes[n.idx as usize].health < 2 && n.kind != NodeKind::Lake)
            .map(|n| {
                let pri = match n.kind {
                    NodeKind::AirDefense | NodeKind::Radar => 3,
                    NodeKind::City | NodeKind::Factory | NodeKind::Fuel | NodeKind::Junction => 2,
                    _ => 1,
                };
                (pri * 10 - state.nodes[n.idx as usize].health as i32 * 5, n.idx)
            })
            .collect();
        damaged.sort_by(|a, b| b.cmp(a));
        for (_, n) in damaged.into_iter().take(2) {
            if supply >= e.repair_cost + 4 {
                c.orders.repairs.push(n);
                supply -= e.repair_cost;
            }
        }
    }
    // Faction actions spend Supply too; reserve for them first.
    faction_actions(&mut c, &mut supply);
    let mut cap = rules::build_capacity(world, state, seat);
    let mut room: i32 = world
        .nodes
        .iter()
        .filter(|n| n.kind.is_base() && state.nodes[n.idx as usize].owner == Some(seat))
        .map(|n| rules::base_capacity(world, state, n.idx, UnitKind::Fighter) - rules::base_occupancy(state, n.idx, None))
        .sum();
    room -= state.pending.iter().filter(|p| p.seat == seat).count() as i32;
    let unique = world.faction(c.f).unique;
    let count = |k: UnitKind| mine.iter().filter(|s| s.kind == k).count() as i32;
    while cap > 0 && room > 0 {
        let fighters = count(UnitKind::Fighter) + c.orders.builds.iter().filter(|&&k| k == UnitKind::Fighter).count() as i32;
        let bombers = count(UnitKind::Bomber) + c.orders.builds.iter().filter(|&&k| k == UnitKind::Bomber).count() as i32;
        let mut want = if fighters < 3 {
            UnitKind::Fighter
        } else if bombers < 2 || bombers < fighters - 1 {
            UnitKind::Bomber
        } else {
            UnitKind::Fighter
        };
        if c.level == Level::Normal && rules::buildable(world, state, seat, unique).is_ok() && world.unit(unique).buildable {
            let uniques = count(unique) + c.orders.builds.iter().filter(|&&k| k == unique).count() as i32;
            let cheap = world.unit(unique).cost <= 3;
            if uniques < if cheap { 3 } else { 1 } {
                want = unique;
            }
        }
        if count(UnitKind::Scout) == 0 && !c.orders.builds.contains(&UnitKind::Scout) {
            want = UnitKind::Scout;
        }
        let cost = world.unit(want).cost;
        if supply < cost || rules::buildable(world, state, seat, want).is_err() {
            break;
        }
        c.orders.builds.push(want);
        supply -= cost;
        cap -= 1;
        room -= 1;
    }
    // Keep orders in a stable, priority order: guards, strikes, escorts, others.
    let prio = |m: &Mission| match m {
        Mission::Guard { .. } | Mission::GuardZone { .. } => 0,
        Mission::Attack { .. } => 1,
        Mission::Escort { .. } | Mission::JamEscort { .. } => 2,
        Mission::Scout { .. } => 3,
        Mission::Sweep { .. } | Mission::JamZone { .. } => 4,
        Mission::Rebase { .. } => 5,
        Mission::Rest => 6,
    };
    c.orders.missions.sort_by_key(|o| (prio(&o.mission), o.sq));
    c.orders
}

fn budget_fuel(c: &mut Ctx, mine: &[Squadron]) {
    let w = c.w;
    let st = c.st;
    let fuel = st.seats[c.seat as usize].fuel;
    loop {
        let mut total = 0;
        let mut costs: Vec<(u32, i32, i32)> = Vec::new(); // (sq, cost, priority; higher = drop first)
        for o in &c.orders.missions {
            let Some(s) = mine.iter().find(|s| s.id == o.sq) else { continue };
            if o.mission == Mission::Rest {
                continue;
            }
            let cost = match risk::preview_with(w, st, &c.orders, s, &o.mission, o.waypoint, &c.planner) {
                Some(p) => p.fuel,
                None => 0,
            };
            total += cost;
            let pri = match o.mission {
                Mission::Guard { .. } | Mission::GuardZone { .. } => 1,
                Mission::Attack { .. } => 2,
                Mission::Escort { .. } | Mission::JamEscort { .. } => 3,
                Mission::Scout { .. } => 4,
                _ => 5,
            };
            costs.push((o.sq, cost, pri));
        }
        if total <= fuel {
            return;
        }
        costs.sort_by_key(|x| (-x.2, -x.1, x.0));
        let Some(&(drop, _, _)) = costs.iter().find(|x| x.1 > 0) else { return };
        c.set(drop, Mission::Rest);
        // Escorts of a dropped lead have nothing to fly with.
        let orphans: Vec<u32> = c
            .orders
            .missions
            .iter()
            .filter(|o| matches!(o.mission, Mission::Escort { sq } | Mission::JamEscort { sq } if sq == drop))
            .map(|o| o.sq)
            .collect();
        for o in orphans {
            c.set(o, Mission::Rest);
        }
    }
}

fn faction_actions(c: &mut Ctx, supply: &mut i32) {
    let w = c.w;
    let st = c.st;
    let seat = c.seat;
    let stage = rules::stage(w, st.night);
    // Bid on the Harvest Shipment.
    if st.harvest > 0 && c.f != FactionId::GrainBelt && *supply >= 6 {
        let bid = (st.harvest / 2 + c.roll(3) as i32 - 1).clamp(1, *supply / 3);
        let bid = if c.level == Level::Easy { (bid / 2).max(1) } else { bid };
        c.orders.harvest_bid = bid;
        *supply -= bid;
    }
    match c.f {
        FactionId::Dreamland => {
            // Bet that a neighbor hits whoever is closest to them.
            let others: Vec<u8> = (0..st.seats.len() as u8).filter(|&s| s != seat).collect();
            if !others.is_empty() && *supply >= 4 {
                let on = others[c.roll(others.len() as u64) as usize];
                let on_cap = w.nodes[w.capitals[rules::faction(st, on).index()] as usize].pos;
                let victim = (0..st.seats.len() as u8)
                    .filter(|&v| v != on)
                    .min_by_key(|&v| dist(on_cap, w.nodes[w.capitals[rules::faction(st, v).index()] as usize].pos))
                    .unwrap_or(seat);
                let stake = 2 + c.roll(2) as i32;
                c.orders.bet = Some(Bet { on, node: None, victim: Some(victim), stake });
                *supply -= stake;
            }
        }
        FactionId::FrontRange => {
            c.orders.intel_sale = Some(IntelSale { price: 3, only: None });
        }
        _ => {}
    }
    // Buy a radar picture from Colorado when flush.
    if c.f != FactionId::FrontRange && rules::is_seated(st, FactionId::FrontRange) && *supply >= 10 && c.level == Level::Normal {
        let t = analysis::best_targets(w, st, seat, 1);
        if let Some(t) = t.first() {
            c.orders.intel_request = Some(IntelRequest { zone: w.nodes[t.node as usize].zone, max_price: 3 });
        }
    }
    // The Mass Driver and the Twine revenge strike.
    let md = w.nodes.iter().find(|n| n.kind == NodeKind::MassDriver).map(|n| n.idx);
    if let Some(md) = md {
        if st.nodes[md as usize].owner == Some(seat) && stage == 2 {
            if let Some(t) = analysis::best_targets(w, st, seat, 1).first() {
                c.orders.mass_driver = Some(t.node);
            }
        }
    }
    if c.f == FactionId::GrainBelt {
        if let Some(v) = st.revenge_against {
            let health = analysis::believed_health(st, seat);
            let best = w
                .nodes
                .iter()
                .filter(|n| st.nodes[n.idx as usize].owner == Some(v) && rules::attack_check(w, st, seat, n.idx).is_ok())
                .max_by_key(|n| (analysis::node_value(w, st, seat, n.idx, &health).0, n.idx));
            if let Some(n) = best {
                c.orders.revenge = Some(n.idx);
            }
        }
    }
}

/// Default carry-over orders plus fresh faction actions: what a quiet turn looks like.
pub fn carry_over(world: &World, state: &GameState, seat: u8) -> Orders {
    let mut o = sim::carry_over(world, state, seat);
    o.missions.sort_by_key(|m| m.sq);
    o
}
