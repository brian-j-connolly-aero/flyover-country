use flyover_ai::{suggest, Level};
use flyover_core::content::FactionId::*;
use flyover_core::state::Event;
use flyover_core::*;

fn main() {
    let w = testkit::world();
    let show = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u8>().unwrap())
        .unwrap_or(3);
    let factions: Vec<flyover_core::content::FactionId> = std::env::args()
        .nth(2)
        .map(|s| {
            s.split(',')
                .map(|x| {
                    flyover_core::content::FactionId::ALL
                        .into_iter()
                        .find(|f| format!("{f:?}") == x)
                        .unwrap()
                })
                .collect()
        })
        .unwrap_or(vec![Pacific, FrontRange, Texas, Beehive]);
    let mut st = new_game(&w, &GameConfig { seed: 3, factions }).unwrap();
    while st.night <= 8 {
        let orders: Vec<Orders> = (0..4).map(|s| suggest(&w, &st, s, Level::Normal)).collect();
        if st.night == show {
            for o in &orders {
                println!(
                    "seat {} ({:?}) builds {:?} repairs {:?}",
                    o.seat, st.seats[o.seat as usize].faction, o.builds, o.repairs
                );
                for m in &o.missions {
                    let s = st.squadrons.iter().find(|s| s.id == m.sq).unwrap();
                    let desc = match &m.mission {
                        flyover_core::state::Mission::Attack { node } => {
                            format!("Attack {}", w.nodes[*node as usize].name)
                        }
                        flyover_core::state::Mission::Guard { node } => {
                            format!("Guard {}", w.nodes[*node as usize].name)
                        }
                        x => format!("{:?}", x),
                    };
                    println!(
                        "   {:?}{} @{} -> {}",
                        s.kind,
                        if s.ace { "*" } else { "" },
                        w.nodes[s.base as usize].name,
                        desc
                    );
                }
                let t = flyover_ai::analysis::best_targets(&w, &st, o.seat, 5);
                for x in t {
                    println!(
                        "      target {} value {} ({})",
                        w.nodes[x.node as usize].name, x.value, x.why
                    );
                }
            }
        }
        let (next, ev) = resolve_night(&w, &st, &orders);
        if st.night == show {
            for e in &ev {
                match &e.e {
                    Event::Engage {
                        a_seat,
                        b_seat,
                        x,
                        y,
                        after,
                        ..
                    } => println!(
                        "t{} engage {}v{} at ({},{}) after {:?}",
                        e.tick,
                        a_seat,
                        b_seat,
                        x,
                        y,
                        after
                            .iter()
                            .map(|s| (s.seat, s.strength))
                            .collect::<Vec<_>>()
                    ),
                    Event::Strike {
                        seat,
                        node,
                        steps,
                        hits,
                        ..
                    } => println!(
                        "t{} strike by {} on {} hits {} steps {}",
                        e.tick, seat, w.nodes[*node as usize].name, hits, steps
                    ),
                    Event::Score {
                        seat,
                        delta,
                        reason,
                        ..
                    } => println!("t{} score {} +{} {:?}", e.tick, seat, delta, reason),
                    Event::Grounded { seat, reason, .. } => {
                        println!("grounded {} {}", seat, reason)
                    }
                    Event::Abort { seat, .. } => println!("t{} abort {}", e.tick, seat),
                    Event::Capture { node, seat, .. } => {
                        println!("capture {} by {}", w.nodes[*node as usize].name, seat)
                    }
                    _ => {}
                }
            }
        }
        st = next;
    }
}
