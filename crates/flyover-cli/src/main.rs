//! `flyover`: headless bot-vs-bot games, the balance harness and replay
//! verification.
//!
//!   flyover play [--seed N] [--factions Pacific,Texas,...] [--level easy|normal]
//!   flyover balance [--games N] [--players 2-7] [--threads N] [--level easy|normal]
//!   flyover hashes --seed N --factions A,B,C,D      (per-night state hashes)
//!   flyover replay <save.json[.gz]>                  (re-resolve and check every hash)

use flyover_ai::{suggest, Level};
use flyover_core::content::{ContentFiles, FactionId, World};
use flyover_core::state::{Event, GameState, Orders, ScoreReason};
use flyover_core::{new_game, resolve_night, state_hash, GameConfig};
use serde::Deserialize;
use std::io::Read;
use std::time::Instant;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn load_world(dir: &str) -> World {
    let read = |f: &str| {
        std::fs::read_to_string(format!("{dir}/{f}")).unwrap_or_else(|e| panic!("{dir}/{f}: {e}"))
    };
    let (map, factions, rules) = (read("map.ron"), read("factions.ron"), read("rules.ron"));
    World::from_files(&ContentFiles {
        map: &map,
        factions: &factions,
        rules: &rules,
    })
    .unwrap_or_else(|e| panic!("{e}"))
}

fn parse_faction(s: &str) -> FactionId {
    FactionId::ALL
        .into_iter()
        .find(|f| format!("{f:?}").eq_ignore_ascii_case(s.trim()))
        .unwrap_or_else(|| panic!("unknown faction {s}"))
}

fn parse_level(s: Option<String>) -> Level {
    match s.as_deref() {
        Some("easy") => Level::Easy,
        _ => Level::Normal,
    }
}

#[derive(Default, Clone)]
struct GameResult {
    factions: Vec<FactionId>,
    scores: Vec<i32>,
    hashes: Vec<String>,
    parked: Vec<bool>,
    /// Points by reason per seat: strike, downed, prize, fetch.
    by_reason: Vec<[i32; 4]>,
    intercepts: usize,
    strikes: usize,
}

fn play(world: &World, seed: u64, factions: &[FactionId], level: Level, log: bool) -> GameResult {
    let mut st: GameState = new_game(
        world,
        &GameConfig {
            seed,
            factions: factions.to_vec(),
        },
    )
    .expect("new game");
    let n = factions.len();
    let mut r = GameResult {
        factions: factions.to_vec(),
        parked: vec![false; n],
        by_reason: vec![[0; 4]; n],
        ..Default::default()
    };
    while !st.over(world) {
        let orders: Vec<Orders> = (0..n as u8)
            .map(|seat| suggest(world, &st, seat, level))
            .collect();
        let (next, events) = resolve_night(world, &st, &orders);
        for e in &events {
            match &e.e {
                Event::BarkPark { seat } => r.parked[*seat as usize] = true,
                Event::Intercept { .. } => r.intercepts += 1,
                Event::Strike { .. } => r.strikes += 1,
                Event::Score {
                    seat,
                    delta,
                    reason,
                } => {
                    let i = match reason {
                        ScoreReason::Strike => 0,
                        ScoreReason::Downed => 1,
                        ScoreReason::Prize => 2,
                        ScoreReason::Fetch => 3,
                    };
                    r.by_reason[*seat as usize][i] += delta;
                }
                _ => {}
            }
        }
        if log {
            let line: Vec<String> = next
                .seats
                .iter()
                .map(|s| format!("{:?} {} ({} supply)", s.faction.unwrap(), s.score, s.supply))
                .collect();
            println!("night {}: {}", st.night, line.join(", "));
        }
        st = next;
        r.hashes.push(state_hash(&st));
    }
    r.scores = st.seats.iter().map(|s| s.score).collect();
    r
}

/// Every way to pick `n` of the seven factions.
fn lineups(n: usize) -> Vec<Vec<FactionId>> {
    let all = FactionId::ALL;
    (0u32..1 << 7)
        .filter(|m| m.count_ones() as usize == n)
        .map(|m| {
            (0..7)
                .filter(|i| m & (1 << i) != 0)
                .map(|i| all[i])
                .collect()
        })
        .collect()
}

