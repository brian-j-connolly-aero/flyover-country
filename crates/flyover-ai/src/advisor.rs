//! Short, plain planning tips. The most important come first; the planning
//! screen shows the top two.

use crate::preview::{preview, Preview};
use flyover_core::rules::{build_cost, opens_on, site_up};
use flyover_core::state::{GameState, Mission, Orders, Target};
use flyover_core::{SiteKind, UnitKind, World};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Tip {
    /// Stable key, for the screen to style or act on.
    pub key: &'static str,
    /// 3 = act now, 2 = worth a look, 1 = good to know.
    pub level: u8,
    pub text: String,
    /// A site or squadron the tip is about, if any.
    pub site: Option<u16>,
    pub sq: Option<u32>,
}

fn tip(key: &'static str, level: u8, text: String) -> Tip {
    Tip {
        key,
        level,
        text,
        site: None,
        sq: None,
    }
}

pub fn advise(world: &World, state: &GameState, orders: &Orders) -> Vec<Tip> {
    let p: Preview = preview(world, state, orders);
    let o = &p.orders;
    let seat = o.seat;
    let me = &state.seats[seat as usize];
    let mut tips = Vec::new();

    if me.doghouse && !o.retreat {
        tips.push(tip(
            "doghouse",
            3,
            "You can rest at Project Bark Park tonight.".into(),
        ));
    }
    if o.retreat {
        tips.push(tip(
            "retreating",
            1,
            "Off to Project Bark Park. Nothing can touch your sites tonight.".into(),
        ));
        return tips;
    }
    if me.fetch && o.fetch.is_none() {
        tips.push(tip(
            "fetch",
            2,
            "The Big Fetch is loaded. Pick a target in the Faction tab.".into(),
        ));
    }

    // Sites rivals can hit tonight with no patrol over them.
    for c in &p.cover {
        let k = world.site(c.site).kind;
        if c.patrols.is_empty()
            && state.night >= opens_on(world, k)
            && state.sites[c.site as usize].health > 0
        {
            let mut t = tip(
                "uncovered",
                2,
                format!("No patrol over {}.", world.site(c.site).name),
            );
            t.site = Some(c.site);
            tips.push(t);
        }
    }
    // Raids through last night's patrols with no escort.
    for r in &p.raids {
        if r.bombers > 0 && r.fighters == 0 && !r.crosses.is_empty() {
            let name = match r.target {
                Target::Site { site } => world.site(site).name.clone(),
                Target::Prize { prize } => world.content.map.prizes[prize as usize].name.clone(),
            };
            let who = state.faction(r.crosses[0].seat);
            let mut t = tip(
                "unescorted",
                2,
                format!("The raid on {name} flies through where {} patrolled last night. Add an escort?", world.faction(who).short),
            );
            t.sq = r.sqs.first().copied();
            tips.push(t);
        }
    }
    // Squadrons with nothing to do.
    let idle = state
        .squadrons_of(seat)
        .filter(|s| o.mission(s.id) == Mission::Rest && s.planes == s.max)
        .count();
    if idle >= 2 {
        tips.push(tip(
            "idle",
            1,
            format!("{idle} squadrons are resting at full strength."),
        ));
    }
    // Money.
    let f = state.faction(seat);
    if !site_up(world, state, seat, SiteKind::Airfield)
        && !o.repairs.contains(&world.site_of(f, SiteKind::Airfield))
    {
        tips.push(tip(
            "airfield",
            2,
            "Your Airfield is down: repair it to build again.".into(),
        ));
    }
    let cheapest = build_cost(world, state, seat, UnitKind::Fighter);
    let count = state.squadrons_of(seat).count() + o.builds.len();
    if p.supply_left >= cheapest
        && count < world.rules().max_squadrons
        && site_up(world, state, seat, SiteKind::Airfield)
    {
        tips.push(tip(
            "supply",
            1,
            format!("{} Supply left: enough for a new squadron.", p.supply_left),
        ));
    }
    tips.sort_by_key(|t| std::cmp::Reverse(t.level));
    tips
}
