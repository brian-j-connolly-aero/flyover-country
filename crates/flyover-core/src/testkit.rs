//! Content bundled at compile time, for tests and the command-line tools.

use crate::content::{ContentFiles, World};

pub const MAP: &str = include_str!("../../../content/map.ron");
pub const FACTIONS: &str = include_str!("../../../content/factions.ron");
pub const RULES: &str = include_str!("../../../content/rules.ron");

pub fn files() -> ContentFiles<'static> {
    ContentFiles {
        map: MAP,
        factions: FACTIONS,
        rules: RULES,
    }
}

pub fn world() -> World {
    World::from_files(&files()).expect("content loads")
}
