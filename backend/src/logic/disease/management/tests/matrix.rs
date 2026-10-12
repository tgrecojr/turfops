//! The static efficacy matrix and per-disease programs.

use super::*;
use crate::models::Efficacy::{Excellent, Fair, Good};
use crate::models::FracClass::*;

/// (class, efficacy, curative, restricted, protects at Severe)
type Row = (FracClass, Efficacy, bool, bool, bool);

fn rows(disease: Disease) -> Vec<Row> {
    programs::program_for(disease)
        .options
        .iter()
        .map(|o| {
            (
                o.class,
                o.efficacy,
                o.curative,
                o.restricted,
                o.protects_at_severe,
            )
        })
        .collect()
}

// @spec DIS-MGMT-MATRIX-001
#[test]
fn every_option_records_its_ppa1_rating_and_the_mapping() {
    let source = include_str!("../programs.rs");
    assert!(source.contains("Univ. of Kentucky PPA-1"));
    assert!(source.contains("4 / 3.5 → Excellent, 3 / 2.5 → Good, 2 / 1.5 → Fair"));
    // Every option entry sits under a comment carrying its source rating.
    let lines: Vec<&str> = source.lines().map(str::trim).collect();
    let mut entries = 0;
    for (i, line) in lines.iter().enumerate() {
        let is_entry = line.starts_with("opt(FracClass::")
            || line.starts_with("OptionSpec {")
            || line.starts_with("const CHLOROTHALONIL");
        // The body of the `opt` constructor is not an option.
        if !is_entry || lines[i - 1].starts_with("const fn opt") {
            continue;
        }
        entries += 1;
        let comment: Vec<&str> = lines[..i]
            .iter()
            .rev()
            .take_while(|l| l.starts_with("//"))
            .copied()
            .collect();
        assert!(!comment.is_empty(), "no rating above: {line}");
        assert!(
            comment
                .iter()
                .any(|l| l.chars().any(|c| c.is_ascii_digit())),
            "no rating in: {comment:?}"
        );
    }
    let listed: usize = [
        Disease::BrownPatch,
        Disease::DollarSpot,
        Disease::PythiumBlight,
        Disease::GrayLeafSpot,
        Disease::RedThread,
    ]
    .iter()
    .map(|d| {
        programs::program_for(*d)
            .options
            .iter()
            .filter(|o| o.class != FracM5)
            .count()
    })
    .sum();
    // Each non-chlorothalonil option, plus the one shared chlorothalonil definition.
    assert_eq!(entries, listed + 1);
}

// @spec DIS-MGMT-MATRIX-002
#[test]
fn options_are_listed_in_preference_order() {
    assert_eq!(
        rows(Disease::BrownPatch),
        vec![
            (Frac11, Excellent, true, false, true),
            (Frac7, Excellent, true, false, true),
            (Frac3, Good, true, false, true),
            (Frac12, Good, false, false, true),
            (Frac1, Good, true, false, true),
            (FracM5, Good, false, true, true),
        ]
    );
    assert_eq!(
        rows(Disease::DollarSpot),
        vec![
            (Frac3, Excellent, true, false, true),
            (Frac7, Excellent, true, false, true),
            (Frac1, Good, true, false, true),
            (FracM5, Good, false, true, true),
        ]
    );
    assert_eq!(
        rows(Disease::PythiumBlight),
        vec![
            (Frac21, Excellent, true, false, true),
            (Frac28, Excellent, true, false, true),
            (Frac4, Good, true, false, true),
            (FracP07, Good, false, false, true),
            (Frac11, Good, false, false, false),
        ]
    );
    assert_eq!(
        rows(Disease::GrayLeafSpot),
        vec![
            (Frac1, Excellent, true, false, true),
            (Frac11, Excellent, true, false, true),
            (Frac3, Fair, false, false, true),
            (FracM5, Good, false, true, true),
        ]
    );
    assert_eq!(
        rows(Disease::RedThread),
        vec![
            (Frac11, Excellent, true, false, true),
            (Frac7, Excellent, true, false, true),
            (Frac3, Good, true, false, true),
        ]
    );
}

// @spec DIS-MGMT-MATRIX-003
#[test]
fn a_class_split_on_a_disease_is_left_out_not_averaged() {
    let dollar_spot = rows(Disease::DollarSpot);
    assert!(dollar_spot.iter().all(|r| r.0 != Frac11));
    // So a strobilurin never counts as dollar spot protection.
    let ctx = ctx_with(vec![app(7, 15, "Insignia")]);
    let m = plan(Disease::DollarSpot, RiskTier::High, &[], date(7, 20), &ctx);
    assert!(m.protection.is_none());
    assert!(m
        .preventative
        .options
        .iter()
        .all(|o| o.frac_class != Frac11));
}

// @spec DIS-MGMT-MATRIX-004
#[test]
fn each_disease_has_intervals_four_practices_and_notes() {
    let expected = [
        (Disease::BrownPatch, "14–28 days", "14 days"),
        (Disease::DollarSpot, "14–28 days", "14 days"),
        (Disease::PythiumBlight, "10–21 days", "5–10 days"),
        (Disease::GrayLeafSpot, "14–21 days", "10–14 days"),
        (Disease::RedThread, "14–28 days", "14–21 days"),
    ];
    for (disease, preventative, curative) in expected {
        let m = plan(
            disease,
            RiskTier::High,
            &[],
            date(7, 20),
            &DiseaseContext::default(),
        );
        assert_eq!(m.preventative.interval, preventative, "{disease:?}");
        assert_eq!(m.curative.interval, curative, "{disease:?}");
        assert_eq!(m.cultural.len(), 4, "{disease:?}");
        let spec = programs::program_for(disease);
        assert!(!spec.notes.is_empty(), "{disease:?}");
        assert!(spec.notes.iter().all(|n| m.notes.contains(&n.to_string())));
    }
}
