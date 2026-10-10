//! Same seed and orders, same war: byte for byte. Plus the invariants every
//! night must keep.

use flyover_ai::{suggest, Level};
use flyover_core::content::FactionId::*;
use flyover_core::content::{FactionId, LinkKind};
use flyover_core::economy;
use flyover_core::state::Orders;
use flyover_core::*;
use std::collections::BTreeSet;

fn play(seed: u64, factions: Vec<FactionId>) -> (Vec<String>, Vec<Vec<Orders>>) {
    let w = testkit::world();
    let mut st = new_game(&w, &GameConfig { seed, factions }).unwrap();
    let mut hashes = Vec::new();
    let mut all_orders = Vec::new();
    while st.night <= w.rules().nights {
        let orders: Vec<Orders> = (0..4)
            .map(|s| {
                suggest(
                    &w,
                    &st,
                    s,
                    if s % 2 == 0 {
                        Level::Normal
                    } else {
                        Level::Easy
                    },
                )
            })
            .collect();
        let (next, _) = resolve_night(&w, &st, &orders);
        check_invariants(&w, &next);
        st = next;
        hashes.push(state_hash(&st));
        all_orders.push(orders);
    }
    (hashes, all_orders)
}

fn check_invariants(w: &World, st: &GameState) {
    let mut ids = BTreeSet::new();
    for s in &st.seats {
        assert!(s.supply >= 0, "negative supply");
        assert!(s.fuel >= 0, "negative fuel");
    }
    for q in &st.squadrons {
        assert!(ids.insert(q.id), "squadron {} appears twice", q.id);
        assert!(q.strength <= state::MAX_STRENGTH);
        assert!((q.base as usize) < w.nodes.len());
        if q.down_until == 0 {
            assert!(q.strength > 0, "an active squadron with no planes");
        }
    }
    for n in &st.nodes {
        assert!(n.health <= 2);
    }
    let e = &w.rules().economy;
    for seat in 0..st.seats.len() as u8 {
        let inc = economy::income(w, st, seat, st.night);
        for (l, s, f) in inc.links {
            let cap = match w.links[l as usize].kind {
                LinkKind::Highway => e.highway,
                LinkKind::Rail => e.rail,
                LinkKind::Pipeline => e.pipeline,
            };
            assert!(s <= cap && f <= cap, "flow above link capacity");
        }
    }
}

#[test]
fn same_seed_same_war() {
    let a = play(77, vec![Pacific, Cascadia, Dreamland, Beehive]);
    let b = play(77, vec![Pacific, Cascadia, Dreamland, Beehive]);
    assert_eq!(a.0, b.0);
    assert_eq!(a.0.len(), 8);
}

#[test]
fn replaying_orders_reproduces_every_hash() {
    let w = testkit::world();
    let factions = vec![Texas, GrainBelt, FrontRange, Pacific];
    let (hashes, orders) = play(4242, factions.clone());
    // A save is just the seed plus everyone's orders, round-tripped through JSON.
    let json = serde_json::to_string(&orders).unwrap();
    let orders: Vec<Vec<Orders>> = serde_json::from_str(&json).unwrap();
    let mut st = new_game(
        &w,
        &GameConfig {
            seed: 4242,
            factions,
        },
    )
    .unwrap();
    for (i, o) in orders.iter().enumerate() {
        let (next, _) = resolve_night(&w, &st, o);
        st = next;
        assert_eq!(state_hash(&st), hashes[i], "night {} diverged", i + 1);
    }
}

#[test]
fn states_survive_a_json_round_trip() {
    let w = testkit::world();
    let st = new_game(
        &w,
        &GameConfig {
            seed: 9,
            factions: vec![Beehive, Texas, Cascadia, GrainBelt],
        },
    )
    .unwrap();
    let orders: Vec<Orders> = (0..4).map(|s| suggest(&w, &st, s, Level::Normal)).collect();
    let (next, events) = resolve_night(&w, &st, &orders);
    let back: GameState = serde_json::from_str(&serde_json::to_string(&next).unwrap()).unwrap();
    assert_eq!(back, next);
    let ev_json = serde_json::to_string(&events).unwrap();
    let ev_back: Vec<EventRec> = serde_json::from_str(&ev_json).unwrap();
    assert_eq!(ev_back.len(), events.len());
}

#[test]
fn every_lineup_runs_without_trouble() {
    let all = FactionId::ALL;
    let mut n = 0;
    for a in 0..7 {
        for b in a + 1..7 {
            for c in b + 1..7 {
                for d in c + 1..7 {
                    let (h, _) = play(n * 31 + 1, vec![all[a], all[b], all[c], all[d]]);
                    assert_eq!(h.len(), 8);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 35);
}

#[test]
fn suggest_plan_is_fast_enough() {
    let w = testkit::world();
    let st = new_game(
        &w,
        &GameConfig {
            seed: 3,
            factions: vec![Pacific, FrontRange, Texas, Beehive],
        },
    )
    .unwrap();
    let t = std::time::Instant::now();
    for s in 0..4 {
        let _ = suggest(&w, &st, s, Level::Normal);
    }
    // Generous in debug builds; release builds are far faster.
    assert!(t.elapsed().as_millis() < 4000);
}