fn balance(world: &World, games: usize, threads: usize, level: Level, players: usize) {
    let lineups = lineups(players);
    let start = Instant::now();
    let jobs: Vec<(u64, Vec<FactionId>)> = (0..games)
        .map(|g| {
            let mut l = lineups[g % lineups.len()].clone();
            // Rotate seat order so nobody always sits first.
            l.rotate_left(g / lineups.len() % players);
            (0xF1F0 + g as u64 * 7919, l)
        })
        .collect();
    let results: Vec<GameResult> = std::thread::scope(|scope| {
        let chunks: Vec<&[(u64, Vec<FactionId>)]> =
            jobs.chunks(jobs.len().div_ceil(threads.max(1))).collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|(s, l)| play(world, *s, l, level, false))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let mut wins = [0.0f64; 7];
    let mut played = [0usize; 7];
    let mut total = [0i64; 7];
    let mut reasons = [[0i64; 4]; 7];
    let (mut park_visits, mut park_wins, mut park_not_last) = (0usize, 0.0f64, 0usize);
    let mut margin = 0i64;
    let (mut intercepts, mut strikes) = (0usize, 0usize);
    for r in &results {
        let best = *r.scores.iter().max().unwrap();
        let winners: Vec<usize> = (0..r.scores.len())
            .filter(|&i| r.scores[i] == best)
            .collect();
        let mut sorted = r.scores.clone();
        sorted.sort();
        margin += (sorted[sorted.len() - 1] - sorted[sorted.len() - 2]) as i64;
        let worst = sorted[0];
        intercepts += r.intercepts;
        strikes += r.strikes;
        for (i, f) in r.factions.iter().enumerate() {
            let k = f.index();
            played[k] += 1;
            total[k] += r.scores[i] as i64;
            for (slot, v) in reasons[k].iter_mut().zip(r.by_reason[i]) {
                *slot += v as i64;
            }
            if winners.contains(&i) {
                wins[k] += 1.0 / winners.len() as f64;
            }
            if r.parked[i] {
                park_visits += 1;
                if winners.contains(&i) {
                    park_wins += 1.0 / winners.len() as f64;
                }
                if r.scores[i] > worst {
                    park_not_last += 1;
                }
            }
        }
    }
    let n = results.len() as f64;
    println!(
        "{} games in {:.1} s",
        results.len(),
        start.elapsed().as_secs_f64()
    );
    println!(
        "{:<12} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "faction", "win %", "avg BR", "strike", "downed", "prize", "fetch"
    );
    // A fair share of wins is 100/players percent; allow a fifth either way.
    let fair = 100.0 / players as f64;
    let band = (fair * 0.8)..=(fair * 1.2);
    let mut bad = false;
    for f in FactionId::ALL {
        let k = f.index();
        let p = played[k].max(1) as f64;
        let rate = 100.0 * wins[k] / p;
        if played[k] > 0 && !band.contains(&rate) {
            bad = true;
        }
        println!(
            "{:<12} {:>6.1}% {:>7.1} {:>7.1} {:>7.1} {:>7.1} {:>7.1}",
            format!("{f:?}"),
            rate,
            total[k] as f64 / p,
            reasons[k][0] as f64 / p,
            reasons[k][1] as f64 / p,
            reasons[k][2] as f64 / p,
            reasons[k][3] as f64 / p,
        );
    }
    println!(
        "Bark Park: {:.2} visits a game; visitors win {:.0}% and finish above last {:.0}%",
        park_visits as f64 / n,
        100.0 * park_wins / park_visits.max(1) as f64,
        100.0 * park_not_last as f64 / park_visits.max(1) as f64
    );
    println!(
        "average winning margin {:.1}; {:.1} interceptions and {:.1} strikes a game",
        margin as f64 / n,
        intercepts as f64 / n,
        strikes as f64 / n
    );
    if bad {
        eprintln!(
            "a faction is outside the {:.0}-{:.0}% win band",
            band.start(),
            band.end()
        );
        std::process::exit(1);
    }
}

#[derive(Deserialize)]
struct SaveNight {
    orders: Vec<Orders>,
    hash: String,
}

#[derive(Deserialize)]
struct Save {
    seed: u64,
    factions: Vec<FactionId>,
    nights: Vec<SaveNight>,
}

fn replay(world: &World, path: &str) {
    let mut bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut d = flate2::read::GzDecoder::new(&bytes[..]);
        let mut out = Vec::new();
        d.read_to_end(&mut out).expect("gunzip");
        bytes = out;
    }
    let save: Save = serde_json::from_slice(&bytes).expect("save file");
    let mut st = new_game(
        world,
        &GameConfig {
            seed: save.seed,
            factions: save.factions.clone(),
        },
    )
    .expect("new game");
    let mut bad = 0;
    for (i, n) in save.nights.iter().enumerate() {
        let (next, _) = resolve_night(world, &st, &n.orders);
        st = next;
        let h = state_hash(&st);
        let ok = h == n.hash;
        if !ok {
            bad += 1;
        }
        println!(
            "night {}: {} {}",
            i + 1,
            h,
            if ok { "ok" } else { "MISMATCH" }
        );
    }
    if bad > 0 {
        eprintln!("{bad} mismatched nights");
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = arg(&args, "--content").unwrap_or_else(|| "content".into());
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");
    match cmd {
        "play" | "hashes" => {
            let world = load_world(&dir);
            let seed: u64 = arg(&args, "--seed")
                .map(|s| s.parse().expect("seed"))
                .unwrap_or(7);
            let factions: Vec<FactionId> = arg(&args, "--factions")
                .map(|s| s.split(',').map(parse_faction).collect())
                .unwrap_or_else(|| {
                    vec![
                        FactionId::Pacific,
                        FactionId::FrontRange,
                        FactionId::Texas,
                        FactionId::Beehive,
                    ]
                });
            let level = parse_level(arg(&args, "--level"));
            let t = Instant::now();
            let r = play(&world, seed, &factions, level, cmd == "play");
            if cmd == "hashes" {
                for h in r.hashes {
                    println!("{h}");
                }
            } else {
                println!(
                    "final: {:?} in {:.0} ms",
                    r.scores,
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
        }
        "balance" => {
            let mut world = load_world(&dir);
            if args.iter().any(|a| a == "--no-perks") {
                world.content.rules.perks = false;
            }
            let games = arg(&args, "--games")
                .map(|s| s.parse().expect("games"))
                .unwrap_or(350);
            let threads = arg(&args, "--threads")
                .map(|s| s.parse().expect("threads"))
                .unwrap_or_else(|| {
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4)
                });
            let players = arg(&args, "--players")
                .map(|s| s.parse().expect("players"))
                .unwrap_or(4);
            assert!((2..=7).contains(&players), "--players takes 2 to 7");
            balance(
                &world,
                games,
                threads,
                parse_level(arg(&args, "--level")),
                players,
            );
        }
        "replay" => {
            let world = load_world(&dir);
            replay(&world, args.get(2).expect("save path"));
        }
        _ => println!(
            "flyover play | balance | hashes | replay <save>   (--content DIR, default ./content)"
        ),
    }
}
