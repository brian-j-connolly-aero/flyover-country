use flyover_core::content::FactionId::*;
use flyover_core::*;

#[test]
fn content_loads_and_a_quiet_war_runs() {
    let w = testkit::world();
    let cfg = GameConfig {
        seed: 42,
        factions: vec![Pacific, Cascadia, FrontRange, Texas],
    };
    let mut st = new_game(&w, &cfg).unwrap();
    for _ in 0..8 {
        let orders: Vec<Orders> = (0..4).map(|s| carry_over(&w, &st, s)).collect();
        let (next, ev) = resolve_night(&w, &st, &orders);
        assert!(!ev.is_empty());
        st = next;
    }
    assert_eq!(st.night, 9);
    for s in &st.seats {
        assert!(s.income_supply > 0 && s.income_fuel > 0);
    }
}

#[test]
fn unpicked_home_regions_fall() {
    let w = testkit::world();
    let st = new_game(
        &w,
        &GameConfig {
            seed: 1,
            factions: vec![Pacific, Cascadia, FrontRange, Texas],
        },
    )
    .unwrap();
    let hoover = w.node("drm_jewel").unwrap();
    let vegas = w.node("drm_capital").unwrap();
    assert!(
        !st.nodes[hoover as usize].active,
        "a fallen crown jewel is only a landmark"
    );
    assert_eq!(st.nodes[vegas as usize].owner, None);
    assert!(rules::capturable(&w, &st, vegas));
    assert_eq!(rules::objective(&w, &st, vegas), 1);
}

#[test]
fn escalation_opens_targets_in_stages() {
    let w = testkit::world();
    let mut st = new_game(
        &w,
        &GameConfig {
            seed: 1,
            factions: vec![Pacific, Cascadia, FrontRange, Texas],
        },
    )
    .unwrap();
    let seattle = w.node("cas_seattle").unwrap();
    let radar = w.node("cas_radar").unwrap();
    let coulee = w.node("cas_jewel").unwrap();
    assert!(rules::attack_check(&w, &st, 0, radar).is_ok());
    assert!(rules::attack_check(&w, &st, 0, seattle).is_err());
    st.night = 3;
    assert!(rules::attack_check(&w, &st, 0, seattle).is_ok());
    assert!(rules::attack_check(&w, &st, 0, coulee).is_err());
    st.night = 6;
    assert!(rules::attack_check(&w, &st, 0, coulee).is_ok());
}

#[test]
fn utah_never_throws_the_first_punch() {
    let w = testkit::world();
    let mut st = new_game(
        &w,
        &GameConfig {
            seed: 1,
            factions: vec![Beehive, Texas, Cascadia, Pacific],
        },
    )
    .unwrap();
    let radar = w.node("tex_radar_bigbend").unwrap();
    assert!(rules::attack_check(&w, &st, 0, radar).is_err());
    st.beehive_grudge.push(1);
    assert!(rules::attack_check(&w, &st, 0, radar).is_ok());
    // Service Project: Utah can guard a neighbor's node.
    let dallas = w.node("tex_dallas").unwrap();
    assert!(rules::guard_check(&w, &st, 0, dallas).is_ok());
    assert!(rules::guard_check(&w, &st, 1, w.node("cas_seattle").unwrap()).is_err());
}

#[test]
fn a_cut_junction_costs_income() {
    let w = testkit::world();
    let st = new_game(
        &w,
        &GameConfig {
            seed: 1,
            factions: vec![Pacific, Cascadia, FrontRange, Texas],
        },
    )
    .unwrap();
    let full = economy::income(&w, &st, 0, 1);
    let mut health: Vec<u8> = st.nodes.iter().map(|n| n.health).collect();
    health[w.node("pac_jct_fresno").unwrap() as usize] = 0;
    let cut = economy::income_with(&w, &st, 0, &health, 1);
    assert!(
        cut.supply < full.supply,
        "Fresno carries the south's Supply"
    );
    assert!(cut.fuel < full.fuel, "and Bakersfield's Fuel");
}
