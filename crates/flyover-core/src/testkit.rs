//! Content bundled at compile time, for tests and the command-line tools.

use crate::content::{Content, ContentFiles, World};

pub const MAP: &str = include_str!("../../../content/map.ron");
pub const UNITS: &str = include_str!("../../../content/units.ron");
pub const FACTIONS: &str = include_str!("../../../content/factions.ron");
pub const RULES: &str = include_str!("../../../content/rules.ron");
pub const PATCH_NOTES: &str = include_str!("../../../content/patch_notes.ron");

pub fn files() -> ContentFiles<'static> {
    ContentFiles {
        map: MAP,
        units: UNITS,
        factions: FACTIONS,
        rules: RULES,
        patch_notes: PATCH_NOTES,
    }
}

pub fn world() -> World {
    World::new(Content::parse(&files()).expect("content parses")).expect("content compiles")
}
