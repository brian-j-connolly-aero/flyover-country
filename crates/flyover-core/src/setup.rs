//! A new game: seats, sites at full health, starting squadrons.

use crate::content::{FactionId, UnitKind, World};
use crate::rules::max_planes;
use crate::state::{GameState, LastNight, PrizeState, SeatState, SiteState, Squadron};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameConfig {
    pub seed: u64,
    /// One faction per seat, in seat order.
    pub factions: Vec<FactionId>,
}

pub const VERSION: u32 = 2;

pub fn new_game(world: &World, cfg: &GameConfig) -> Result<GameState, String> {
    if cfg.factions.len() < 2 || cfg.factions.len() > 7 {
        return Err("Pick two to seven factions.".into());
    }
    for (i, f) in cfg.factions.iter().enumerate() {
        if cfg.factions[..i].contains(f) {
            return Err("Each faction can only be picked once.".into());
        }
    }
    let r = world.rules();
    let seats = cfg
        .factions
        .iter()
        .map(|&f| SeatState {
            faction: Some(f),
            supply: r.start_supply,
            ..Default::default()
        })
        .collect();
    let sites = world
        .content
        .map
        .sites
        .iter()
        .map(|s| {
            let max = world.max_health(s.kind);
            let active = cfg.factions.contains(&s.faction);
            SiteState {
                health: if active { max } else { 0 },
                max,
                active,
            }
        })
        .collect();
    let mut state = GameState {
        version: VERSION,
        seed: cfg.seed,
        night: 1,
        seats,
        sites,
        prizes: vec![PrizeState::default(); world.content.map.prizes.len()],
        squadrons: Vec::new(),
        next_sq: 1,
        last: LastNight::default(),
        history: Vec::new(),
    };
    for seat in 0..cfg.factions.len() as u8 {
        add_squadron(world, &mut state, seat, UnitKind::Fighter, true);
        add_squadron(world, &mut state, seat, UnitKind::Fighter, false);
        add_squadron(world, &mut state, seat, UnitKind::Bomber, false);
        add_squadron(world, &mut state, seat, UnitKind::Bomber, false);
    }
    Ok(state)
}

/// A fresh, full-strength squadron for `seat`.
pub fn add_squadron(
    world: &World,
    state: &mut GameState,
    seat: u8,
    kind: UnitKind,
    ace: bool,
) -> u32 {
    let max = max_planes(world, state, seat, kind);
    let id = state.next_sq;
    state.next_sq += 1;
    let slot = if kind == UnitKind::Fighter { 0 } else { 1 };
    let num = if ace {
        0
    } else {
        state.seats[seat as usize].built[slot] += 1;
        state.seats[seat as usize].built[slot]
    };
    state.squadrons.push(Squadron {
        id,
        seat,
        kind,
        planes: max,
        max,
        ace,
        num,
    });
    id
}
