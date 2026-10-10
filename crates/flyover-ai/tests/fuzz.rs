//! Property-style fuzzing: throw random (often illegal) orders at the
//! simulation and check that it never panics and every invariant holds.

use flyover_core::content::{FactionId, UnitKind};
use flyover_core::rng::Rng;
use flyover_core::state::*;
use flyover_core::*;
use std::collections::BTreeSet;

fn random_orders(w: &World, st: &GameState, seat: u8, rng: &mut Rng) -> Orders {
    let nn = w.nodes.len() as u32;
    let nz = w.zones.len() as u32;
    let mine: Vec<&Squadron> = st.squadrons.iter().filter(|s| s.seat == seat).collect();
    let mut o = Orders {
        seat,
        ..Default::default()
    };
    for s in &mine {
        let mission = match rng.below(10) {
            0 => Mission::Rest,
            1 | 2 => Mission::Attack {
                node: rng.below(nn) as u16,
            },
            3 => Mission::Guard {
                node: rng.below(nn) as u16,
            },
            4 => Mission::Sweep {
                zone: rng.below(nz) as u16,
            },
            5 => Mission::Scout {
                zone: rng.below(nz) as u16,
            },
            6 => Mission::JamZone {
                zone: rng.below(nz) as u16,
            },
            7 => Mission::Escort {
                sq: mine[rng.below(mine.len() as u32) as usize].id,
            },
            8 => Mission::Rebase {
                node: rng.below(nn) as u16,
            },
            _ => Mission::GuardZone {
                zone: rng.below(nz) as u16,
            },
        };
        let waypoint = if rng.chance(150) {
            // Sometimes straight into the Static.
            Some((rng.below(1300) as i32 - 50, rng.below(1000) as i32))
        } else {
            None
        };
        o.missions.push(SqOrder {
            sq: s.id,
            mission,
            waypoint,
        });
    }
    for _ in 0..rng.below(5) {
        o.builds
            .push(UnitKind::ALL[rng.below(UnitKind::ALL.len() as u32) as usize]);
    }
    for _ in 0..rng.below(3) {
        o.repairs.push(rng.below(nn) as u16);
    }
    o.harvest_bid = rng.below(12) as i32 - 2;
    if rng.chance(500) {
        o.bet = Some(Bet {
            on: rng.below(5) as u8,
            node: Some(rng.below(nn + 5) as u16),
            victim: None,
            stake: rng.below(10) as i32 - 2,
        });
    }
    if rng.chance(500) {
        o.intel_sale = Some(IntelSale {
            price: rng.below(8) as i32 - 1,
            only: None,
        });
        o.intel_request = Some(IntelRequest {
            zone: rng.below(nz + 3) as u16,
            max_price: rng.below(8) as i32,
        });
    }
    o.mass_driver = Some(rng.below(nn + 3) as u16);
    o.revenge = Some(rng.below(nn) as u16);
    o
}

#[test]
fn random_orders_never_break_the_rules() {
    let w = testkit::world();
    let all = FactionId::ALL;
    for game in 0..40u64 {
        let mut rng = Rng::for_night(game, 0, 99);
        let mut picks: Vec<FactionId> = all.to_vec();
        let mut factions = Vec::new();
        for _ in 0..4 {
            factions.push(picks.remove(rng.below(picks.len() as u32) as usize));
        }
        let mut st = new_game(
            &w,
            &GameConfig {
                seed: game,
                factions,
            },
        )
        .unwrap();
        while st.night <= w.rules().nights {
            let orders: Vec<Orders> = (0..4)
                .map(|s| random_orders(&w, &st, s, &mut rng))
                .collect();
            let (next, events) = resolve_night(&w, &st, &orders);
            let mut ids = BTreeSet::new();
            for s in &next.seats {
                assert!(
                    s.supply >= 0 && s.fuel >= 0,
                    "negative stockpile in game {game}"
                );
            }
            for q in &next.squadrons {
                assert!(ids.insert(q.id), "squadron in two places");
                assert!(q.strength <= MAX_STRENGTH);
            }
            for e in &events {
                assert!(e.tick >= 0 && e.tick <= w.rules().ticks);
            }
            assert_eq!(next.night, st.night + 1);
            st = next;
        }
    }
}
