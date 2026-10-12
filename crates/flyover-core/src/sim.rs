//! One night of air war. Every raid flies a straight line from its Airfield to
//! its target. Patrol circles jump any enemy raid that crosses them. Raids that
//! get through face flak, then drop bombs or fight for a prize. Dawn pays
//! income, recovers resting squadrons and builds new ones.

use crate::content::{FactionId, SiteKind, UnitKind, World};
use crate::geom::{along, circle_entry, dist, P};
use crate::rng::Rng;
use crate::rules::{
    doghouse_eligible, has_perk, max_planes, normalize, patrol_radius, site_up, spend, stage,
};
use crate::setup::add_squadron;
use crate::state::{
    DawnReport, Event, EventRec, GameState, LastNight, Loss, Mission, Orders, PatrolMark, RaidMark,
    ScoreReason, Target,
};

struct Raid {
    id: u32,
    seat: u8,
    target: Target,
    from: P,
    to: P,
    sqs: Vec<usize>,
    arrive: u32,
    /// Has met a patrol already tonight.
    met: bool,
}

struct Patrol {
    sq: usize,
    seat: u8,
    at: P,
    r: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    /// A raid enters a patrol circle (raid index, patrol index).
    Enter(usize, usize),
    /// A raid reaches its target.
    Arrive(usize),
    /// The Big Fetch lands (seat).
    Fetch(u8),
}

struct Night<'w> {
    w: &'w World,
    st: GameState,
    orders: Vec<Orders>,
    rng: Rng,
    log: Vec<EventRec>,
    raids: Vec<Raid>,
    patrols: Vec<Patrol>,
    at_park: Vec<bool>,
    /// Who holds each prize tonight: the seat and the raids sitting on it.
    holders: Vec<Option<(u8, Vec<usize>)>>,
}

pub fn resolve_night(
    world: &World,
    state: &GameState,
    orders: &[Orders],
) -> (GameState, Vec<EventRec>) {
    let n = state.seats.len();
    let mut cleaned = Vec::with_capacity(n);
    for seat in 0..n as u8 {
        let o = orders
            .iter()
            .find(|o| o.seat == seat)
            .cloned()
            .unwrap_or_else(|| Orders::empty(seat));
        cleaned.push(normalize(world, state, &o));
    }
    let mut night = Night {
        w: world,
        st: state.clone(),
        orders: cleaned,
        rng: Rng::for_night(state.seed, state.night, 0x5EED),
        log: Vec::new(),
        raids: Vec::new(),
        patrols: Vec::new(),
        at_park: vec![false; n],
        holders: vec![None; world.content.map.prizes.len()],
    };
    night.run();
    (night.st, night.log)
}

impl<'w> Night<'w> {
    fn emit(&mut self, tick: u32, e: Event) {
        self.log.push(EventRec { tick, e });
    }

    fn sq_index(&self, id: u32) -> Option<usize> {
        self.st.squadrons.iter().position(|s| s.id == id)
    }

    fn run(&mut self) {
        let night = self.st.night;
        self.emit(
            0,
            Event::NightStart {
                night,
                stage: stage(self.w, night),
            },
        );
        self.prepare();
        let steps = self.plan();
        for (tick, step) in steps {
            match step {
                Step::Enter(r, p) => self.intercept(tick, r, p),
                Step::Arrive(r) => self.arrive(tick, r),
                Step::Fetch(seat) => self.fetch(tick, seat),
            }
        }
        self.dawn();
    }

