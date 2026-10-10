//! `flyover`: headless bot-vs-bot games, the balance harness and replay
//! verification.
//!
//!   flyover play [--seed N] [--factions Pacific,Texas,...] [--level easy|normal] [--log]
//!   flyover balance [--games N] [--threads N] [--level easy|normal]
//!   flyover hashes --seed N --factions A,B,C,D      (per-night state hashes)
//!   flyover replay <save.json[.gz]>                  (re-resolve and check every hash)

use flyover_ai::{suggest, Level};
use flyover_core::content::{Content, ContentFiles, FactionId, World};
use flyover_core::state::{Event, GameState, Orders};
use flyover_core::{new_game, resolve_night, state_hash, GameConfig};
use serde::Deserialize;
use std::io::Read;
use std::time::Instant;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn load_world(dir: &str) -> World {
    let read = |f: &str| std::fs::read_to_string(format!("{dir}/{f}")).unwrap_or_else(|e| panic!("{dir}/{f}: {e}"));
    let (map, units, factions, rules, patch) =
        (read("map.ron"), read("units.ron"), read("factions.ron"), read("rules.ron"), read("patch_notes.ron"));
    let files = ContentFiles { map: &map, units: &units, factions: &factions, rules: &rules, patch_notes: &patch };
    World::new(Content::parse(&files).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"))
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

struct GameResult {
    factions: Vec<FactionId>,
    scores: Vec<i32>,
    hashes: Vec<String>,
    /// fights, splashes, strikes, steps, captures, jewel hits, grounded
    stats: [i64; 7],
    /// Bragging Rights by reason: strike, jewel, splash, ace, ground, objective, other
    by_reason: Vec<[i64; 7]>,
}

fn play(world: &World, seed: u64, factions: &[FactionId], levels: &[Level], log: bool) -> GameResult {
    let mut st = new_game(world, &GameConfig { seed, factions: factions.to_vec() }).expect("new game");
    let mut hashes = Vec::new();
    let mut stats = [0i64; 7];
    let mut by_reason = vec![[0i64; 7]; factions.len()];
    while st.night <= world.rules().nights {
        let orders: Vec<Orders> = (0..factions.len() as u8).map(|s| suggest(world, &st, s, levels[s as usize])).collect();
        let (next, ev) = resolve_night(world, &st, &orders);
        for e in &ev {
            use flyover_core::state::ScoreReason as R;
            match &e.e {
                Event::Engage { .. } => stats[0] += 1,
                Event::Splash { .. } => stats[1] += 1,
                Event::Strike { steps, .. } => {
                    stats[2] += 1;
                    stats[3] += *steps as i64
                }
                Event::Capture { .. } => stats[4] += 1,
                Event::JewelEffect { .. } => stats[5] += 1,
                Event::Grounded { .. } => stats[6] += 1,
                Event::Score { seat, delta, reason, .. } => {
                    let k = match reason {
                        R::Strike | R::MassDriver | R::Revenge => 0,
                        R::Jewel => 1,
                        R::Splash => 2,
                        R::Ace => 3,
                        R::Ground => 4,
                        R::Objective => 5,
                        R::Casserole => 6,
                    };
                    by_reason[*seat as usize][k] += *delta as i64;
                }
                _ => {}
            }
        }
        if log {
            print_night(world, &st, &next, &ev);
        }
        st = next;
        hashes.push(state_hash(&st));
    }
    GameResult { factions: factions.to_vec(), scores: st.seats.iter().map(|s| s.score).collect(), hashes, stats, by_reason }
}

fn print_night(world: &World, before: &GameState, after: &GameState, ev: &[flyover_core::EventRec]) {
    let mut fights = 0;
    let mut splashes = 0;
    let mut strikes = 0;
    let mut steps = 0;
    let mut flak = 0;
    let mut grounded = 0;
    for e in ev {
        match &e.e {
            Event::Engage { .. } => fights += 1,
            Event::Splash { .. } => splashes += 1,
            Event::Strike { steps: s, .. } => {
                strikes += 1;
                steps += s
            }
            Event::Flak { .. } => flak += 1,
            Event::Grounded { .. } => grounded += 1,
            _ => {}
        }
    }
    println!(
        "Night {}: {} dogfights, {} flak volleys, {} splashes, {} strikes ({} steps), {} grounded",
        before.night, fights, flak, splashes, strikes, steps, grounded
    );
    for (i, s) in after.seats.iter().enumerate() {
        let sq = after.squadrons.iter().filter(|q| q.seat == i as u8 && q.active()).count();
        println!(
            "  {:<24} score {:>3}  supply {:>3} (+{:>2})  fuel {:>3} (+{:>2})  squadrons {:>2}",
            world.faction(s.faction).name,
            s.score,
            s.supply,
            s.income_supply,
            s.fuel,
            s.income_fuel,
            sq
        );
    }
}

fn lineups() -> Vec<Vec<FactionId>> {
    let all = FactionId::ALL;
    let mut out = Vec::new();
    for a in 0..7 {
        for b in a + 1..7 {
            for c in b + 1..7 {
                for d in c + 1..7 {
                    out.push(vec![all[a], all[b], all[c], all[d]]);
                }
            }
        }
    }
    out
}

fn balance(world: &World, games: usize, threads: usize, level: Level) {
    let lineups = lineups();
    let start = Instant::now();
    let jobs: Vec<(u64, Vec<FactionId>)> = (0..games)
        .map(|g| {
            let mut l = lineups[g % lineups.len()].clone();
            // Rotate seat order so nobody always sits first.
            l.rotate_left(g / lineups.len() % 4);
            (0xF1F0 + g as u64 * 7919, l)
        })
        .collect();
    let results: Vec<GameResult> = std::thread::scope(|scope| {
        let chunks: Vec<&[(u64, Vec<FactionId>)]> = jobs.chunks(jobs.len().div_ceil(threads.max(1))).collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || chunk.iter().map(|(seed, l)| play(world, *seed, l, &[level; 4], false)).collect::<Vec<_>>())
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("worker")).collect()
    });
    let mut wins = [0f64; 7];
    let mut played = [0u32; 7];
    let mut score_sum = [0i64; 7];
    let mut margin = 0i64;
    let mut stats = [0i64; 7];
    let mut reasons = [[0i64; 7]; 7];
    for r in &results {
        for k in 0..7 {
            stats[k] += r.stats[k];
        }
        for (i, f) in r.factions.iter().enumerate() {
            for k in 0..7 {
                reasons[f.index()][k] += r.by_reason[i][k];
            }
        }
        let best = *r.scores.iter().max().unwrap();
        let winners = r.scores.iter().filter(|&&s| s == best).count() as f64;
        let mut sorted = r.scores.clone();
        sorted.sort();
        margin += (sorted[3] - sorted[2]) as i64;
        for (i, f) in r.factions.iter().enumerate() {
            played[f.index()] += 1;
            score_sum[f.index()] += r.scores[i] as i64;
            if r.scores[i] == best {
                wins[f.index()] += 1.0 / winners;
            }
        }
    }
    println!("{} games in {:.1}s ({:?} bots)", results.len(), start.elapsed().as_secs_f64(), level);
    println!("{:<26} {:>6} {:>8} {:>9}", "faction", "games", "win %", "avg BR");
    let mut ok = true;
    for f in FactionId::ALL {
        let i = f.index();
        let pct = 100.0 * wins[i] / played[i].max(1) as f64;
        let flag = if (20.0..=30.0).contains(&pct) { "" } else { "  <- outside 20-30%" };
        if !flag.is_empty() {
            ok = false;
        }
        println!(
            "{:<26} {:>6} {:>7.1}% {:>9.1}{}",
            world.faction(f).name,
            played[i],
            pct,
            score_sum[i] as f64 / played[i].max(1) as f64,
            flag
        );
    }
    println!("average winning margin: {:.1}", margin as f64 / results.len().max(1) as f64);
    let n = results.len().max(1) as f64;
    println!(
        "per game: {:.1} dogfights, {:.1} splashes, {:.1} strikes, {:.1} steps, {:.1} captures, {:.2} jewel hits, {:.1} grounded",
        stats[0] as f64 / n, stats[1] as f64 / n, stats[2] as f64 / n, stats[3] as f64 / n, stats[4] as f64 / n, stats[5] as f64 / n, stats[6] as f64 / n
    );
    println!("avg BR by source:          strike  jewel  splash   ace  ground  object  cass");
    for f in FactionId::ALL {
        let i = f.index();
        let g = played[i].max(1) as f64;
        let r = reasons[i];
        println!(
            "{:<26} {:>6.1} {:>6.1} {:>7.1} {:>5.1} {:>7.1} {:>7.1} {:>5.1}",
            world.faction(f).name, r[0] as f64 / g, r[1] as f64 / g, r[2] as f64 / g, r[3] as f64 / g, r[4] as f64 / g, r[5] as f64 / g, r[6] as f64 / g
        );
    }
    if !ok {
        std::process::exit(2);
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
    let mut st = new_game(world, &GameConfig { seed: save.seed, factions: save.factions.clone() }).expect("new game");
    let mut bad = 0;
    for (i, n) in save.nights.iter().enumerate() {
        let (next, _) = resolve_night(world, &st, &n.orders);
        st = next;
        let h = state_hash(&st);
        let ok = h == n.hash;
        if !ok {
            bad += 1;
        }
        println!("night {}: {} {}", i + 1, h, if ok { "ok" } else { "MISMATCH" });
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
            let seed: u64 = arg(&args, "--seed").map(|s| s.parse().expect("seed")).unwrap_or(7);
            let factions: Vec<FactionId> = arg(&args, "--factions")
                .map(|s| s.split(',').map(parse_faction).collect())
                .unwrap_or_else(|| vec![FactionId::Pacific, FactionId::FrontRange, FactionId::Texas, FactionId::Beehive]);
            let level = parse_level(arg(&args, "--level"));
            let t = Instant::now();
            let r = play(&world, seed, &factions, &vec![level; factions.len()], cmd == "play");
            if cmd == "hashes" {
                for h in r.hashes {
                    println!("{h}");
                }
            } else {
                println!("final: {:?} in {:.0} ms", r.scores, t.elapsed().as_secs_f64() * 1000.0);
            }
        }
        "balance" => {
            let world = load_world(&dir);
            let games = arg(&args, "--games").map(|s| s.parse().expect("games")).unwrap_or(350);
            let threads = arg(&args, "--threads")
                .map(|s| s.parse().expect("threads"))
                .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4));
            balance(&world, games, threads, parse_level(arg(&args, "--level")));
        }
        "replay" => {
            let world = load_world(&dir);
            replay(&world, args.get(2).expect("save path"));
        }
        _ => {
            println!("flyover play | balance | hashes | replay <save>   (--content DIR, default ./content)");
        }
    }
}
