//! A mixed-integer bilevel linear problem (MIBLP) as BOBILib defines it:
//!
//! ```text
//! min  c_u x + d_u y            (upper level, the leader)
//! s.t. upper-level rows
//!      y in argmin { e y : lower-level rows }   (lower level, the follower)
//! ```
//!
//! Rows are stored as ranges `lo <= a x <= hi`, which covers L, G, E and
//! ranged rows alike.

#[derive(Clone, Debug)]
pub struct Var {
    pub name: String,
    pub lb: f64,
    pub ub: f64,
    pub integer: bool,
    /// Upper-level objective coefficient.
    pub obj: f64,
    /// True when the follower controls this variable.
    pub lower: bool,
    /// Lower-level objective coefficient (zero for leader variables).
    pub ll_obj: f64,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub name: String,
    pub lo: f64,
    pub hi: f64,
    /// True for a lower-level (follower) row.
    pub lower: bool,
    pub coefs: Vec<(usize, f64)>,
}

#[derive(Clone, Debug, Default)]
pub struct Bilevel {
    pub name: String,
    pub vars: Vec<Var>,
    pub rows: Vec<Row>,
    /// Constant added to the upper-level objective.
    pub obj_offset: f64,
}

impl Bilevel {
    pub fn var_index(&self) -> std::collections::HashMap<&str, usize> {
        self.vars
            .iter()
            .enumerate()
            .map(|(i, v)| (v.name.as_str(), i))
            .collect()
    }

    pub fn upper_obj(&self, val: &[f64]) -> f64 {
        self.obj_offset
            + self
                .vars
                .iter()
                .zip(val)
                .map(|(v, x)| v.obj * x)
                .sum::<f64>()
    }

    pub fn lower_obj(&self, val: &[f64]) -> f64 {
        self.vars
            .iter()
            .zip(val)
            .map(|(v, x)| v.ll_obj * x)
            .sum::<f64>()
    }

    /// Largest violation of bounds, integrality and rows (absolute).
    pub fn max_violation(&self, val: &[f64]) -> (f64, String) {
        let mut worst = (0.0, String::new());
        let mut note = |amount: f64, what: &dyn Fn() -> String| {
            if amount > worst.0 {
                worst = (amount, what());
            }
        };
        for (v, &x) in self.vars.iter().zip(val) {
            note(v.lb - x, &|| format!("lower bound of {}", v.name));
            note(x - v.ub, &|| format!("upper bound of {}", v.name));
            if v.integer {
                note((x - x.round()).abs(), &|| {
                    format!("integrality of {}", v.name)
                });
            }
        }
        for r in &self.rows {
            let a: f64 = r.coefs.iter().map(|&(j, c)| c * val[j]).sum();
            note(r.lo - a, &|| format!("row {} (>=)", r.name));
            note(a - r.hi, &|| format!("row {} (<=)", r.name));
        }
        worst
    }

    pub fn summary(&self) -> String {
        let nl = self.vars.iter().filter(|v| v.lower).count();
        let ml = self.rows.iter().filter(|r| r.lower).count();
        format!(
            "{}: {} vars ({} leader, {} follower), {} rows ({} leader, {} follower)",
            self.name,
            self.vars.len(),
            self.vars.len() - nl,
            nl,
            self.rows.len(),
            self.rows.len() - ml,
            ml
        )
    }
}
