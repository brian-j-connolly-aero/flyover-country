//! Per-faction battle statistics over many bot games (for balance work).
use flyover_ai::{suggest, Level};
use flyover_core::content::FactionId;
use flyover_core::state::{Event, MissionTag, ScoreReason};
use flyover_core::*;

#[derive(Default, Clone)]
struct S {
    games: u32,
    wins: f64,
    br: i64,
    pride: i64,
    strike_launch: i64,
    strike_done: i64,
    steps: i64,
    aborts: i64,
    lost: i64,
    kills: i64,
    flak_hits_taken: i64,
    fights: i64,
    guard_launch: i64,
    capture: i64,
    supply_end: i64,
    fuel_end: i64,
    squads_end: i64,
    grounded: i64,
    br_reason: [i64; 9],
}

fn main() {
    let w = testkit::world();
    let games: usize = std::env::args()
        .nth(1)
        .map(|s| s.parse().unwrap())
        .unwrap_or(140);
    let mut st_all: Vec<S> = vec![S::default(); 7];
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
    for g in 0..games {
        let l = lineups[g % lineups.len()].clone();
        let mut st = new_game(
            &w,
            &GameConfig {
                seed: 1000 + g as u64,
                factions: l.clone(),
            },
        )
        .unwrap();
        while st.night <= 8 {
            let orders: Vec<Orders> = (0..4).map(|s| suggest(&w, &st, s, Level::Normal)).collect();
            let (next, ev) = resolve_night(&w, &st, &orders);
            for e in &ev {
                let f = |s: u8| l[s as usize].index();
                match &e.e {
                    Event::Launch { seat, mission, .. } => {
                        if *mission == MissionTag::Attack {
                            st_all[f(*seat)].strike_launch += 1
                        }
                        if *mission == MissionTag::Guard {
                            st_all[f(*seat)].guard_launch += 1
                        }
                    }
                    Event::Strike { seat, steps, .. } => {
                        st_all[f(*seat)].strike_done += 1;
                        st_all[f(*seat)].steps += *steps as i64;
                    }
                    Event::Abort { seat, .. } => st_all[f(*seat)].aborts += 1,
                    Event::Splash { seat, by_seat, .. } => {
                        st_all[f(*seat)].lost += 1;
                        if let Some(b) = by_seat {
                            st_all[f(*b)].kills += 1;
                        }
                    }
                    Event::Flak { seat, shots, .. } => {
                        st_all[f(*seat)].flak_hits_taken +=
                            shots.iter().filter(|s| s.hit).count() as i64
                    }
                    Event::Engage { a_seat, b_seat, .. } => {
                        st_all[f(*a_seat)].fights += 1;
                        st_all[f(*b_seat)].fights += 1;
                    }
                    Event::Capture { seat, .. } => st_all[f(*seat)].capture += 1,
                    Event::Grounded { seat, .. } => st_all[f(*seat)].grounded += 1,
                    Event::Score {
                        seat,
                        delta,
                        reason,
                        ..
                    } => {
                        let k = match reason {
                            ScoreReason::Strike => 0,
                            ScoreReason::Jewel => 1,
                            ScoreReason::Splash => 2,
                            ScoreReason::Ace => 3,
                            ScoreReason::Ground => 4,
                            ScoreReason::Objective => 5,
                            ScoreReason::Casserole => 6,
                            ScoreReason::MassDriver | ScoreReason::Fetch => 7,
                            ScoreReason::Revenge => 8,
                        };
                        st_all[f(*seat)].br_reason[k] += *delta as i64;
                    }
                    _ => {}
                }
            }
            st = next;
        }
        let best = st.seats.iter().map(|s| s.score).max().unwrap();
        let n = st.seats.iter().filter(|s| s.score == best).count() as f64;
        for (i, s) in st.seats.iter().enumerate() {
            let x = &mut st_all[s.faction.index()];
            x.games += 1;
            x.br += s.score as i64;
            x.pride += s.pride_lost as i64;
            if s.score == best {
                x.wins += 1.0 / n;
            }
            x.supply_end += s.supply as i64;
            x.fuel_end += s.fuel as i64;
            x.squads_end += st
                .squadrons
                .iter()
                .filter(|q| q.seat == i as u8 && q.active())
                .count() as i64;
        }
    }
    println!("{:<11} {:>5} {:>5} {:>5} | {:>6} {:>6} {:>5} {:>5} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>4} {:>4} {:>4} {:>4}", "faction", "win%", "BR", "pride", "strL", "strD", "steps", "abrt", "lost", "kill", "flak", "fight", "capt", "sup", "fuel", "sqd", "grnd");
    for f in FactionId::ALL {
        let x = &st_all[f.index()];
        let g = x.games.max(1) as f64;
        println!(
            "{:<11} {:>5.1} {:>5.1} {:>5.1} | {:>6.1} {:>6.1} {:>5.1} {:>5.1} | {:>5.1} {:>5.1} {:>5.1} {:>5.1} | {:>4.1} {:>4.1} {:>4.1} {:>4.1} {:>4.1}",
            format!("{:?}", f), 100.0 * x.wins / g, x.br as f64 / g, x.pride as f64 / g,
            x.strike_launch as f64 / g, x.strike_done as f64 / g, x.steps as f64 / g, x.aborts as f64 / g,
            x.lost as f64 / g, x.kills as f64 / g, x.flak_hits_taken as f64 / g, x.fights as f64 / g,
            x.capture as f64 / g, x.supply_end as f64 / g, x.fuel_end as f64 / g, x.squads_end as f64 / g, x.grounded as f64 / g
        );
        let r = x.br_reason;
        println!("            BR: strike {:.1} jewel {:.1} splash {:.1} ace {:.1} ground {:.1} obj {:.1} cass {:.1} md {:.1} rev {:.1}",
            r[0] as f64 / g, r[1] as f64 / g, r[2] as f64 / g, r[3] as f64 / g, r[4] as f64 / g, r[5] as f64 / g, r[6] as f64 / g, r[7] as f64 / g, r[8] as f64 / g);
    }
}
