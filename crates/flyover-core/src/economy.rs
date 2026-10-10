//! Supply network: income is the maximum flow from producers to the capital
//! through undamaged nodes and links. Players never see the word "max-flow";
//! they see thick lines and red breaks.

use crate::content::{FactionId, LinkKind, NodeKind, World};
use crate::rules;
use crate::state::{EffectKind, GameState};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Commodity {
    Supply,
    Fuel,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Income {
    pub supply: i32,
    pub fuel: i32,
    /// Production before the network takes its cut.
    pub raw_supply: i32,
    pub raw_fuel: i32,
    /// (link, supply flow, fuel flow) for links carrying anything.
    pub links: Vec<(u16, i32, i32)>,
}

const BIG: i32 = 1_000;

struct Graph {
    // edges: to, cap, rev
    adj: Vec<Vec<usize>>,
    to: Vec<usize>,
    cap: Vec<i32>,
}

impl Graph {
    fn new(n: usize) -> Graph {
        Graph { adj: vec![Vec::new(); n], to: Vec::new(), cap: Vec::new() }
    }
    fn add(&mut self, a: usize, b: usize, c: i32) -> usize {
        let e = self.to.len();
        self.to.push(b);
        self.cap.push(c);
        self.adj[a].push(e);
        self.to.push(a);
        self.cap.push(0);
        self.adj[b].push(e + 1);
        e
    }
    /// Edmonds-Karp. Deterministic: BFS order follows insertion order.
    fn max_flow(&mut self, s: usize, t: usize) -> i32 {
        let n = self.adj.len();
        let mut total = 0;
        loop {
            let mut prev = vec![usize::MAX; n];
            let mut q = std::collections::VecDeque::new();
            q.push_back(s);
            prev[s] = usize::MAX - 1;
            while let Some(u) = q.pop_front() {
                if u == t {
                    break;
                }
                for &e in &self.adj[u] {
                    let v = self.to[e];
                    if self.cap[e] > 0 && prev[v] == usize::MAX {
                        prev[v] = e;
                        q.push_back(v);
                    }
                }
            }
            if prev[t] == usize::MAX {
                return total;
            }
            let mut f = i32::MAX;
            let mut v = t;
            while v != s {
                let e = prev[v];
                f = f.min(self.cap[e]);
                v = self.to[e ^ 1];
            }
            let mut v = t;
            while v != s {
                let e = prev[v];
                self.cap[e] -= f;
                self.cap[e ^ 1] += f;
                v = self.to[e ^ 1];
            }
            total += f;
        }
    }
}

/// Production of one node at a given health for its holder.
pub fn production(world: &World, state: &GameState, node: u16, health: u8, c: Commodity) -> i32 {
    let n = &world.nodes[node as usize];
    let e = &world.rules().economy;
    let scale = |v: i32| v * health as i32 / 2;
    match (n.kind, c) {
        (NodeKind::City, Commodity::Supply) => scale(e.city_supply),
        // A captured fallen capital works like a city.
        (NodeKind::Capital, Commodity::Supply) if n.home.map(|f| !rules::is_seated(state, f)).unwrap_or(false) => {
            scale(e.city_supply)
        }
        (NodeKind::Fuel, Commodity::Fuel) => match n.home {
            Some(FactionId::FrontRange) => scale(e.brewery),
            Some(FactionId::Texas) => scale(e.fuel_site * e.texas_fuel_mult),
            _ => scale(e.fuel_site),
        },
        _ => 0,
    }
}

fn throughput(world: &World, health: u8) -> i32 {
    match health {
        0 => 0,
        1 => world.rules().economy.half_node_throughput,
        _ => BIG,
    }
}

fn link_cap(world: &World, k: LinkKind, c: Commodity) -> i32 {
    let e = &world.rules().economy;
    match (k, c) {
        (LinkKind::Highway, _) => e.highway,
        (LinkKind::Rail, _) => e.rail,
        (LinkKind::Pipeline, Commodity::Fuel) => e.pipeline,
        (LinkKind::Pipeline, Commodity::Supply) => 0,
    }
}

/// Flow of one commodity to the capital. Returns (flow, per-link flow).
fn flow(world: &World, state: &GameState, seat: u8, health: &[u8], c: Commodity) -> (i32, i32, Vec<(u16, i32)>) {
    let f = rules::faction(state, seat);
    let capital = world.capitals[f.index()] as usize;
    let n = world.nodes.len();
    // node v: in = 2v, out = 2v+1; source = 2n, sink = capital in.
    let s = 2 * n;
    let mut g = Graph::new(2 * n + 1);
    let owned = |v: usize| state.nodes[v].owner == Some(seat) && world.nodes[v].kind.on_network();
    let mut raw = 0;
    for v in 0..n {
        if !owned(v) || v == capital {
            continue;
        }
        g.add(2 * v, 2 * v + 1, throughput(world, health[v]));
        let p = production(world, state, v as u16, health[v], c);
        if p > 0 {
            raw += p;
            g.add(s, 2 * v + 1, p);
        }
    }
    let mut link_edges = Vec::new();
    for l in &world.links {
        let (a, b) = (l.a as usize, l.b as usize);
        if !owned(a) || !owned(b) {
            continue;
        }
        let cap = link_cap(world, l.kind, c);
        if cap == 0 {
            continue;
        }
        // out -> in in both directions; the capital is the sink (its "in").
        let ea = g.add(2 * a + 1, 2 * b, cap);
        let eb = g.add(2 * b + 1, 2 * a, cap);
        link_edges.push((l.idx, ea, eb, cap));
    }
    let total = g.max_flow(s, 2 * capital);
    let mut per_link = Vec::new();
    for (idx, ea, eb, cap) in link_edges {
        let fa = cap - g.cap[ea];
        let fb = cap - g.cap[eb];
        let net = (fa - fb).abs();
        if net > 0 {
            per_link.push((idx, net));
        }
    }
    (total, raw, per_link)
}

/// Tonight's income for a seat given node health (use `state.nodes` health by
/// default, or a hypothetical for the Weak Spot view). `night` selects which
/// crown-jewel effects apply.
pub fn income_with(world: &World, state: &GameState, seat: u8, health: &[u8], night: u8) -> Income {
    let f = rules::faction(state, seat);
    let e = &world.rules().economy;
    let (mut supply, raw_s, ls) = flow(world, state, seat, health, Commodity::Supply);
    let (mut fuel, raw_f, lf) = flow(world, state, seat, health, Commodity::Fuel);
    supply += e.capital_supply;
    fuel += e.capital_fuel;
    if f == FactionId::Dreamland {
        supply += e.strip_supply_bonus;
    }
    // Truck stops are fuel caches: no network needed.
    for n in &world.nodes {
        if n.kind == NodeKind::TruckStop && state.nodes[n.idx as usize].owner == Some(seat) {
            fuel += e.truck_stop_fuel;
        }
    }
    if f == FactionId::FrontRange {
        fuel = fuel.min(e.brewery_cap);
    }
    if rules::effect_active(state, seat, EffectKind::StripDark, night) {
        supply = 0;
    }
    if rules::effect_active(state, seat, EffectKind::FuelHalf, night) {
        fuel /= 2;
    }
    let mut links: Vec<(u16, i32, i32)> = Vec::new();
    for (l, v) in ls {
        links.push((l, v, 0));
    }
    for (l, v) in lf {
        match links.iter_mut().find(|x| x.0 == l) {
            Some(x) => x.2 = v,
            None => links.push((l, 0, v)),
        }
    }
    links.sort_by_key(|x| x.0);
    Income { supply, fuel, raw_supply: raw_s + e.capital_supply, raw_fuel: raw_f + e.capital_fuel, links }
}

pub fn income(world: &World, state: &GameState, seat: u8, night: u8) -> Income {
    let health: Vec<u8> = state.nodes.iter().map(|n| n.health).collect();
    income_with(world, state, seat, &health, night)
}

/// Stockpile cap: two nights of income, with a floor.
pub fn cap(world: &World, income: i32) -> i32 {
    let e = &world.rules().economy;
    (income * e.stockpile_nights).max(e.stockpile_floor)
}
