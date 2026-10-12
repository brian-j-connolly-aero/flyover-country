//! Browser bindings. The frontend fetches the RON content files, hands them to
//! [`Engine::new`], and from then on passes game state and orders as JSON.
//! The engine is stateless apart from a one-entry parse cache.

use flyover_ai::{advisor, plan, preview, Level};
use flyover_core::content::{ContentFiles, World};
use flyover_core::state::{GameState, Mission, Orders};
use flyover_core::{new_game, resolve_night, rules, state_hash, GameConfig};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// What one dog says, by situation. Every list is a pool: the screens never
/// show the same line twice in a game while the pool lasts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaderLines {
    /// The first morning, right after the pick.
    pub intro: Vec<String>,
    /// An ordinary morning.
    pub dawn: Vec<String>,
    pub ahead: Vec<String>,
    pub behind: Vec<String>,
    /// The morning after one of their sites got hit.
    pub hit: Vec<String>,
    /// The morning after their raid landed.
    pub landed: Vec<String>,
    /// Off to Project Bark Park.
    pub park: Vec<String>,
    /// Back from the park.
    pub back: Vec<String>,
    /// Throwing the Big Fetch.
    pub fetch: Vec<String>,
    pub win: Vec<String>,
    pub lose: Vec<String>,
}

/// Short radio calls during the night.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Radio {
    /// Their patrol jumps a raid.
    pub intercept: Vec<String>,
    /// Their raid gets jumped.
    pub jumped: Vec<String>,
    /// Their bombs land.
    pub hit: Vec<String>,
    /// Their bombs miss.
    pub miss: Vec<String>,
    /// They take or hold a prize.
    pub prize: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlavorDef {
    pub leaders: BTreeMap<String, LeaderLines>,
    pub radio: BTreeMap<String, Radio>,
    /// Newspaper headlines by story.
    pub headlines: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize)]
struct ContentOut<'a> {
    width: i32,
    height: i32,
    zones: &'a [flyover_core::content::ZoneDef],
    fringe: &'a [Vec<flyover_core::geom::P>],
    mississippi: &'a [flyover_core::geom::P],
    sites: &'a [flyover_core::content::SiteDef],
    prizes: &'a [flyover_core::content::PrizeDef],
    landmarks: &'a [flyover_core::content::LandmarkDef],
    factions: &'a [flyover_core::content::FactionDef],
    rules: &'a flyover_core::content::RulesDef,
    flavor: &'a FlavorDef,
}

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn to_json<T: Serialize>(v: &T) -> Result<String, JsValue> {
    serde_json::to_string(v).map_err(err)
}

#[wasm_bindgen]
pub struct Engine {
    world: World,
    flavor: FlavorDef,
    cache: RefCell<Option<(String, GameState)>>,
}

impl Engine {
    fn state(&self, json: &str) -> Result<GameState, JsValue> {
        if let Some((k, st)) = self.cache.borrow().as_ref() {
            if k == json {
                return Ok(st.clone());
            }
        }
        let st: GameState = serde_json::from_str(json).map_err(err)?;
        *self.cache.borrow_mut() = Some((json.to_string(), st.clone()));
        Ok(st)
    }
}

#[derive(Serialize)]
struct Resolved {
    state: GameState,
    events: Vec<flyover_core::EventRec>,
}

#[derive(Serialize)]
struct Checked {
    preview: preview::Preview,
    tips: Vec<advisor::Tip>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(map: &str, factions: &str, rules: &str, flavor: &str) -> Result<Engine, JsValue> {
        let world = World::from_files(&ContentFiles {
            map,
            factions,
            rules,
        })
        .map_err(err)?;
        let flavor: FlavorDef =
            ron::de::from_str(flavor).map_err(|e| err(format!("flavor.ron: {e}")))?;
        Ok(Engine {
            world,
            flavor,
            cache: RefCell::new(None),
        })
    }

    /// Everything the screens need to draw the map and write the words.
    pub fn content_json(&self) -> Result<String, JsValue> {
        let c = &self.world.content;
        to_json(&ContentOut {
            width: c.map.width,
            height: c.map.height,
            zones: &c.map.zones,
            fringe: &c.map.fringe,
            mississippi: &c.map.mississippi,
            sites: &c.map.sites,
            prizes: &c.map.prizes,
            landmarks: &c.map.landmarks,
            factions: &c.factions,
            rules: &c.rules,
            flavor: &self.flavor,
        })
    }

    pub fn new_game(&self, config: &str) -> Result<String, JsValue> {
        let cfg: GameConfig = serde_json::from_str(config).map_err(err)?;
        to_json(&new_game(&self.world, &cfg).map_err(err)?)
    }

    /// Resolve a night: `orders` is a JSON array with one entry per seat.
    pub fn resolve(&self, state: &str, orders: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let orders: Vec<Orders> = serde_json::from_str(orders).map_err(err)?;
        let (state, events) = resolve_night(&self.world, &st, &orders);
        to_json(&Resolved { state, events })
    }

    /// The plan as the night will read it, what the map should draw for it,
    /// and the advisor's tips.
    pub fn check(&self, state: &str, orders: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let o: Orders = serde_json::from_str(orders).map_err(err)?;
        to_json(&Checked {
            preview: preview::preview(&self.world, &st, &o),
            tips: advisor::advise(&self.world, &st, &o),
        })
    }

    /// Why a squadron can't take a mission ("" if it can).
    pub fn mission_error(&self, state: &str, sq: u32, mission: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let m: Mission = serde_json::from_str(mission).map_err(err)?;
        let Some(s) = st.squadron(sq) else {
            return Ok("No such squadron.".into());
        };
        Ok(rules::mission_check(&self.world, &st, s, &m)
            .err()
            .unwrap_or_default())
    }

