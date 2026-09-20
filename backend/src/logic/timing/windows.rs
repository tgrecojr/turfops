use super::series::{Crossing, Direction, SeasonDay};
use crate::models::timing::{WindowId, WindowPriority};
use crate::models::GrassType;
use chrono::{Datelike, NaiveDate};

// Soil thresholds are 5-day means at 5 cm (°F).
/// Earliest worthwhile spring pre-emergent: the barrier is down well before germination.
pub const SPRING_PRE_EM_EARLY_F: f64 = 45.0;
/// Start of the ideal spring pre-emergent stretch.
pub const SPRING_PRE_EM_IDEAL_F: f64 = 50.0;
/// Crabgrass germinates once the top 2 in of soil holds about 55 °F.
pub const CRABGRASS_GERMINATION_F: f64 = 55.0;
/// Running GDD (base 50 °F) by which crabgrass germination is under way.
pub const CRABGRASS_GERMINATION_GDD: f64 = 200.0;
/// Poa annua and other winter annuals begin germinating as soil cools through 70 °F.
pub const FALL_PRE_EM_OPEN_F: f64 = 70.0;
/// Germination is well under way below this: a fall barrier is increasingly late.
pub const FALL_PRE_EM_LATE_F: f64 = 65.0;
/// Most winter annuals are up, and cool-season seed germinates too slowly to establish.
pub const FALL_COOL_LIMIT_F: f64 = 55.0;
/// Above this, seedlings face heat and disease pressure.
pub const SEEDING_WARM_LIMIT_F: f64 = 75.0;
/// Cool-season seed starts germinating in spring.
pub const SPRING_SEEDING_OPEN_F: f64 = 50.0;
/// Spring seeding past this runs into summer stress before establishment.
pub const SPRING_SEEDING_CLOSE_F: f64 = 65.0;
/// Cold enough that dormant seed will not sprout before winter.
pub const DORMANT_SEEDING_OPEN_F: f64 = 40.0;
/// Soil warming back through this ends dormant seeding; it is spring seeding from here.
pub const DORMANT_SEEDING_CLOSE_F: f64 = 45.0;

/// One edge of a window.
#[derive(Debug, Clone, PartialEq)]
pub enum Boundary {
    Soil(Crossing),
    /// This many days before the typical (median) first fall freeze.
    BeforeFirstFreeze(i64),
    /// Running GDD (base 50 °F) reaches this total.
    Gdd(f64),
    /// Whichever of these comes first.
    Earliest(Vec<Boundary>),
}

