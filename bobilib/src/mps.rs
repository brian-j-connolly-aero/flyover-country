//! Readers for BOBILib's instance pair: an MPS file (the whole problem, as a
//! single-level MIP) and a name-based AUX file (which variables, rows and
//! objective belong to the follower). The MPS reader takes free-format files
//! whose names contain no spaces, which is what BOBILib ships.

use crate::model::{Bilevel, Row, Var};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

fn open(path: &Path) -> Result<Box<dyn BufRead>> {
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let r: Box<dyn Read> = if path.extension().is_some_and(|e| e == "gz") {
        Box::new(flate2::read::MultiGzDecoder::new(f))
    } else {
        Box::new(f)
    };
    Ok(Box::new(BufReader::with_capacity(1 << 20, r)))
}

fn num(s: &str, line: usize) -> Result<f64> {
    match s {
        "Inf" | "inf" | "Infinity" | "infinity" | "+Inf" | "+inf" | "1e+30" | "1e30" => {
            Ok(f64::INFINITY)
        }
        "-Inf" | "-inf" | "-Infinity" | "-infinity" | "-1e+30" | "-1e30" => Ok(f64::NEG_INFINITY),
        _ => s
            .parse::<f64>()
            .map_err(|_| format!("line {line}: bad number {s:?}")),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    N,
    L,
    G,
    E,
}

/// Read an MPS file into a [`Bilevel`] with every variable and row on the
/// upper level. [`apply_aux`] then moves the follower's part down.
pub fn read_mps(path: &Path) -> Result<Bilevel> {
    let reader = open(path)?;
    let mut m = Bilevel::default();
    let mut section = String::new();
    let mut obj_row: Option<String> = None;
    let mut row_of: HashMap<String, usize> = HashMap::new();
    let mut kinds: Vec<Kind> = Vec::new();
    let mut col_of: HashMap<String, usize> = HashMap::new();
    let mut in_int = false;
    let mut maximize = false;
    let mut explicit_bounds: Vec<bool> = Vec::new();
    let mut ranges: Vec<(usize, f64)> = Vec::new();
    let mut rhs: Vec<f64> = Vec::new();

    for (ln, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let ln = ln + 1;
        if line.starts_with('*') || line.trim().is_empty() {
            continue;
        }
        let t: Vec<&str> = line.split_whitespace().collect();
        if !line.starts_with(' ') && !line.starts_with('\t') {
            section = t[0].to_ascii_uppercase();
            match section.as_str() {
                "NAME" => m.name = t.get(1).unwrap_or(&"").to_string(),
                "OBJSENSE" => {
                    if let Some(s) = t.get(1) {
                        maximize =
                            s.eq_ignore_ascii_case("MAX") || s.eq_ignore_ascii_case("MAXIMIZE");
                    }
                }
                "ROWS" | "COLUMNS" | "RHS" | "RANGES" | "BOUNDS" | "ENDATA" => {}
                "OBJSENSE_MAX" => maximize = true,
                other => return Err(format!("line {ln}: unsupported MPS section {other}")),
            }
            continue;
        }
        match section.as_str() {
            "OBJSENSE" => {
                maximize = t[0].eq_ignore_ascii_case("MAX") || t[0].eq_ignore_ascii_case("MAXIMIZE")
            }
            "ROWS" => {
                let kind = match t[0] {
                    "N" | "n" => Kind::N,
                    "L" | "l" => Kind::L,
                    "G" | "g" => Kind::G,
                    "E" | "e" => Kind::E,
                    k => return Err(format!("line {ln}: bad row type {k}")),
                };
                let name = t[1].to_string();
                if kind == Kind::N {
                    if obj_row.is_none() {
                        obj_row = Some(name);
                    }
                    continue;
                }
                row_of.insert(name.clone(), m.rows.len());
                kinds.push(kind);
                rhs.push(0.0);
                m.rows.push(Row {
                    name,
                    lo: f64::NEG_INFINITY,
                    hi: f64::INFINITY,
                    lower: false,
                    coefs: Vec::new(),
                });
            }
            "COLUMNS" => {
                if t.len() >= 3 && t[1].trim_matches('\'') == "MARKER" {
                    let what = t[2].trim_matches('\'');
                    in_int = what == "INTORG";
                    if what != "INTORG" && what != "INTEND" {
                        return Err(format!("line {ln}: bad marker {what}"));
                    }
                    continue;
                }
                let j = match col_of.get(t[0]) {
                    Some(&j) => j,
                    None => {
                        let j = m.vars.len();
                        col_of.insert(t[0].to_string(), j);
                        m.vars.push(Var {
                            name: t[0].to_string(),
                            lb: 0.0,
                            ub: f64::INFINITY,
                            integer: in_int,
                            obj: 0.0,
                            lower: false,
                            ll_obj: 0.0,
                        });
                        explicit_bounds.push(false);
                        j
                    }
                };
                for pair in t[1..].chunks(2) {
                    if pair.len() < 2 {
                        return Err(format!("line {ln}: odd COLUMNS entry"));
                    }
                    let v = num(pair[1], ln)?;
                    if Some(pair[0]) == obj_row.as_deref() {
                        m.vars[j].obj += v;
                    } else if let Some(&i) = row_of.get(pair[0]) {
                        if v != 0.0 {
                            m.rows[i].coefs.push((j, v));
                        }
                    } else {
                        // A second N row: ignored, as other readers do.
                    }
                }
            }
            "RHS" | "RANGES" => {
                let rest = if t.len() % 2 == 1 { &t[1..] } else { &t[..] };
                for pair in rest.chunks(2) {
                    let v = num(pair[1], ln)?;
                    if section == "RHS" {
                        if Some(pair[0]) == obj_row.as_deref() {
                            m.obj_offset = -v;
                        } else if let Some(&i) = row_of.get(pair[0]) {
                            rhs[i] = v;
                        } else {
                            return Err(format!("line {ln}: RHS for unknown row {}", pair[0]));
                        }
                    } else if let Some(&i) = row_of.get(pair[0]) {
                        ranges.push((i, v));
                    } else {
                        return Err(format!("line {ln}: RANGES for unknown row {}", pair[0]));
                    }
                }
            }
            "BOUNDS" => {
                let ty = t[0].to_ascii_uppercase();
                let needs_value = !matches!(ty.as_str(), "FR" | "MI" | "PL" | "BV");
                // Forms: TYPE BND COL [VAL] or TYPE COL [VAL] (no bound-set name).
                let (col, val) = match (needs_value, t.len()) {
                    (true, 4) => (t[2], Some(num(t[3], ln)?)),
                    (true, 3) => (t[1], Some(num(t[2], ln)?)),
                    (false, 3) => (t[2], None),
                    (false, 4) => (t[2], Some(num(t[3], ln)?)),
                    (false, 2) => (t[1], None),
                    _ => return Err(format!("line {ln}: bad BOUNDS line")),
                };
                let j = *col_of
                    .get(col)
                    .ok_or_else(|| format!("line {ln}: bound for unknown column {col}"))?;
                explicit_bounds[j] = true;
                let v = &mut m.vars[j];
                match ty.as_str() {
                    "UP" => {
                        let u = val.unwrap();
                        if u < 0.0 && v.lb == 0.0 {
                            v.lb = f64::NEG_INFINITY;
                        }
                        v.ub = u;
                    }
                    "LO" => v.lb = val.unwrap(),
                    "FX" => {
                        v.lb = val.unwrap();
                        v.ub = val.unwrap();
                    }
                    "FR" => {
                        v.lb = f64::NEG_INFINITY;
                        v.ub = f64::INFINITY;
                    }
                    "MI" => v.lb = f64::NEG_INFINITY,
                    "PL" => v.ub = f64::INFINITY,
                    "BV" => {
                        v.lb = 0.0;
                        v.ub = 1.0;
                        v.integer = true;
                    }
                    "LI" => {
                        v.lb = val.unwrap();
                        v.integer = true;
                    }
                    "UI" => {
                        let u = val.unwrap();
                        if u < 0.0 && v.lb == 0.0 {
                            v.lb = f64::NEG_INFINITY;
                        }
                        v.ub = u;
                        v.integer = true;
                    }
                    _ => return Err(format!("line {ln}: unsupported bound type {ty}")),
                }
            }
            "ENDATA" => break,
            _ => return Err(format!("line {ln}: data outside a section")),
        }
    }

    for (i, r) in m.rows.iter_mut().enumerate() {
        match kinds[i] {
            Kind::L => r.hi = rhs[i],
            Kind::G => r.lo = rhs[i],
            Kind::E => {
                r.lo = rhs[i];
                r.hi = rhs[i];
            }
            Kind::N => unreachable!(),
        }
    }
    for (i, range) in ranges {
        let b = rhs[i];
        let r = &mut m.rows[i];
        match kinds[i] {
            Kind::L => r.lo = b - range.abs(),
            Kind::G => r.hi = b + range.abs(),
            Kind::E if range >= 0.0 => r.hi = b + range,
            Kind::E => r.lo = b + range,
            Kind::N => {}
        }
    }
    if maximize {
        for v in &mut m.vars {
            v.obj = -v.obj;
        }
        m.obj_offset = -m.obj_offset;
    }
    Ok(m)
}

/// Mark the follower's variables, objective and rows from a name-based AUX file.
pub fn apply_aux(m: &mut Bilevel, path: &Path) -> Result<()> {
    let reader = open(path)?;
    let cols: HashMap<String, usize> = m
        .vars
        .iter()
        .enumerate()
        .map(|(i, v)| (v.name.clone(), i))
        .collect();
    let rows: HashMap<String, usize> = m
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| (r.name.clone(), i))
        .collect();
    let mut block = String::new();
    let (mut nv, mut nc) = (None, None);
    let (mut seen_v, mut seen_c) = (0usize, 0usize);
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let t: Vec<&str> = line.split_whitespace().collect();
        if t.is_empty() {
            continue;
        }
        if t[0].starts_with('@') {
            block = t[0].to_string();
            let inline = t.get(1).map(|s| s.to_string());
            if block == "@NUMVARS" || block == "@NUMCONSTRS" {
                if let Some(p) = inline {
                    let n = p.parse::<usize>().map_err(|e| e.to_string())?;
                    if block == "@NUMVARS" {
                        nv = Some(n)
                    } else {
                        nc = Some(n)
                    }
                }
            }
            continue;
        }
        match block.as_str() {
            "@NUMVARS" => nv = Some(t[0].parse().map_err(|_| "bad @NUMVARS".to_string())?),
            "@NUMCONSTRS" => nc = Some(t[0].parse().map_err(|_| "bad @NUMCONSTRS".to_string())?),
            "@VARSBEGIN" => {
                let j = *cols
                    .get(t[0])
                    .ok_or_else(|| format!("aux: unknown variable {}", t[0]))?;
                let c = if t.len() > 1 { num(t[1], 0)? } else { 0.0 };
                m.vars[j].lower = true;
                m.vars[j].ll_obj = c;
                seen_v += 1;
            }
            "@CONSTRSBEGIN" => {
                let i = *rows
                    .get(t[0])
                    .ok_or_else(|| format!("aux: unknown row {}", t[0]))?;
                m.rows[i].lower = true;
                seen_c += 1;
            }
            _ => {}
        }
    }
    if nv.is_some_and(|n| n != seen_v) || nc.is_some_and(|n| n != seen_c) {
        return Err(format!(
            "aux: counts disagree (vars {seen_v} vs {nv:?}, rows {seen_c} vs {nc:?})"
        ));
    }
    Ok(())
}

/// Read an instance from `<stem>.mps(.gz)` and `<stem>.aux`.
pub fn read_instance(mps: &Path) -> Result<Bilevel> {
    let mut m = read_mps(mps)?;
    let s = mps.to_string_lossy();
    let stem = s
        .strip_suffix(".mps.gz")
        .or_else(|| s.strip_suffix(".mps"))
        .ok_or_else(|| format!("{s}: expected .mps or .mps.gz"))?;
    apply_aux(&mut m, Path::new(&format!("{stem}.aux")))?;
    if m.name.is_empty() {
        m.name = Path::new(stem)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
    }
    Ok(m)
}