    /// Why a seat can't patrol here ("" if it can).
    pub fn patrol_error(&self, state: &str, seat: u8, x: i32, y: i32) -> Result<String, JsValue> {
        let st = self.state(state)?;
        Ok(rules::patrol_ok(&self.world, &st, seat, (x, y))
            .err()
            .unwrap_or_default())
    }

    /// Why a seat can't hit this site tonight ("" if it can).
    pub fn site_error(&self, state: &str, seat: u8, site: u16) -> Result<String, JsValue> {
        let st = self.state(state)?;
        Ok(rules::site_open(&self.world, &st, seat, site)
            .err()
            .unwrap_or_default())
    }

    pub fn suggest(&self, state: &str, seat: u8, level: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let level = if level == "easy" {
            Level::Easy
        } else {
            Level::Normal
        };
        to_json(&plan::suggest(&self.world, &st, seat, level))
    }

    /// Last night's missions that still make sense, as tonight's starting plan.
    pub fn carry_over(&self, state: &str, prev: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let prev: Orders = serde_json::from_str(prev).map_err(err)?;
        to_json(&flyover_core::carry_over(&self.world, &st, &prev))
    }

    pub fn state_hash(&self, state: &str) -> Result<String, JsValue> {
        Ok(state_hash(&self.state(state)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACTIONS: [&str; 7] = [
        "Pacific",
        "Cascadia",
        "FrontRange",
        "Texas",
        "Dreamland",
        "GrainBelt",
        "Beehive",
    ];

    fn flavor() -> FlavorDef {
        ron::from_str(include_str!("../../../content/flavor.ron")).expect("flavor.ron parses")
    }

    /// Every `{placeholder}` in `line` must be one of `allowed`.
    fn check_placeholders(section: &str, line: &str, allowed: &[&str]) {
        let mut rest = line;
        while let Some(open) = rest.find('{') {
            let close = rest[open..]
                .find('}')
                .unwrap_or_else(|| panic!("{section}: unclosed {{ in {line:?}"));
            let name = &rest[open + 1..open + close];
            assert!(
                allowed.contains(&name),
                "{section}: unknown placeholder {{{name}}} in {line:?}"
            );
            rest = &rest[open + close + 1..];
        }
    }

    fn check_pool(section: &str, lines: &[String], min: usize, max_len: usize, allowed: &[&str]) {
        assert!(
            lines.len() >= min,
            "{section} needs at least {min} lines, has {}",
            lines.len()
        );
        for l in lines {
            assert!(l.chars().count() <= max_len, "{section} too long: {l:?}");
            assert!(
                !l.contains(": ") || l.starts_with("Griffin:"),
                "{section} reads like a title: {l:?}"
            );
            check_placeholders(section, l, allowed);
        }
        let mut seen = std::collections::BTreeSet::new();
        for l in lines {
            assert!(seen.insert(l), "{section} repeats {l:?}");
        }
    }

    #[test]
    fn flavor_is_complete_and_well_formed() {
        let f = flavor();
        for fac in FACTIONS {
            let l = f
                .leaders
                .get(fac)
                .unwrap_or_else(|| panic!("leaders.{fac}"));
            let pools = [
                ("intro", &l.intro, 2),
                ("dawn", &l.dawn, 8),
                ("ahead", &l.ahead, 4),
                ("behind", &l.behind, 4),
                ("hit", &l.hit, 4),
                ("landed", &l.landed, 4),
                ("park", &l.park, 2),
                ("back", &l.back, 2),
                ("fetch", &l.fetch, 2),
                ("win", &l.win, 2),
                ("lose", &l.lose, 2),
            ];
            for (k, lines, min) in pools {
                check_pool(
                    &format!("leaders.{fac}.{k}"),
                    lines,
                    min,
                    110,
                    &["rival", "site"],
                );
            }
            let r = f.radio.get(fac).unwrap_or_else(|| panic!("radio.{fac}"));
            for (k, lines) in [
                ("intercept", &r.intercept),
                ("jumped", &r.jumped),
                ("hit", &r.hit),
                ("miss", &r.miss),
                ("prize", &r.prize),
            ] {
                check_pool(
                    &format!("radio.{fac}.{k}"),
                    lines,
                    4,
                    80,
                    &["rival", "site", "prize"],
                );
            }
        }
        for k in [
            "first", "strike", "hq", "jewel", "wiped", "prize", "quiet", "park", "fetch", "win",
        ] {
            let lines = f
                .headlines
                .get(k)
                .unwrap_or_else(|| panic!("headlines.{k}"));
            check_pool(
                &format!("headlines.{k}"),
                lines,
                4,
                72,
                &["attacker", "victim", "site", "prize", "n"],
            );
        }
    }

    #[test]
    fn engine_round_trip() {
        let t = flyover_core::testkit::files();
        let e = Engine::new(
            t.map,
            t.factions,
            t.rules,
            include_str!("../../../content/flavor.ron"),
        )
        .expect("engine");
        let st = e
            .new_game(r#"{"seed":3,"factions":["Pacific","Texas"]}"#)
            .expect("new game");
        let plan = e.suggest(&st, 0, "normal").expect("suggest");
        let checked = e.check(&st, &plan).expect("check");
        assert!(checked.contains("\"preview\""));
        let plan1 = e.suggest(&st, 1, "normal").expect("suggest");
        let out = e
            .resolve(&st, &format!("[{plan},{plan1}]"))
            .expect("resolve");
        assert!(out.contains("\"Dawn\""));
    }
}