impl Boundary {
    pub fn label(&self) -> String {
        match self {
            Boundary::Soil(c) => {
                let verb = match c.direction {
                    Direction::Rising => "warms",
                    Direction::Falling => "cools",
                };
                format!("5 cm soil {verb} to {:.0}°F", c.threshold_f)
            }
            Boundary::BeforeFirstFreeze(days) => {
                format!("{days} days before the typical first fall freeze")
            }
            Boundary::Gdd(total) => format!("{total:.0} growing degree days (base 50°F)"),
            Boundary::Earliest(options) => {
                let labels: Vec<String> = options.iter().map(Boundary::label).collect();
                format!("{}, whichever comes first", labels.join(" or "))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec {
    pub id: WindowId,
    pub name: &'static str,
    pub priority: WindowPriority,
    pub opens: Boundary,
    /// When set, the window is merely `Open` before this and `Ideal` after it.
    pub ideal_from: Option<Boundary>,
    /// When set, the window is `Closing` after this.
    pub ideal_until: Option<Boundary>,
    pub closes: Boundary,
}

fn rising(threshold_f: f64) -> Boundary {
    // Feb 1 keeps a January thaw from opening a spring window.
    Boundary::Soil(Crossing {
        threshold_f,
        direction: Direction::Rising,
        scan_from: SeasonDay::new(0, 2, 1),
        scan_to: SeasonDay::new(0, 7, 31),
    })
}

fn falling(threshold_f: f64, scan_from: SeasonDay) -> Boundary {
    Boundary::Soil(Crossing {
        threshold_f,
        direction: Direction::Falling,
        scan_from,
        scan_to: SeasonDay::new(0, 12, 31),
    })
}

const AUG_1: SeasonDay = SeasonDay::new(0, 8, 1);
/// Seeding before mid-August puts seedlings into peak heat and disease pressure
/// even in a cool spell.
const AUG_15: SeasonDay = SeasonDay::new(0, 8, 15);

/// Days a new stand needs before the first freeze: `(comfortable, minimum)`. Kentucky
/// bluegrass is slow to germinate (2–4 weeks); perennial ryegrass is up in under one.
pub fn establishment_days(grass: GrassType) -> (i64, i64) {
    match grass {
        GrassType::KentuckyBluegrass => (60, 45),
        GrassType::PerennialRyegrass => (35, 21),
        _ => (45, 30),
    }
}

fn pre_emergent_specs() -> Vec<WindowSpec> {
    vec![
        WindowSpec {
            id: WindowId::SpringPreEmergent,
            name: "Spring Pre-Emergent",
            priority: WindowPriority::Primary,
            opens: rising(SPRING_PRE_EM_EARLY_F),
            ideal_from: Some(rising(SPRING_PRE_EM_IDEAL_F)),
            ideal_until: None,
            closes: Boundary::Earliest(vec![
                rising(CRABGRASS_GERMINATION_F),
                Boundary::Gdd(CRABGRASS_GERMINATION_GDD),
            ]),
        },
        WindowSpec {
            id: WindowId::FallPreEmergent,
            name: "Fall Pre-Emergent",
            priority: WindowPriority::Primary,
            opens: falling(FALL_PRE_EM_OPEN_F, AUG_1),
            ideal_from: None,
            ideal_until: Some(falling(FALL_PRE_EM_LATE_F, AUG_1)),
            closes: falling(FALL_COOL_LIMIT_F, AUG_1),
        },
    ]
}

fn seeding_specs(grass: GrassType) -> Vec<WindowSpec> {
    let (comfortable, minimum) = establishment_days(grass);
    vec![
        WindowSpec {
            id: WindowId::FallSeeding,
            name: "Fall Seeding & Overseeding",
            priority: WindowPriority::Primary,
            opens: falling(SEEDING_WARM_LIMIT_F, AUG_15),
            ideal_from: None,
            ideal_until: Some(Boundary::BeforeFirstFreeze(comfortable)),
            closes: Boundary::Earliest(vec![
                Boundary::BeforeFirstFreeze(minimum),
                falling(FALL_COOL_LIMIT_F, AUG_15),
            ]),
        },
        WindowSpec {
            id: WindowId::SpringSeeding,
            name: "Spring Seeding",
            priority: WindowPriority::Secondary,
            opens: rising(SPRING_SEEDING_OPEN_F),
            ideal_from: None,
            ideal_until: None,
            closes: rising(SPRING_SEEDING_CLOSE_F),
        },
        WindowSpec {
            id: WindowId::DormantSeeding,
            name: "Dormant Seeding",
            priority: WindowPriority::Optional,
            opens: Boundary::Soil(Crossing {
                threshold_f: DORMANT_SEEDING_OPEN_F,
                direction: Direction::Falling,
                scan_from: SeasonDay::new(0, 11, 1),
                scan_to: SeasonDay::new(1, 1, 31),
            }),
            ideal_from: None,
            ideal_until: None,
            closes: Boundary::Soil(Crossing {
                threshold_f: DORMANT_SEEDING_CLOSE_F,
                direction: Direction::Rising,
                scan_from: SeasonDay::new(1, 2, 1),
                scan_to: SeasonDay::new(1, 7, 31),
            }),
        },
    ]
}

/// Pre-emergent windows apply to any turf; the seeding windows are cool-season only.
pub fn specs_for(grass: GrassType) -> Vec<WindowSpec> {
    let mut specs = pre_emergent_specs();
    if grass.is_cool_season() {
        specs.extend(seeding_specs(grass));
    }
    specs
}

/// Calendar year a window opens in, for the season `today` belongs to. Spring windows
/// roll to next year once summer ends; dormant seeding opened last year until July.
pub fn season_year(id: WindowId, today: NaiveDate) -> i32 {
    let year = today.year();
    match id {
        WindowId::SpringPreEmergent | WindowId::SpringSeeding if today.month() >= 8 => year + 1,
        WindowId::DormantSeeding if today.month() <= 6 => year - 1,
        _ => year,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    #[test]
    fn warm_season_turf_gets_only_pre_emergent_windows() {
        let ids: Vec<WindowId> = specs_for(GrassType::Bermuda).iter().map(|s| s.id).collect();
        assert_eq!(
            ids,
            vec![WindowId::SpringPreEmergent, WindowId::FallPreEmergent]
        );
        assert_eq!(specs_for(GrassType::TallFescue).len(), 5);
    }

    #[test]
    fn bluegrass_needs_a_longer_runway_than_ryegrass() {
        let specs = specs_for(GrassType::KentuckyBluegrass);
        let fall = specs
            .iter()
            .find(|s| s.id == WindowId::FallSeeding)
            .unwrap();
        assert_eq!(fall.ideal_until, Some(Boundary::BeforeFirstFreeze(60)));
        assert!(establishment_days(GrassType::PerennialRyegrass).1 < 30);
    }

    #[test]
    fn season_year_rolls_by_window() {
        assert_eq!(season_year(WindowId::SpringPreEmergent, date(4, 1)), 2026);
        assert_eq!(season_year(WindowId::SpringPreEmergent, date(9, 20)), 2027);
        assert_eq!(season_year(WindowId::FallSeeding, date(3, 1)), 2026);
        assert_eq!(season_year(WindowId::DormantSeeding, date(1, 15)), 2025);
        assert_eq!(season_year(WindowId::DormantSeeding, date(9, 20)), 2026);
    }

    #[test]
    fn composite_label_names_every_trigger() {
        let specs = pre_emergent_specs();
        assert_eq!(
            specs[0].closes.label(),
            "5 cm soil warms to 55°F or 200 growing degree days (base 50°F), whichever comes first"
        );
    }
}
