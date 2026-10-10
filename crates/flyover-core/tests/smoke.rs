use flyover_core::content::FactionId::*;
use flyover_core::*;

#[test]
fn content_loads_and_a_quiet_war_runs() {
    let w = testkit::world();
    let cfg = GameConfig { seed: 42, factions: vec![Pacific, Cascadia, FrontRange, Texas] };
    let mut st = new_game(&w, &cfg).unwrap();
    for _ in 0..8 {
        let orders: Vec<Orders> = (0..4).map(|s| carry_over(&w, &st, s)).collect();
        let (next, ev) = resolve_night(&w, &st, &orders);
        assert!(!ev.is_empty());
        st = next;
    }
    assert_eq!(st.night, 9);
    for s in &st.seats {
        println!("{:?} supply {} fuel {} score {} income {}/{}", s.faction, s.supply, s.fuel, s.score, s.income_supply, s.income_fuel);
    }
}
