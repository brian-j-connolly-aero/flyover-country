use flyover_core::content::{FactionId, SiteKind, UnitKind};
use flyover_core::rules::{mission_check, normalize};
use flyover_core::state::{Event, Mission, MissionOrder, Orders, Target};
use flyover_core::{new_game, resolve_night, state_hash, testkit, GameConfig, GameState, World};

fn game(factions: &[FactionId]) -> (World, GameState) {
    let w = testkit::world();
    let st = new_game(
        &w,
        &GameConfig {
            seed: 7,
            factions: factions.to_vec(),
        },
    )
    .unwrap();
    (w, st)
}

fn sqs(st: &GameState, seat: u8, kind: UnitKind) -> Vec<u32> {
    st.squadrons_of(seat)
        .filter(|s| s.kind == kind)
        .map(|s| s.id)
        .collect()
}

fn order(seat: u8, list: &[(u32, Mission)]) -> Orders {
    Orders {
        seat,
        missions: list
            .iter()
            .map(|&(sq, mission)| MissionOrder { sq, mission })
            .collect(),
        ..Default::default()
    }
}

fn site(w: &World, f: FactionId, k: SiteKind) -> u16 {
    w.site_of(f, k)
}

#[test]
fn new_game_starts_level() {
    let (w, st) = game(&[
        FactionId::Pacific,
        FactionId::Texas,
        FactionId::Beehive,
        FactionId::FrontRange,
    ]);
    assert_eq!(st.night, 1);
    for seat in 0..4 {
        assert_eq!(st.squadrons_of(seat).count(), 4);
        assert_eq!(st.squadrons_of(seat).filter(|s| s.ace).count(), 1);
    }
    // Texas bombers fly with four planes.
    assert!(st
        .squadrons_of(1)
        .filter(|s| s.kind == UnitKind::Bomber)
        .all(|s| s.planes == 4));
    // Cascadia isn't playing: its sites are inactive.
    let cas = site(&w, FactionId::Cascadia, SiteKind::HQ);
    assert!(!st.sites[cas as usize].active);
}

#[test]
fn targets_open_in_stages() {
    let (w, st) = game(&[FactionId::Pacific, FactionId::Texas]);
    let b = st
        .squadron(sqs(&st, 0, UnitKind::Bomber)[0])
        .unwrap()
        .clone();
    let fly = |s| Mission::Fly {
        target: Target::Site { site: s },
    };
    assert!(mission_check(
        &w,
        &st,
        &b,
        &fly(site(&w, FactionId::Texas, SiteKind::Airfield))
    )
    .is_ok());
    assert!(mission_check(
        &w,
        &st,
        &b,
        &fly(site(&w, FactionId::Texas, SiteKind::Factory))
    )
    .is_err());
    assert!(mission_check(&w, &st, &b, &fly(site(&w, FactionId::Texas, SiteKind::HQ))).is_err());
    assert!(mission_check(
        &w,
        &st,
        &b,
        &fly(site(&w, FactionId::Pacific, SiteKind::Airfield))
    )
    .is_err());
    let mut later = st.clone();
    later.night = 6;
    assert!(mission_check(
        &w,
        &later,
        &b,
        &fly(site(&w, FactionId::Texas, SiteKind::HQ))
    )
    .is_ok());
    // Bombers don't patrol; fighters can't patrol over a rival.
    let home = w.site(site(&w, FactionId::Pacific, SiteKind::HQ)).pos;
    let away = w.site(site(&w, FactionId::Texas, SiteKind::HQ)).pos;
    assert!(mission_check(&w, &st, &b, &Mission::Patrol { at: home }).is_err());
    let f = st
        .squadron(sqs(&st, 0, UnitKind::Fighter)[0])
        .unwrap()
        .clone();
    assert!(mission_check(&w, &st, &f, &Mission::Patrol { at: home }).is_ok());
    assert!(mission_check(&w, &st, &f, &Mission::Patrol { at: away }).is_err());
}

