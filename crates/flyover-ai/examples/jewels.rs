use flyover_ai::{suggest, Level};
use flyover_core::content::{FactionId, UnitKind};
use flyover_core::state::Event;
use flyover_core::*;
fn main() {
    let w = testkit::world();
    let all = FactionId::ALL;
    let mut lineups = Vec::new();
    for a in 0..7 {
        for b in a + 1..7 {
            for c in b + 1..7 {
                for d in c + 1..7 {
                    lineups.push(vec![all[a], all[b], all[c], all[d]]);
                }
            }
        }
    }
    let mut jewel = [[0i64; 2]; 7];
    let mut steps_by_kind = std::collections::BTreeMap::<String, (i64, i64)>::new();
    for g in 0..140 {
        let l = lineups[g % lineups.len()].clone();
        let mut st = new_game(
            &w,
            &GameConfig {
                seed: 900 + g as u64,
                factions: l.clone(),
            },
        )
        .unwrap();
        while st.night <= 8 {
            let orders: Vec<Orders> = (0..4).map(|s| suggest(&w, &st, s, Level::Normal)).collect();
            let (next, ev) = resolve_night(&w, &st, &orders);
            for e in &ev {
                if let Event::Strike {
                    seat,
                    kinds,
                    steps,
                    jewel: j,
                    ..
                } = &e.e
                {
                    let f = l[*seat as usize].index();
                    if *j && *steps > 0 {
                        jewel[f][if kinds.contains(&UnitKind::Longhorn) {
                            1
                        } else {
                            0
                        }] += 1;
                    }
                    for k in kinds {
                        let x = steps_by_kind.entry(format!("{k:?}")).or_default();
                        x.0 += 1;
                        x.1 += *steps as i64;
                    }
                }
            }
            st = next;
        }
    }
    for f in all {
        println!(
            "{:?}: jewel hits {} (with Longhorn {})",
            f,
            jewel[f.index()][0] + jewel[f.index()][1],
            jewel[f.index()][1]
        );
    }
    for (k, (n, s)) in steps_by_kind {
        println!("{k}: {n} strikes, {:.2} steps each", s as f64 / n as f64);
    }
}
