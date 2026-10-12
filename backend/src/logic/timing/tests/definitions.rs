//! The five windows as defined: boundaries, scan ranges, grass buffers, season years.

use super::*;
use crate::logic::timing::series::{Crossing, Direction, SeasonDay};
use crate::logic::timing::windows::{
    establishment_days, season_year, specs_for, Boundary, WindowSpec,
};
use crate::models::timing::WindowPriority;

fn spec(grass: GrassType, id: WindowId) -> WindowSpec {
    specs_for(grass).into_iter().find(|s| s.id == id).unwrap()
}

fn soil(threshold_f: f64, direction: Direction, from: SeasonDay, to: SeasonDay) -> Boundary {
    Boundary::Soil(Crossing {
        threshold_f,
        direction,
        scan_from: from,
        scan_to: to,
    })
}

const FEB_1: SeasonDay = SeasonDay::new(0, 2, 1);
const JUL_31: SeasonDay = SeasonDay::new(0, 7, 31);
const DEC_31: SeasonDay = SeasonDay::new(0, 12, 31);

fn rising(threshold_f: f64) -> Boundary {
    soil(threshold_f, Direction::Rising, FEB_1, JUL_31)
}

fn falling(threshold_f: f64, month: u32, day: u32) -> Boundary {
    soil(
        threshold_f,
        Direction::Falling,
        SeasonDay::new(0, month, day),
        DEC_31,
    )
}

// @spec TIME-ENG-001
#[test]
fn the_five_windows_have_their_defined_boundaries() {
    let grass = GrassType::TallFescue;
    let spring_pre = spec(grass, WindowId::SpringPreEmergent);
    assert_eq!(spring_pre.priority, WindowPriority::Primary);
    assert_eq!(spring_pre.opens, rising(45.0));
    assert_eq!(spring_pre.ideal_from, Some(rising(50.0)));
    assert_eq!(spring_pre.ideal_until, None);
    assert_eq!(
        spring_pre.closes,
        Boundary::Earliest(vec![rising(55.0), Boundary::Gdd(200.0)])
    );

    let fall_pre = spec(grass, WindowId::FallPreEmergent);
    assert_eq!(fall_pre.priority, WindowPriority::Primary);
    assert_eq!(fall_pre.opens, falling(70.0, 8, 1));
    assert_eq!(fall_pre.ideal_from, None);
    assert_eq!(fall_pre.ideal_until, Some(falling(65.0, 8, 1)));
    assert_eq!(fall_pre.closes, falling(55.0, 8, 1));

    let (comfortable, minimum) = establishment_days(grass);
    let fall_seed = spec(grass, WindowId::FallSeeding);
    assert_eq!(fall_seed.priority, WindowPriority::Primary);
    assert_eq!(fall_seed.opens, falling(75.0, 8, 15));
    assert_eq!(
        fall_seed.ideal_until,
        Some(Boundary::BeforeFirstFreeze(comfortable))
    );
    assert_eq!(
        fall_seed.closes,
        Boundary::Earliest(vec![
            Boundary::BeforeFirstFreeze(minimum),
            falling(55.0, 8, 15),
        ])
    );

    let spring_seed = spec(grass, WindowId::SpringSeeding);
    assert_eq!(spring_seed.priority, WindowPriority::Secondary);
    assert_eq!(spring_seed.opens, rising(50.0));
    assert_eq!(spring_seed.closes, rising(65.0));
    assert_eq!(
        (spring_seed.ideal_from, spring_seed.ideal_until),
        (None, None)
    );

    let dormant = spec(grass, WindowId::DormantSeeding);
    assert_eq!(dormant.priority, WindowPriority::Optional);
    assert_eq!(
        dormant.opens,
        soil(
            40.0,
            Direction::Falling,
            SeasonDay::new(0, 11, 1),
            SeasonDay::new(1, 1, 31)
        )
    );
    assert_eq!(
        dormant.closes,
        soil(
            45.0,
            Direction::Rising,
            SeasonDay::new(1, 2, 1),
            SeasonDay::new(1, 7, 31)
        )
    );
}