#[test]
fn patrols_jump_raids_that_cross_them() {
    let (w, st) = game(&[FactionId::Pacific, FactionId::Texas]);
    let target = site(&w, FactionId::Texas, SiteKind::Airfield);
    let tpos = w.site(target).pos;
    let raid: Vec<(u32, Mission)> = sqs(&st, 0, UnitKind::Bomber)
        .into_iter()
        .map(|id| {
            (
                id,
                Mission::Fly {
                    target: Target::Site { site: target },
                },
            )
        })
        .collect();
    // Texas patrols right over its Airfield: the raid must cross it.
    let guard = sqs(&st, 1, UnitKind::Fighter);
    let o1 = order(
        1,
        &[
            (guard[0], Mission::Patrol { at: tpos }),
            (guard[1], Mission::Patrol { at: tpos }),
        ],
    );
    let (after, events) = resolve_night(&w, &st, &[order(0, &raid), o1]);
    let intercepts = events
        .iter()
        .filter(|e| matches!(e.e, Event::Intercept { .. }))
        .count();
    assert_eq!(intercepts, 2, "both patrols jump the raid");
    // With nobody patrolling, the raid gets through and strikes.
    let (_, quiet) = resolve_night(&w, &st, &[order(0, &raid), Orders::empty(1)]);
    assert!(quiet
        .iter()
        .all(|e| !matches!(e.e, Event::Intercept { .. })));
    assert!(quiet.iter().any(|e| matches!(e.e, Event::Strike { .. })));
    assert_eq!(after.night, 2);
}

#[test]
fn illegal_orders_become_rest() {
    let (w, st) = game(&[FactionId::Pacific, FactionId::Texas]);
    let b = sqs(&st, 0, UnitKind::Bomber)[0];
    let hq = site(&w, FactionId::Texas, SiteKind::HQ);
    let o = order(
        0,
        &[(
            b,
            Mission::Fly {
                target: Target::Site { site: hq },
            },
        )],
    );
    let n = normalize(&w, &st, &o);
    assert!(n.missions.is_empty());
    // Can't build what you can't afford: 6 Supply buys Texas one bomber, not two.
    let o = Orders {
        seat: 1,
        builds: vec![UnitKind::Bomber, UnitKind::Bomber],
        ..Default::default()
    };
    let n = normalize(&w, &st, &o);
    assert_eq!(n.builds.len(), 1);
}

#[test]
fn first_to_a_prize_holds_it_and_scores_at_dawn() {
    let (w, st) = game(&[FactionId::Beehive, FactionId::Texas]);
    let faithful = w
        .content
        .map
        .prizes
        .iter()
        .position(|p| p.id == "prize_faithful")
        .unwrap() as u16;
    let f = sqs(&st, 0, UnitKind::Fighter);
    let o = order(
        0,
        &[(
            f[1],
            Mission::Fly {
                target: Target::Prize { prize: faithful },
            },
        )],
    );
    let (after, events) = resolve_night(&w, &st, &[o, Orders::empty(1)]);
    assert!(events.iter().any(|e| matches!(e.e, Event::Hold { .. })));
    assert_eq!(after.prizes[faithful as usize].owner, Some(0));
    assert_eq!(after.seats[0].score, 1);
}

#[test]
fn bombs_bounce_off_the_bark_park_fence() {
    let (w, mut st) = game(&[FactionId::Pacific, FactionId::Texas]);
    st.night = 3;
    st.seats[1].doghouse = true;
    let target = site(&w, FactionId::Texas, SiteKind::Factory);
    let raid: Vec<(u32, Mission)> = sqs(&st, 0, UnitKind::Bomber)
        .into_iter()
        .map(|id| {
            (
                id,
                Mission::Fly {
                    target: Target::Site { site: target },
                },
            )
        })
        .collect();
    let park = Orders {
        seat: 1,
        retreat: true,
        ..Default::default()
    };
    let (after, events) = resolve_night(&w, &st, &[order(0, &raid), park]);
    assert!(events.iter().any(|e| matches!(e.e, Event::Bounce { .. })));
    assert_eq!(
        after.sites[target as usize].health,
        after.sites[target as usize].max
    );
    assert!(after.seats[1].fetch && after.seats[1].been_to_park);
    assert!(after.squadrons_of(1).count() >= 4);
}

