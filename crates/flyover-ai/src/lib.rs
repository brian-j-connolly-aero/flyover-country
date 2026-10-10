//! The advisor, Suggest Plan, bots, risk colors and network analysis.
//! Everything here reads game state; nothing here changes it.

pub mod advisor;
pub mod analysis;
pub mod plan;
pub mod risk;
pub mod view;

pub use plan::{suggest, Level};
