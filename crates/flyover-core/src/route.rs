//! Routing: missions plan themselves around known air defense with A*
//! ("A-star") over a threat-weighted grid, then get smoothed to a few legs.
//! A player-dragged waypoint overrides the plan with straight legs.

use crate::content::{dist, dist2, NodeKind, World, P};
use crate::rules;
use crate::state::GameState;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const CELL: i32 = 24;

#[derive(Clone, Debug)]
pub struct Threat {
    pub pos: P,
    pub radius: i32,
    /// Known health (1 or 2).
    pub weight: i32,
}

/// Enemy air defense sites this seat knows about.
pub fn threats_for(world: &World, state: &GameState, seat: u8) -> Vec<Threat> {
    let mut out = Vec::new();
    let ad = &world.rules().air_defense;
    let known = state.intel.get(seat as usize).map(|i| &i.known);
    for n in &world.nodes {
        if n.kind != NodeKind::AirDefense {
            continue;
        }
        let ns = &state.nodes[n.idx as usize];
        let Some(owner) = ns.owner else { continue };
        if owner == seat {
            continue;
        }
        let h = known
            .and_then(|k| k.get(n.idx as usize).copied())
            .unwrap_or(ns.health);
        if h == 0 {
            continue;
        }
        let radius = if rules::faction(state, owner) == crate::content::FactionId::GrainBelt {
            ad.grainbelt_range
        } else {
            ad.range
        };
        out.push(Threat {
            pos: n.pos,
            radius,
            weight: h as i32,
        });
    }
    out
}

fn penalty(threats: &[Threat], p: P) -> i32 {
    let mut pen = 0;
    for t in threats {
        let r = t.radius + CELL / 2;
        if dist2(p, t.pos) <= (r as i64) * (r as i64) {
            pen += 60 * t.weight;
        }
    }
    pen
}

/// Highest threat penalty along a straight segment (sampled).
fn segment_penalty(threats: &[Threat], a: P, b: P) -> i32 {
    let len = dist(a, b).max(1);
    let steps = (len / 8).max(1);
    let mut worst = 0;
    for i in 0..=steps {
        let p = (a.0 + (b.0 - a.0) * i / steps, a.1 + (b.1 - a.1) * i / steps);
        worst = worst.max(penalty(threats, p));
    }
    worst
}

fn crosses_static(world: &World, a: P, b: P) -> bool {
    let len = dist(a, b).max(1);
    let steps = (len / 12).max(1);
    (0..=steps)
        .any(|i| world.in_static((a.0 + (b.0 - a.0) * i / steps, a.1 + (b.1 - a.1) * i / steps)))
}

/// A cached threat grid for one seat's picture of the sky. Building the grid
/// is the expensive part; planning many routes on it is cheap.
pub struct Planner<'a> {
    world: &'a World,
    pub threats: Vec<Threat>,
    cost: Vec<i32>,
    gw: i32,
    gh: i32,
}