#[test]
fn utah_watches_raids_passing_through_and_others_dont() {
    // Cascadia bombs Texas, flying over Utah and Colorado on the way.
    let (w, st) = game(&[
        FactionId::Cascadia,
        FactionId::Texas,
        FactionId::Beehive,
        FactionId::FrontRange,
    ]);
    let target = site(&w, FactionId::Texas, SiteKind::Airfield);
    let from = w
        .site(site(&w, FactionId::Cascadia, SiteKind::Airfield))
        .pos;
    let to = w.site(target).pos;
    let raid: Vec<(u32, Mission)> = sqs(&st, 0, UnitKind::Bomber)
        .into_iter()
        .map(|id| {
            (
                id,
                Mission::Fly {
                    target: Target::Site { site: target },
                },
            )
        })
        .collect();
    let over = |f: FactionId| {
        let len = flyover_core::geom::dist(from, to);
        (1..80)
            .map(|k| flyover_core::geom::along(from, to, len * k / 80))
            .find(|&p| w.region_at(p).and_then(|r| r.faction()) == Some(f))
    };
    let utah = over(FactionId::Beehive).expect("the path crosses Utah");
    let colorado = over(FactionId::FrontRange).expect("the path crosses Colorado");
    let orders = [
        order(0, &raid),
        Orders::empty(1),
        order(
            2,
            &[(
                sqs(&st, 2, UnitKind::Fighter)[1],
                Mission::Patrol { at: utah },
            )],
        ),
        order(
            3,
            &[(
                sqs(&st, 3, UnitKind::Fighter)[1],
                Mission::Patrol { at: colorado },
            )],
        ),
    ];
    let (_, events) = resolve_night(&w, &st, &orders);
    let jumped_by = |seat: u8| {
        events
            .iter()
            .any(|e| matches!(e.e, Event::Intercept { patrol_seat, .. } if patrol_seat == seat))
    };
    assert!(jumped_by(2), "Utah watches every raid");
    assert!(!jumped_by(3), "Colorado only guards its own sites");
}

/// Every seat sends every bomber at a rival Airfield and patrols at home, for
/// eight nights. The same inputs must give the same game, and nothing may go
/// out of bounds.
#[test]
fn full_game_is_deterministic_and_sane() {
    let play = || {
        let (w, mut st) = game(&[
            FactionId::Pacific,
            FactionId::Cascadia,
            FactionId::Texas,
            FactionId::GrainBelt,
        ]);
        let mut hashes = Vec::new();
        while !st.over(&w) {
            let orders: Vec<Orders> = (0..4u8)
                .map(|seat| {
                    let rival = st.faction((seat + 1) % 4);
                    let target = Target::Site {
                        site: w.site_of(rival, SiteKind::Airfield),
                    };
                    let home = w.site(w.site_of(st.faction(seat), SiteKind::HQ)).pos;
                    let mut list = Vec::new();
                    for s in st.squadrons_of(seat) {
                        let m = match s.kind {
                            UnitKind::Bomber => Mission::Fly { target },
                            UnitKind::Fighter if s.ace => Mission::Fly { target },
                            UnitKind::Fighter => Mission::Patrol { at: home },
                        };
                        list.push((s.id, m));
                    }
                    let mut o = order(seat, &list);
                    o.builds = vec![UnitKind::Bomber];
                    o
                })
                .collect();
            let (next, events) = resolve_night(&w, &st, &orders);
            assert!(matches!(events.last().unwrap().e, Event::Dawn(_)));
            for s in &next.sites {
                assert!(s.health >= 0 && s.health <= s.max);
            }
            for s in &next.squadrons {
                assert!(s.planes > 0 && s.planes <= s.max);
            }
            st = next;
            hashes.push(state_hash(&st));
        }
        assert_eq!(st.history.len(), 8);
        hashes
    };
    assert_eq!(play(), play());
}
