//! Who shoots down whom, and with what (balance work).
use flyover_ai::{suggest, Level};
use flyover_core::content::FactionId;
use flyover_core::state::Event;
use flyover_core::*;
use std::collections::BTreeMap;

fn main() {
    let w = testkit::world();
    let focus = std::env::args().nth(1).unwrap_or("Beehive".into());
    let games: usize = std::env::args()
        .nth(2)
        .map(|s| s.parse().unwrap())
        .unwrap_or(100);
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
    let lineups: Vec<_> = lineups
        .into_iter()
        .filter(|l| l.iter().any(|f| format!("{f:?}") == focus))
        .collect();
    let mut by_kind: BTreeMap<String, i64> = BTreeMap::new();
    let mut victim_kind: BTreeMap<String, i64> = BTreeMap::new();
    let mut ctx: BTreeMap<String, i64> = BTreeMap::new();
    let mut n = 0;
    for g in 0..games {
        let l = lineups[g % lineups.len()].clone();
        let me = l.iter().position(|f| format!("{f:?}") == focus).unwrap() as u8;
        let mut st = new_game(
            &w,
            &GameConfig {
                seed: 5000 + g as u64,
                factions: l.clone(),
            },
        )
        .unwrap();
        while st.night <= 8 {
            let orders: Vec<Orders> = (0..4).map(|s| suggest(&w, &st, s, Level::Normal)).collect();
            let (next, ev) = resolve_night(&w, &st, &orders);
            // group id -> mission tag
            let mut tags = BTreeMap::new();
            for e in &ev {
                if let Event::Launch {
                    group,
                    mission,
                    seat,
                    ..
                } = &e.e
                {
                    tags.insert((*seat, *group), *mission);
                }
            }
            for e in &ev {
                if let Event::Engage {
                    a_seat,
                    b_seat,
                    a_group,
                    b_group,
                    ..
                } = &e.e
                {
                    if *a_seat == me {
                        *ctx.entry(format!(
                            "we hunt ({:?}) vs their {:?}",
                            tags.get(&(*a_seat, *a_group)),
                            tags.get(&(*b_seat, *b_group))
                        ))
                        .or_default() += 1;
                    }
                    if *b_seat == me {
                        *ctx.entry(format!(
                            "we are hunted ({:?}) by their {:?}",
                            tags.get(&(*b_seat, *b_group)),
                            tags.get(&(*a_seat, *a_group))
                        ))
                        .or_default() += 1;
                    }
                }
                if let Event::Splash {
                    by_seat: Some(b),
                    by_sq,
                    by_site,
                    kind,
                    ground,
                    ..
                } = &e.e
                {
                    if *b == me {
                        n += 1;
                        let k = if *ground {
                            "ground strike".to_string()
                        } else if by_site.is_some() {
                            "air defense".into()
                        } else {
                            let s = st
                                .squadrons
                                .iter()
                                .find(|s| Some(s.id) == *by_sq)
                                .or(next.squadrons.iter().find(|s| Some(s.id) == *by_sq));
                            s.map(|s| format!("{:?}{}", s.kind, if s.ace { "*" } else { "" }))
                                .unwrap_or("?".into())
                        };
                        *by_kind.entry(k).or_default() += 1;
                        *victim_kind.entry(format!("{:?}", kind)).or_default() += 1;
                    }
                }
            }
            st = next;
        }
    }
    println!("{focus}: {:.1} kills per game", n as f64 / games as f64);
    println!("by shooter: {:?}", by_kind);
    println!("victims: {:?}", victim_kind);
    for (k, v) in ctx {
        println!("  {k}: {:.2}/game", v as f64 / games as f64);
    }
}