impl<'a> Planner<'a> {
    pub fn new(world: &'a World, threats: Vec<Threat>) -> Planner<'a> {
        let gw = (world.content.map.width + CELL - 1) / CELL + 2;
        let gh = (world.content.map.height + CELL - 1) / CELL + 2;
        let mut cost = vec![0i32; (gw * gh) as usize];
        for y in 0..gh {
            for x in 0..gw {
                let p = (x * CELL + CELL / 2, y * CELL + CELL / 2);
                cost[(y * gw + x) as usize] = if world.in_static(p) {
                    -1
                } else {
                    penalty(&threats, p)
                };
            }
        }
        Planner {
            world,
            threats,
            cost,
            gw,
            gh,
        }
    }

    pub fn for_seat(world: &'a World, state: &GameState, seat: u8) -> Planner<'a> {
        Planner::new(world, threats_for(world, state, seat))
    }

    /// Plan a path from `from` to `to`. Returns the polyline including both ends.
    pub fn plan(&self, from: P, to: P, waypoint: Option<P>) -> Vec<P> {
        let world = self.world;
        let threats = &self.threats;
        if let Some(w) = waypoint {
            return vec![from, w, to];
        }
        let end_pen = penalty(threats, to);
        if segment_penalty(threats, from, to) <= end_pen.max(penalty(threats, from))
            && !crosses_static(world, from, to)
        {
            return vec![from, to];
        }
        let (gw, gh) = (self.gw, self.gh);
        let cell_of =
            |p: P| -> (i32, i32) { ((p.0 / CELL).clamp(0, gw - 1), (p.1 / CELL).clamp(0, gh - 1)) };
        let center = |c: (i32, i32)| -> P { (c.0 * CELL + CELL / 2, c.1 * CELL + CELL / 2) };
        let idx = |c: (i32, i32)| -> usize { (c.1 * gw + c.0) as usize };
        let n = (gw * gh) as usize;
        let s = cell_of(from);
        let t = cell_of(to);
        // Endpoints may sit inside threat rings or the Static; never refuse to route.
        let cost_at = |i: usize| -> i32 {
            if i == idx(s) || i == idx(t) {
                self.cost[i].max(0)
            } else {
                self.cost[i]
            }
        };
        let h = |c: (i32, i32)| -> i32 {
            let dx = (c.0 - t.0).abs();
            let dy = (c.1 - t.1).abs();
            10 * (dx + dy) - 6 * dx.min(dy)
        };
        let mut g = vec![i32::MAX; n];
        let mut prev = vec![usize::MAX; n];
        let mut heap = BinaryHeap::new();
        g[idx(s)] = 0;
        heap.push(Reverse((h(s), idx(s))));
        const DIRS: [(i32, i32, i32); 8] = [
            (1, 0, 10),
            (-1, 0, 10),
            (0, 1, 10),
            (0, -1, 10),
            (1, 1, 14),
            (1, -1, 14),
            (-1, 1, 14),
            (-1, -1, 14),
        ];
        while let Some(Reverse((f, u))) = heap.pop() {
            if u == idx(t) {
                break;
            }
            let ux = (u as i32) % gw;
            let uy = (u as i32) / gw;
            if f > g[u].saturating_add(h((ux, uy))) {
                continue;
            }
            for (dx, dy, step) in DIRS {
                let (vx, vy) = (ux + dx, uy + dy);
                if vx < 0 || vy < 0 || vx >= gw || vy >= gh {
                    continue;
                }
                let v = idx((vx, vy));
                let c = cost_at(v);
                if c < 0 {
                    continue;
                }
                let ng = g[u].saturating_add(step + c);
                if ng < g[v] {
                    g[v] = ng;
                    prev[v] = u;
                    heap.push(Reverse((ng + h((vx, vy)), v)));
                }
            }
        }
        if prev[idx(t)] == usize::MAX && idx(s) != idx(t) {
            return vec![from, to];
        }
        let mut cells = Vec::new();
        let mut u = idx(t);
        while u != usize::MAX {
            cells.push(u);
            if u == idx(s) {
                break;
            }
            u = prev[u];
        }
        cells.reverse();
        let mut pts: Vec<P> = Vec::with_capacity(cells.len() + 2);
        pts.push(from);
        for &c in cells.iter().skip(1).take(cells.len().saturating_sub(2)) {
            pts.push(center(((c as i32) % gw, (c as i32) / gw)));
        }
        pts.push(to);
        smooth(world, threats, &pts)
    }
}

/// One-off route plan (builds a fresh threat grid).
pub fn plan(world: &World, threats: &[Threat], from: P, to: P, waypoint: Option<P>) -> Vec<P> {
    Planner::new(world, threats.to_vec()).plan(from, to, waypoint)
}

/// Greedy string pulling that never makes the threat exposure worse.
fn smooth(world: &World, threats: &[Threat], pts: &[P]) -> Vec<P> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    let mut i = 0;
    while i < pts.len() - 1 {
        let mut best = i + 1;
        let mut j = pts.len() - 1;
        while j > i + 1 {
            let mut worst_path = 0;
            for w in pts[i..=j].windows(2) {
                worst_path = worst_path.max(segment_penalty(threats, w[0], w[1]));
            }
            if segment_penalty(threats, pts[i], pts[j]) <= worst_path
                && !crosses_static(world, pts[i], pts[j])
            {
                best = j;
                break;
            }
            j -= 1;
        }
        out.push(pts[best]);
        i = best;
    }
    out
}

pub fn path_len(path: &[P]) -> i32 {
    path.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// Does a path cross the Static?
pub fn path_in_static(world: &World, path: &[P]) -> bool {
    path.windows(2).any(|w| crosses_static(world, w[0], w[1]))
}

/// Count how many known threat rings a path passes through.
pub fn threats_crossed(threats: &[Threat], path: &[P]) -> Vec<usize> {
    let mut hit = Vec::new();
    for (ti, t) in threats.iter().enumerate() {
        let r2 = (t.radius as i64) * (t.radius as i64);
        let crossed = path.windows(2).any(|w| {
            let len = dist(w[0], w[1]).max(1);
            let steps = (len / 8).max(1);
            (0..=steps).any(|i| {
                let p = (
                    w[0].0 + (w[1].0 - w[0].0) * i / steps,
                    w[0].1 + (w[1].1 - w[0].1) * i / steps,
                );
                dist2(p, t.pos) <= r2
            })
        });
        if crossed {
            hit.push(ti);
        }
    }
    hit
}
