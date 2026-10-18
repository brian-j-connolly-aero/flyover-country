//! BOBILib's solution file format (`<name>.res.json`).

use crate::model::Bilevel;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolutionFile {
    pub name: String,
    #[serde(default = "optimistic")]
    pub bilevel_type: String,
    pub status: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    pub objective_value: Option<f64>,
    #[serde(default)]
    pub upper_level_decisions: Map<String, Value>,
    #[serde(default)]
    pub lower_level_decisions: Map<String, Value>,
}

fn optimistic() -> String {
    "optimistic".into()
}

impl SolutionFile {
    pub fn read(path: &Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn from_values(m: &Bilevel, status: &str, val: &[f64]) -> Self {
        let mut up = Map::new();
        let mut lo = Map::new();
        for (v, &x) in m.vars.iter().zip(val) {
            let x = if v.integer { x.round() } else { x };
            let entry = (v.name.clone(), Value::from(x));
            if v.lower {
                lo.insert(entry.0, entry.1)
            } else {
                up.insert(entry.0, entry.1)
            };
        }
        SolutionFile {
            name: m.name.clone(),
            bilevel_type: optimistic(),
            status: Some(status.into()),
            difficulty: None,
            objective_value: Some(m.upper_obj(val)),
            upper_level_decisions: up,
            lower_level_decisions: lo,
        }
    }

    /// Values in model order; variables missing from the file are zero.
    pub fn values(&self, m: &Bilevel) -> Vec<f64> {
        m.vars
            .iter()
            .map(|v| {
                let map = if v.lower {
                    &self.lower_level_decisions
                } else {
                    &self.upper_level_decisions
                };
                map.get(&v.name).and_then(Value::as_f64).unwrap_or(0.0)
            })
            .collect()
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
    }
}
