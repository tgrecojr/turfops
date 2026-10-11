//! Plan states: coverage, precedence, and the feeding activities.

use super::fixtures::*;
use super::*;

// @spec RULE-ENG-010
#[test]
fn every_covered_activity_gets_a_state() {
    let profile = profile();
    let states = season_states(&SeasonInputs {
        today: d(6, 1),
        env: &mild(),
        profile: &profile,
        history: &[],
        feed: &[],
    });
    let mut ids: Vec<&str> = states.iter().map(|(id, _)| *id).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            "early_fall_fert",
            "fall_herbicide",
            "grub_preventative",
            "mid_fall_fert",
            "spring_herbicide",
            "spring_nitrogen",
            "winterizer"
        ]
    );
}

// @spec RULE-ENG-010
#[test]
fn logged_work_outranks_the_feed_and_answers_do_not_matter() {
    let two_fed = Case::on(d(10, 10)).log(fed(d(9, 5))).log(fed(d(10, 6)));
    assert_eq!(
        two_fed
            .feed(feeding("fall_fert_mid"))
            .state("mid_fall_fert"),
        Done
    );
    let mut dismissed = feeding("fall_fert_early");
    dismissed.dismissed = true;
    assert_eq!(
        Case::on(d(9, 10)).feed(dismissed).state("early_fall_fert"),
        Due
    );
}

// @spec RULE-ENG-011
#[test]
fn spring_nitrogen_state() {
    assert_eq!(Case::on(d(1, 15)).state("spring_nitrogen"), NotYet);
    assert_eq!(
        Case::on(d(3, 20))
            .feed(rec("spring_n_wait"))
            .state("spring_nitrogen"),
        NotYet
    );
    assert_eq!(
        Case::on(d(3, 20))
            .feed(feeding("spring_n_ready"))
            .state("spring_nitrogen"),
        Due
    );
    assert_eq!(
        Case::on(d(3, 20))
            .feed(past_target("spring_n_ready"))
            .state("spring_nitrogen"),
        NotNeeded
    );
    let no_soil = env(None, None, None);
    assert_eq!(
        Case::on(d(3, 20))
            .env(no_soil.clone())
            .log(fed(d(3, 10)))
            .state("spring_nitrogen"),
        Done
    );
    let mut shrub = fed(d(3, 10));
    shrub.plant_id = Some(4);
    assert_eq!(
        Case::on(d(3, 20)).log(shrub).state("spring_nitrogen"),
        NotYet,
        "plant feeding"
    );
    assert_eq!(
        Case::on(d(1, 20))
            .log(fed(d(1, 10)))
            .state("spring_nitrogen"),
        NotYet,
        "January"
    );
    assert_eq!(
        Case::on(d(3, 20))
            .env(no_soil.clone())
            .state("spring_nitrogen"),
        Unknown
    );
    assert_eq!(
        Case::on(d(5, 2)).env(no_soil).state("spring_nitrogen"),
        Passed
    );
    assert_eq!(
        Case::on(d(4, 10))
            .env(env(Some(67.0), None, Some(67.0)))
            .state("spring_nitrogen"),
        Passed
    );
}

// @spec RULE-ENG-012
#[test]
fn fall_feeding_phases_follow_the_count_of_fall_feedings() {
    let sept = Case::on(d(9, 10)).feed(feeding("fall_fert_early"));
    assert_eq!(sept.state("early_fall_fert"), Due);
    assert_eq!(sept.state("mid_fall_fert"), NotYet);
    assert_eq!(sept.state("winterizer"), NotYet);

    let oct = Case::on(d(10, 10))
        .log(fed(d(9, 5)))
        .feed(feeding("fall_fert_mid"));
    assert_eq!(oct.state("early_fall_fert"), Done);
    assert_eq!(oct.state("mid_fall_fert"), Due);
    assert_eq!(oct.state("winterizer"), NotYet);

    let skipped_sept = Case::on(d(10, 10));
    assert_eq!(skipped_sept.state("early_fall_fert"), Passed);
    assert_eq!(
        skipped_sept.state("mid_fall_fert"),
        NotYet,
        "rule silent inside its month"
    );

    let potash = Case::on(d(10, 10)).log({
        let mut a = fed(d(9, 5));
        a.nitrogen_pct = Some(0.0);
        a
    });
    assert_eq!(
        potash.state("early_fall_fert"),
        Passed,
        "0-0-x is not a feeding"
    );
    assert_eq!(
        Case::on(d(9, 10))
            .log(fed(d(8, 30)))
            .state("early_fall_fert"),
        NotYet,
        "August"
    );

    let nov = Case::on(d(11, 15)).log(fed(d(9, 5))).log(fed(d(10, 6)));
    assert_eq!(
        nov.feed(past_target("fall_fert_winterizer"))
            .state("winterizer"),
        NotNeeded
    );
    let december = Case::on(d(12, 2));
    for id in ["early_fall_fert", "mid_fall_fert", "winterizer"] {
        assert_eq!(december.state(id), Passed, "{id}");
    }

    let blind = Case::on(d(10, 10)).env(env(None, None, None));
    assert_eq!(blind.state("early_fall_fert"), Passed);
    assert_eq!(blind.state("mid_fall_fert"), Unknown);
    assert_eq!(blind.state("winterizer"), NotYet);
}
