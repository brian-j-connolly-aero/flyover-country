//! Browser bindings. The frontend fetches the RON content files, hands them to
//! [`Engine::new`], and from then on passes game state and orders as JSON.
//! The engine is stateless apart from a one-entry parse cache.

use flyover_ai::{advisor, analysis, plan, risk, view, Level};
use flyover_core::content::{Content, ContentFiles, World, P};
use flyover_core::route::Planner;
use flyover_core::state::{GameState, Mission, Orders};
use flyover_core::{new_game, resolve_night, rules, state_hash, GameConfig};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AceDef {
    pub callsign: String,
    pub launch: Vec<String>,
    pub kill: Vec<String>,
    pub duel: Vec<String>,
    pub downed: Vec<String>,
    pub returns: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaderDef {
    pub name: String,
    pub title: String,
    pub bio: String,
    pub pick: Vec<String>,
    pub dawn: Vec<String>,
    pub winning: Vec<String>,
    pub losing: Vec<String>,
    pub our_jewel_hit: Vec<String>,
    pub victory: Vec<String>,
    pub defeat: Vec<String>,
}

/// Radio chatter, ace lines, leader lines, advisor text and headlines.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlavorDef {
    pub chatter: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub aces: BTreeMap<String, AceDef>,
    pub leaders: BTreeMap<String, LeaderDef>,
    pub advisor: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub headlines: BTreeMap<String, Vec<String>>,
    pub curtain: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize)]
struct ContentOut<'a> {
    width: i32,
    height: i32,
    nodes: &'a [flyover_core::content::Node],
    zones: &'a [flyover_core::content::Zone],
    links: &'a [flyover_core::content::Link],
    fringe: &'a [Vec<P>],
    mississippi: &'a [P],
    units: &'a [flyover_core::content::UnitDef],
    factions: &'a [flyover_core::content::FactionDef],
    rules: &'a flyover_core::content::RulesDef,
    patch_notes: &'a [flyover_core::content::PatchNote],
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

#[derive(Deserialize)]
struct PreviewReq {
    sq: u32,
    mission: Mission,
    #[serde(default)]
    waypoint: Option<P>,
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

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(map: &str, units: &str, factions: &str, rules: &str, patch_notes: &str, flavor: &str) -> Result<Engine, JsValue> {
        let content = Content::parse(&ContentFiles { map, units, factions, rules, patch_notes }).map_err(err)?;
        let world = World::new(content).map_err(err)?;
        let flavor: FlavorDef = ron::de::from_str(flavor).map_err(|e| err(format!("flavor.ron: {e}")))?;
        Ok(Engine { world, flavor, cache: RefCell::new(None) })
    }

    /// Everything the renderer needs to draw the map and write the words.
    pub fn content_json(&self) -> Result<String, JsValue> {
        let w = &self.world;
        let c = &w.content;
        to_json(&ContentOut {
            width: c.map.width,
            height: c.map.height,
            nodes: &w.nodes,
            zones: &w.zones,
            links: &w.links,
            fringe: &c.map.fringe,
            mississippi: &c.map.mississippi,
            units: &c.units,
            factions: &c.factions,
            rules: &c.rules,
            patch_notes: &c.patch_notes,
            flavor: &self.flavor,
        })
    }

    pub fn new_game(&self, config: &str) -> Result<String, JsValue> {
        let cfg: GameConfig = serde_json::from_str(config).map_err(err)?;
        to_json(&new_game(&self.world, &cfg).map_err(err)?)
    }

    /// Resolve a night. Returns {"state": ..., "events": [...], "hash": "..."}.
    pub fn resolve(&self, state: &str, orders: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let orders: Vec<Orders> = serde_json::from_str(orders).map_err(err)?;
        let (next, events) = resolve_night(&self.world, &st, &orders);
        let hash = state_hash(&next);
        to_json(&serde_json::json!({ "state": next, "events": events, "hash": hash }))
    }

    pub fn seat_view(&self, state: &str, seat: u8) -> Result<String, JsValue> {
        to_json(&view::seat_view(&self.world, &self.state(state)?, seat))
    }

    pub fn public_view(&self, state: &str) -> Result<String, JsValue> {
        to_json(&view::public_view(&self.world, &self.state(state)?))
    }

    /// Route, Fuel and risk for every flying mission in a draft.
    pub fn previews(&self, state: &str, orders: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let o: Orders = serde_json::from_str(orders).map_err(err)?;
        let planner = Planner::for_seat(&self.world, &st, o.seat);
        let mut out = Vec::new();
        for so in &o.missions {
            let Some(sq) = st.squadrons.iter().find(|s| s.id == so.sq) else { continue };
            if let Some(p) = risk::preview_with(&self.world, &st, &o, sq, &so.mission, so.waypoint, &planner) {
                out.push(p);
            }
        }
        to_json(&out)
    }

    /// One mission preview against a draft (for drag feedback).
    pub fn preview(&self, state: &str, orders: &str, req: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let o: Orders = serde_json::from_str(orders).map_err(err)?;
        let r: PreviewReq = serde_json::from_str(req).map_err(err)?;
        let Some(sq) = st.squadrons.iter().find(|s| s.id == r.sq) else { return Ok("null".into()) };
        let check = rules::mission_check(&self.world, &st, sq, &r.mission).err();
        let p = risk::preview(&self.world, &st, &o, sq, &r.mission, r.waypoint);
        to_json(&serde_json::json!({ "preview": p, "error": check }))
    }

    pub fn check_mission(&self, state: &str, sq: u32, mission: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let m: Mission = serde_json::from_str(mission).map_err(err)?;
        let Some(s) = st.squadrons.iter().find(|s| s.id == sq) else { return to_json(&Some("No such squadron.")) };
        to_json(&rules::mission_check(&self.world, &st, s, &m).err())
    }

    pub fn advise(&self, state: &str, seat: u8, orders: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let o: Orders = serde_json::from_str(orders).map_err(err)?;
        to_json(&advisor::advise(&self.world, &st, seat, &o))
    }

    pub fn suggest(&self, state: &str, seat: u8, level: &str) -> Result<String, JsValue> {
        let st = self.state(state)?;
        let level = if level.eq_ignore_ascii_case("easy") { Level::Easy } else { Level::Normal };
        to_json(&plan::suggest(&self.world, &st, seat, level))
    }

    pub fn carry_over(&self, state: &str, seat: u8) -> Result<String, JsValue> {
        to_json(&plan::carry_over(&self.world, &self.state(state)?, seat))
    }

    pub fn weak_spot(&self, state: &str, viewer: u8, target: u8) -> Result<String, JsValue> {
        to_json(&analysis::weak_spot(&self.world, &self.state(state)?, viewer, target))
    }

    pub fn best_targets(&self, state: &str, seat: u8) -> Result<String, JsValue> {
        to_json(&analysis::best_targets(&self.world, &self.state(state)?, seat, 6))
    }

    pub fn state_hash(&self, state: &str) -> Result<String, JsValue> {
        Ok(state_hash(&self.state(state)?))
    }
}