    /// Pay for repairs and builds, send factions to the park, and form patrols and raids.
    fn prepare(&mut self) {
        let n = self.st.seats.len();
        for seat in 0..n {
            let o = self.orders[seat].clone();
            let cost = spend(self.w, &self.st, &o);
            self.st.seats[seat].supply -= cost;
            for &site in &o.repairs {
                let s = &mut self.st.sites[site as usize];
                s.health = (s.health + 1).min(s.max);
            }
            if o.retreat {
                self.at_park[seat] = true;
                self.st.seats[seat].been_to_park = true;
                self.emit(0, Event::BarkPark { seat: seat as u8 });
            }
        }
        // Patrols, in squadron order.
        for i in 0..self.st.squadrons.len() {
            let sq = &self.st.squadrons[i];
            let seat = sq.seat;
            if let Mission::Patrol { at } = self.orders[seat as usize].mission(sq.id) {
                let r = patrol_radius(self.w, &self.st, seat);
                let (id, planes) = (sq.id, sq.planes);
                self.patrols.push(Patrol { sq: i, seat, at, r });
                self.emit(
                    1,
                    Event::Patrol {
                        seat,
                        sq: id,
                        at,
                        r,
                        planes,
                    },
                );
            }
        }
        // Raids: one per seat and target, launched from that seat's Airfield.
        let launch = self.w.rules().launch_tick;
        let speed = self.w.rules().speed.max(1);
        for seat in 0..n as u8 {
            let mut targets: Vec<Target> = Vec::new();
            for sq in self.st.squadrons_of(seat) {
                if let Mission::Fly { target } = self.orders[seat as usize].mission(sq.id) {
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
            }
            targets.sort();
            let from = self
                .w
                .site(self.w.site_of(self.st.faction(seat), SiteKind::Airfield))
                .pos;
            for target in targets {
                let sqs: Vec<usize> = (0..self.st.squadrons.len())
                    .filter(|&i| {
                        let s = &self.st.squadrons[i];
                        s.seat == seat
                            && self.orders[seat as usize].mission(s.id) == Mission::Fly { target }
                    })
                    .collect();
                let to = self.target_pos(target);
                let len = dist(from, to);
                let arrive = launch + ((len + speed - 1) / speed).max(1) as u32;
                let id = self.raids.len() as u32 + 1;
                let (fighters, bombers) = self.count(&sqs);
                let ids = sqs.iter().map(|&i| self.st.squadrons[i].id).collect();
                self.emit(
                    launch,
                    Event::Launch {
                        raid: id,
                        seat,
                        sqs: ids,
                        from,
                        to,
                        target,
                        fighters,
                        bombers,
                        arrive,
                    },
                );
                self.raids.push(Raid {
                    id,
                    seat,
                    target,
                    from,
                    to,
                    sqs,
                    arrive,
                    met: false,
                });
            }
        }
    }

    fn target_pos(&self, t: Target) -> P {
        match t {
            Target::Site { site } => self.w.site(site).pos,
            Target::Prize { prize } => self.w.content.map.prizes[prize as usize].pos,
        }
    }

    /// Fighter and bomber planes still flying in these squadrons.
    fn count(&self, sqs: &[usize]) -> (i32, i32) {
        let mut f = 0;
        let mut b = 0;
        for &i in sqs {
            let s = &self.st.squadrons[i];
            match s.kind {
                UnitKind::Fighter => f += s.planes,
                UnitKind::Bomber => b += s.planes,
            }
        }
        (f, b)
    }

    /// Everything that will happen tonight, in order.
    fn plan(&self) -> Vec<(u32, Step)> {
        let launch = self.w.rules().launch_tick;
        let speed = self.w.rules().speed.max(1);
        let mut steps = Vec::new();
        for (ri, raid) in self.raids.iter().enumerate() {
            let len = dist(raid.from, raid.to);
            // Patrols jump raids headed for their own sites or for a prize.
            // Utah's neighborhood watch jumps every raid that flies through.
            let victim = match raid.target {
                Target::Site { site } => self.st.site_owner(self.w, site),
                Target::Prize { .. } => None,
            };
            let at_prize = matches!(raid.target, Target::Prize { .. });
            for (pi, p) in self.patrols.iter().enumerate() {
                if p.seat == raid.seat {
                    continue;
                }
                let watch = has_perk(self.w, &self.st, p.seat, FactionId::Beehive);
                if !(at_prize || victim == Some(p.seat) || watch) {
                    continue;
                }
                if let Some(s) = circle_entry(raid.from, raid.to, p.at, p.r) {
                    if s < len {
                        steps.push((launch + (s / speed) as u32, Step::Enter(ri, pi)));
                    }
                }
            }
            steps.push((raid.arrive, Step::Arrive(ri)));
        }
        for (seat, o) in self.orders.iter().enumerate() {
            if o.fetch.is_some() {
                steps.push((self.w.rules().bark_park.fetch_tick, Step::Fetch(seat as u8)));
            }
        }
        steps.sort();
        steps
    }

    fn alive(&self, sqs: &[usize]) -> bool {
        sqs.iter().any(|&i| self.st.squadrons[i].planes > 0)
    }

    fn intercept(&mut self, tick: u32, ri: usize, pi: usize) {
        let pq = self.patrols[pi].sq;
        if self.st.squadrons[pq].planes <= 0 || !self.alive(&self.raids[ri].sqs) {
            return;
        }
        let (rseat, pseat, at) = {
            let r = &self.raids[ri];
            let p = &self.patrols[pi];
            let s = circle_entry(r.from, r.to, p.at, p.r).unwrap_or(0);
            (r.seat, p.seat, along(r.from, r.to, s))
        };
        let (rid, pid) = (self.raids[ri].id, self.st.squadrons[pq].id);
        // Cascadian raids hide in the clouds: the first patrol they meet only
        // gets one round at them.
        let cloud = !self.raids[ri].met && has_perk(self.w, &self.st, rseat, FactionId::Cascadia);
        self.raids[ri].met = true;
        let rounds = if cloud {
            1
        } else {
            self.w.rules().combat.rounds
        };
        let raid_sqs = self.raids[ri].sqs.clone();
        let (patrol_loss, raid_loss) = self.fight(&[pq], &raid_sqs, rounds);
        let patrol_lost: i32 = patrol_loss.iter().map(|l| l.planes).sum();
        let downed: i32 = raid_loss.iter().map(|l| l.planes).sum();
        let wiped = !self.alive(&raid_sqs);
        self.emit(
            tick,
            Event::Intercept {
                raid: rid,
                seat: rseat,
                patrol: pid,
                patrol_seat: pseat,
                at,
                raid_lost: raid_loss.clone(),
                patrol_lost,
                cloud,
                wiped,
            },
        );
        self.after_fight(tick, pseat, downed, rseat, patrol_lost);
        self.down_check(tick, &raid_loss);
        self.down_check(tick, &patrol_loss);
    }

    /// Score shootdowns for both sides of a fight.
    fn after_fight(&mut self, tick: u32, a: u8, a_downed: i32, b: u8, b_downed: i32) {
        let per = self.w.rules().score.plane;
        for (seat, downed, victim) in [(a, a_downed, b), (b, b_downed, a)] {
            if downed > 0 {
                self.st.seats[seat as usize].stats.downed += downed;
                self.st.seats[victim as usize].stats.lost += downed;
                self.score(tick, seat, downed * per, ScoreReason::Downed);
            }
        }
    }

    fn score(&mut self, tick: u32, seat: u8, delta: i32, reason: ScoreReason) {
        if delta == 0 {
            return;
        }
        self.st.seats[seat as usize].score += delta;
        self.emit(
            tick,
            Event::Score {
                seat,
                delta,
                reason,
            },
        );
    }

    /// Announce squadrons that just lost their last plane.
    fn down_check(&mut self, tick: u32, losses: &[Loss]) {
        for l in losses {
            if let Some(i) = self.sq_index(l.sq) {
                let s = &self.st.squadrons[i];
                if s.planes <= 0 && l.planes > 0 {
                    let e = Event::SquadronDown {
                        sq: s.id,
                        seat: s.seat,
                        ace: s.ace,
                    };
                    self.emit(tick, e);
                }
            }
        }
    }

    /// Two sides trade fire for some rounds. Fighters shoot; bombers don't.
    /// Hits land on fighters first, then bombers.
    fn fight(&mut self, a: &[usize], b: &[usize], rounds: i32) -> (Vec<Loss>, Vec<Loss>) {
        let mut loss_a: Vec<Loss> = Vec::new();
        let mut loss_b: Vec<Loss> = Vec::new();
        for _ in 0..rounds {
            let ha = self.volley(a);
            let hb = self.volley(b);
            self.remove(b, ha, &mut loss_b, false);
            self.remove(a, hb, &mut loss_a, false);
        }
        (loss_a, loss_b)
    }

    /// Hits scored by every fighter plane on a side.
    fn volley(&mut self, side: &[usize]) -> i32 {
        let c = &self.w.rules().combat;
        let (hit, ace_hit) = (c.hit, c.ace_hit);
        let mut hits = 0;
        for &i in side {
            let s = &self.st.squadrons[i];
            if s.kind != UnitKind::Fighter {
                continue;
            }
            let chance = if s.ace { ace_hit } else { hit };
            for _ in 0..s.planes {
                if self.rng.chance(chance) {
                    hits += 1;
                }
            }
        }
        hits
    }

    /// Take `hits` planes off a side: fighters first (bombers first when
    /// `bombers_first`), the fullest squadron first, aces last.
    fn remove(&mut self, side: &[usize], hits: i32, losses: &mut Vec<Loss>, bombers_first: bool) {
        let order = if bombers_first {
            [UnitKind::Bomber, UnitKind::Fighter]
        } else {
            [UnitKind::Fighter, UnitKind::Bomber]
        };
        let mut left = hits;
        for kind in order {
            while left > 0 {
                let pick = side
                    .iter()
                    .copied()
                    .filter(|&i| {
                        self.st.squadrons[i].kind == kind && self.st.squadrons[i].planes > 0
                    })
                    .max_by_key(|&i| {
                        let s = &self.st.squadrons[i];
                        (!s.ace, s.planes, std::cmp::Reverse(s.id))
                    });
                let Some(i) = pick else { break };
                self.st.squadrons[i].planes -= 1;
                left -= 1;
                let id = self.st.squadrons[i].id;
                match losses.iter_mut().find(|l| l.sq == id) {
                    Some(l) => l.planes += 1,
                    None => losses.push(Loss { sq: id, planes: 1 }),
                }
            }
        }
    }

    fn arrive(&mut self, tick: u32, ri: usize) {
        if !self.alive(&self.raids[ri].sqs) {
            return;
        }
        match self.raids[ri].target {
            Target::Site { site } => self.strike(tick, ri, site),
            Target::Prize { prize } => self.claim(tick, ri, prize),
        }
    }

    fn pip_value(&self, site: u16) -> i32 {
        let s = &self.w.rules().score;
        match self.w.site(site).kind {
            SiteKind::HQ | SiteKind::Jewel => s.big_pip,
            SiteKind::Factory | SiteKind::Airfield => s.pip,
        }
    }

    fn strike(&mut self, tick: u32, ri: usize, site: u16) {
        let (rid, seat) = (self.raids[ri].id, self.raids[ri].seat);
        let Some(victim) = self.st.site_owner(self.w, site) else {
            return;
        };
        if self.at_park[victim as usize] {
            self.emit(
                tick,
                Event::Bounce {
                    raid: rid,
                    seat,
                    site,
                    victim,
                },
            );
            return;
        }
        let c = &self.w.rules().combat;
        let (flak_hit, bomb_hit, crop) = (c.flak_hit, c.bomb_hit, c.crop_duster_hit);
        let shots = if self.st.sites[site as usize].health <= 0 {
            0
        } else if self.w.site(site).kind == SiteKind::HQ {
            c.hq_flak_shots
        } else {
            c.flak_shots
        };
        let mut hits = 0;
        for _ in 0..shots {
            if self.rng.chance(flak_hit) {
                hits += 1;
            }
        }
        let sqs = self.raids[ri].sqs.clone();
        let mut flak_loss = Vec::new();
        self.remove(&sqs, hits, &mut flak_loss, true);
        let downed: i32 = flak_loss.iter().map(|l| l.planes).sum();
        if shots > 0 {
            self.emit(
                tick,
                Event::Flak {
                    raid: rid,
                    seat,
                    site,
                    victim,
                    raid_lost: flak_loss.clone(),
                },
            );
        }
        if downed > 0 {
            self.after_fight(tick, victim, downed, seat, 0);
            self.down_check(tick, &flak_loss);
        }
        let (_, bombers) = self.count(&sqs);
        // Grain Belt crop dusters fly low and hit more often.
        let chance = if has_perk(self.w, &self.st, seat, FactionId::GrainBelt) {
            crop
        } else {
            bomb_hit
        };
        let mut bomb_hits = 0;
        for _ in 0..bombers {
            if self.rng.chance(chance) {
                bomb_hits += 1;
            }
        }
        let health = self.st.sites[site as usize].health;
        let dealt = bomb_hits.min(health.max(0));
        self.st.sites[site as usize].health = health - dealt;
        if bombers > 0 {
            self.emit(
                tick,
                Event::Strike {
                    raid: rid,
                    seat,
                    site,
                    victim,
                    bombers,
                    hits: dealt,
                    health: health - dealt,
                },
            );
        }
        if dealt > 0 {
            self.st.seats[seat as usize].stats.pips_dealt += dealt;
            self.st.seats[victim as usize].stats.pips_taken += dealt;
            let v = self.pip_value(site);
            self.score(tick, seat, dealt * v, ScoreReason::Strike);
        }
    }

    fn claim(&mut self, tick: u32, ri: usize, prize: u16) {
        let (rid, seat) = (self.raids[ri].id, self.raids[ri].seat);
        let p = prize as usize;
        match self.holders[p].clone() {
            None => {
                self.holders[p] = Some((seat, vec![ri]));
                self.emit(
                    tick,
                    Event::Hold {
                        raid: rid,
                        seat,
                        prize,
                    },
                );
            }
            Some((holder, mut raids)) if holder == seat => {
                raids.push(ri);
                self.holders[p] = Some((holder, raids));
                self.emit(
                    tick,
                    Event::Hold {
                        raid: rid,
                        seat,
                        prize,
                    },
                );
            }
            Some((holder, raids)) => {
                let held: Vec<usize> = raids
                    .iter()
                    .flat_map(|&r| self.raids[r].sqs.clone())
                    .collect();
                let arriving = self.raids[ri].sqs.clone();
                let rounds = self.w.rules().combat.rounds;
                let (holder_lost, raid_lost) = self.fight(&held, &arriving, rounds);
                let (hf, hb) = self.count(&held);
                let (af, ab) = self.count(&arriving);
                let taken = af + ab > hf + hb;
                if taken {
                    self.holders[p] = Some((seat, vec![ri]));
                } else if hf + hb == 0 {
                    self.holders[p] = None;
                }
                self.emit(
                    tick,
                    Event::PrizeFight {
                        raid: rid,
                        seat,
                        prize,
                        holder,
                        raid_lost: raid_lost.clone(),
                        holder_lost: holder_lost.clone(),
                        taken,
                    },
                );
                let a_downed: i32 = raid_lost.iter().map(|l| l.planes).sum();
                let b_downed: i32 = holder_lost.iter().map(|l| l.planes).sum();
                self.after_fight(tick, holder, a_downed, seat, b_downed);
                self.down_check(tick, &raid_lost);
                self.down_check(tick, &holder_lost);
            }
        }
    }

    fn fetch(&mut self, tick: u32, seat: u8) {
        let Some(site) = self.orders[seat as usize].fetch else {
            return;
        };
        self.st.seats[seat as usize].fetch = false;
        let Some(victim) = self.st.site_owner(self.w, site) else {
            return;
        };
        if self.at_park[victim as usize] {
            let health = self.st.sites[site as usize].health;
            self.emit(
                tick,
                Event::Fetch {
                    seat,
                    site,
                    victim,
                    hits: 0,
                    health,
                    bounced: true,
                },
            );
            return;
        }
        let pips = self.w.rules().bark_park.fetch_pips;
        let health = self.st.sites[site as usize].health;
        let dealt = pips.min(health.max(0));
        self.st.sites[site as usize].health = health - dealt;
        self.emit(
            tick,
            Event::Fetch {
                seat,
                site,
                victim,
                hits: dealt,
                health: health - dealt,
                bounced: false,
            },
        );
        if dealt > 0 {
            self.st.seats[seat as usize].stats.pips_dealt += dealt;
            self.st.seats[victim as usize].stats.pips_taken += dealt;
            let v = self.pip_value(site);
            self.score(tick, seat, dealt * v, ScoreReason::Fetch);
        }
    }

    fn dawn(&mut self) {
        let tick = self.w.rules().ticks;
        let n = self.st.seats.len();
        let r = self.w.rules().clone();
        // Prizes go to whoever holds them with planes left.
        for p in 0..self.holders.len() {
            if let Some((seat, raids)) = self.holders[p].clone() {
                let sqs: Vec<usize> = raids
                    .iter()
                    .flat_map(|&r| self.raids[r].sqs.clone())
                    .collect();
                if self.alive(&sqs) {
                    self.st.prizes[p].owner = Some(seat);
                }
            }
        }
        let mut report = DawnReport {
            income: vec![0; n],
            prizes: self.st.prizes.iter().map(|p| p.owner).collect(),
            built: vec![Vec::new(); n],
            returned: Vec::new(),
            doghouse: Vec::new(),
        };
        for seat in 0..n as u8 {
            let held = self
                .st
                .prizes
                .iter()
                .filter(|p| p.owner == Some(seat))
                .count() as i32;
            // Dreamland's high rollers get paid double Supply for prizes.
            let pay = if has_perk(self.w, &self.st, seat, FactionId::Dreamland) {
                2
            } else {
                1
            };
            if held > 0 {
                self.st.seats[seat as usize].stats.prizes += held;
                self.score(tick, seat, held * r.score.prize, ScoreReason::Prize);
            }
            let f = self.st.faction(seat);
            let mut income = held * r.income.prize * pay;
            if site_up(self.w, &self.st, seat, SiteKind::HQ) {
                income += r.income.hq;
            }
            let factory = self.st.sites[self.w.site_of(f, SiteKind::Factory) as usize].health;
            if factory > 0 {
                income += factory + r.income.factory_base;
            }
            self.st.seats[seat as usize].supply += income;
            report.income[seat as usize] = income;
        }
        // Resting squadrons get a plane back if their Airfield stands.
        for i in 0..self.st.squadrons.len() {
            let s = &self.st.squadrons[i];
            let rested = self.orders[s.seat as usize].mission(s.id) == Mission::Rest;
            if rested && s.planes > 0 && site_up(self.w, &self.st, s.seat, SiteKind::Airfield) {
                let s = &mut self.st.squadrons[i];
                s.planes = (s.planes + 1).min(s.max);
            }
        }
        self.st.squadrons.retain(|s| s.planes > 0);
        // New squadrons, if the Airfield survived the night (otherwise refunded).
        for seat in 0..n as u8 {
            let builds = self.orders[seat as usize].builds.clone();
            for k in builds {
                if site_up(self.w, &self.st, seat, SiteKind::Airfield) {
                    add_squadron(self.w, &mut self.st, seat, k, false);
                    report.built[seat as usize].push(k);
                } else {
                    let cost = crate::rules::build_cost(self.w, &self.st, seat, k);
                    self.st.seats[seat as usize].supply += cost;
                }
            }
        }
        // Back from Project Bark Park: refilled, topped up, repaired, Fetch loaded.
        for seat in 0..n as u8 {
            if !self.at_park[seat as usize] {
                continue;
            }
            report.returned.push(seat);
            for s in self.st.squadrons.iter_mut().filter(|s| s.seat == seat) {
                s.planes = s.max;
            }
            let mut next = UnitKind::Bomber;
            while self.st.squadrons_of(seat).count() < r.bark_park.squadrons.min(r.max_squadrons) {
                add_squadron(self.w, &mut self.st, seat, next, false);
                report.built[seat as usize].push(next);
                next = if next == UnitKind::Bomber {
                    UnitKind::Fighter
                } else {
                    UnitKind::Bomber
                };
            }
            let f = self.st.faction(seat);
            for k in [
                SiteKind::HQ,
                SiteKind::Factory,
                SiteKind::Airfield,
                SiteKind::Jewel,
            ] {
                let s = &mut self.st.sites[self.w.site_of(f, k) as usize];
                s.health = s.max;
            }
            let me = &mut self.st.seats[seat as usize];
            me.supply += r.bark_park.supply;
            me.fetch = true;
        }
        // Texas bombers grow or shrink with the perk (it follows the Crown Jewel).
        for i in 0..self.st.squadrons.len() {
            let (seat, kind) = (self.st.squadrons[i].seat, self.st.squadrons[i].kind);
            let max = max_planes(self.w, &self.st, seat, kind);
            let s = &mut self.st.squadrons[i];
            s.max = max;
            s.planes = s.planes.min(max);
        }
        // Marks for tomorrow's map, then on to the next night.
        self.st.last = LastNight {
            patrols: self
                .patrols
                .iter()
                .map(|p| PatrolMark {
                    seat: p.seat,
                    at: p.at,
                    r: p.r,
                })
                .collect(),
            raids: self
                .raids
                .iter()
                .map(|r| RaidMark {
                    seat: r.seat,
                    from: r.from,
                    to: r.to,
                    target: r.target,
                })
                .collect(),
        };
        self.st
            .history
            .push(self.st.seats.iter().map(|s| s.score).collect());
        self.st.night += 1;
        let next = self.st.night;
        for seat in 0..n as u8 {
            let ok = next <= r.nights && doghouse_eligible(self.w, &self.st, seat, next);
            self.st.seats[seat as usize].doghouse = ok;
            if ok {
                report.doghouse.push(seat);
            }
        }
        self.emit(tick, Event::Dawn(Box::new(report)));
    }
}

/// Last night's orders, kept where they still make sense: the default for a
/// new night's plan.
pub fn carry_over(world: &World, state: &GameState, prev: &Orders) -> Orders {
    let mut o = Orders::empty(prev.seat);
    for m in &prev.missions {
        if let Some(sq) = state.squadron(m.sq) {
            if crate::rules::mission_check(world, state, sq, &m.mission).is_ok() {
                o.missions.push(m.clone());
            }
        }
    }
    o
}
