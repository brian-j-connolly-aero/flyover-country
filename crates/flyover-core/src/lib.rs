//! Flyover Country's deterministic simulation. Pure functions over plain data:
//! no I/O, no clock, no floats. Same seed and orders, same night, everywhere.

pub mod content;
pub mod economy;
pub mod rng;
pub mod route;
pub mod rules;
pub mod setup;
pub mod sim;
pub mod state;
pub mod testkit;
pub mod vision;

pub use content::{Content, ContentFiles, FactionId, UnitKind, World};
pub use setup::{new_game, GameConfig};
pub use sim::{carry_over, resolve_night};
pub use state::{EventRec, GameState, Orders};

/// Hash of a state's canonical JSON; replays must reproduce it exactly.
pub fn state_hash(state: &GameState) -> String {
    let bytes = serde_json::to_vec(state).expect("state serializes");
    format!("{:016x}", rng::fnv1a(&bytes))
}
