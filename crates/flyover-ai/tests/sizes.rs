//! Any number of sides from two to seven plays a whole war with the bots.

use flyover_ai::{suggest, Level};
use flyover_core::content::FactionId;
use flyover_core::state::{Event, Orders};
use flyover_core::{new_game, resolve_night, testkit, GameConfig};

#[test]
fn two_to_seven_sides_play_a_whole_war() {
    let w = testkit::world();
    for n in 2..=7 {
        for seed in [3u64, 41, 977] {
            let factions: Vec<FactionId> = FactionId::ALL
                .iter()
                .copied()
                .cycle()
                .skip(seed as usize % 7)
                .take(n)
                .collect();
            let mut st = new_game(
                &w,
                &GameConfig {
                    seed,
                    factions: factions.clone(),
                },
            )
            .expect("new game");
            while !st.over(&w) {
                let orders: Vec<Orders> = (0..n as u8)
                    .map(|seat| suggest(&w, &st, seat, Level::Normal))
                    .collect();
                let (next, events) = resolve_night(&w, &st, &orders);
                assert!(matches!(events.last().unwrap().e, Event::Dawn(_)));
                assert!(next
                    .sites
                    .iter()
                    .all(|s| s.health >= 0 && s.health <= s.max));
                assert!(next
                    .squadrons
                    .iter()
                    .all(|s| s.planes > 0 && s.planes <= s.max));
                st = next;
            }
            assert_eq!(st.seats.len(), n);
            assert!(
                st.seats.iter().all(|s| s.score >= 0),
                "{n} sides, seed {seed}"
            );
            assert!(
                st.seats.iter().any(|s| s.score > 0),
                "{n} sides, seed {seed}: nobody scored"
            );
        }
    }
    let one = new_game(
        &w,
        &GameConfig {
            seed: 1,
            factions: vec![FactionId::Texas],
        },
    );
    assert!(one.is_err());
}