// @spec TIME-ENG-002
#[test]
fn soil_crossings_scan_feb_to_july_rising_and_through_december_falling() {
    let dormant_ranges = [
        (SeasonDay::new(0, 11, 1), SeasonDay::new(1, 1, 31)),
        (SeasonDay::new(1, 2, 1), SeasonDay::new(1, 7, 31)),
    ];
    let mut seen = 0;
    for spec in specs_for(GrassType::TallFescue) {
        let mut soil_boundaries = Vec::new();
        for boundary in [
            Some(&spec.opens),
            spec.ideal_from.as_ref(),
            spec.ideal_until.as_ref(),
            Some(&spec.closes),
        ]
        .into_iter()
        .flatten()
        {
            match boundary {
                Boundary::Earliest(options) => soil_boundaries.extend(options.iter()),
                other => soil_boundaries.push(other),
            }
        }
        for boundary in soil_boundaries {
            let Boundary::Soil(c) = boundary else {
                continue;
            };
            seen += 1;
            if spec.id == WindowId::DormantSeeding {
                // The one window with its own scan ranges.
                assert!(dormant_ranges.contains(&(c.scan_from, c.scan_to)), "{c:?}");
                continue;
            }
            match c.direction {
                Direction::Rising => {
                    assert_eq!((c.scan_from, c.scan_to), (FEB_1, JUL_31), "{c:?}")
                }
                Direction::Falling => assert_eq!(c.scan_to, DEC_31, "{c:?}"),
            }
        }
    }
    assert_eq!(seen, 12, "every soil boundary checked");
}

// @spec TIME-ENG-003
#[test]
fn establishment_buffers_depend_on_the_grass() {
    assert_eq!(establishment_days(GrassType::KentuckyBluegrass), (60, 45));
    assert_eq!(establishment_days(GrassType::PerennialRyegrass), (35, 21));
    for grass in [
        GrassType::TallFescue,
        GrassType::FineFescue,
        GrassType::Mixed,
    ] {
        assert_eq!(establishment_days(grass), (45, 30), "{grass:?}");
    }
    // The fall seeding window uses them: ideal until the comfortable buffer, closed by
    // the minimum one.
    let kbg = spec(GrassType::KentuckyBluegrass, WindowId::FallSeeding);
    assert_eq!(kbg.ideal_until, Some(Boundary::BeforeFirstFreeze(60)));
    let Boundary::Earliest(options) = &kbg.closes else {
        panic!("composite close");
    };
    assert!(options.contains(&Boundary::BeforeFirstFreeze(45)));
}

// @spec TIME-ENG-005
#[test]
fn each_window_belongs_to_its_season_year() {
    let spring = [WindowId::SpringPreEmergent, WindowId::SpringSeeding];
    for id in spring {
        assert_eq!(season_year(id, date(2026, 7, 31)), 2026, "{id:?}");
        assert_eq!(season_year(id, date(2026, 8, 1)), 2027, "{id:?}");
        assert_eq!(season_year(id, date(2026, 12, 31)), 2027, "{id:?}");
        assert_eq!(season_year(id, date(2026, 1, 1)), 2026, "{id:?}");
    }
    let dormant = WindowId::DormantSeeding;
    assert_eq!(season_year(dormant, date(2026, 1, 1)), 2025);
    assert_eq!(season_year(dormant, date(2026, 6, 30)), 2025);
    assert_eq!(season_year(dormant, date(2026, 7, 1)), 2026);
    assert_eq!(season_year(dormant, date(2026, 12, 31)), 2026);
    for id in [WindowId::FallSeeding, WindowId::FallPreEmergent] {
        for (m, d) in [(1, 1), (6, 30), (8, 1), (12, 31)] {
            assert_eq!(season_year(id, date(2026, m, d)), 2026, "{id:?} {m}/{d}");
        }
    }
}
