//! Bots, Suggest Plan, planning previews and advisor tips. Everything here
//! reads game state; nothing here changes it.

pub mod advisor;
pub mod plan;
pub mod preview;

pub use plan::{suggest, Level};
