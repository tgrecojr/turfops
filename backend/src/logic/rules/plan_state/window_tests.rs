//! Plan states: broadleaf herbicide and grub control.

use super::fixtures::*;

// @spec RULE-ENG-013
#[test]
fn spring_herbicide_state() {
    let post = |on| applied(ApplicationType::PostEmergent, on);
    assert_eq!(Case::on(d(2, 10)).state("spring_herbicide"), NotYet);
    assert_eq!(
        Case::on(d(3, 15))
            .feed(rec("broadleaf_spring"))
            .state("spring_herbicide"),
        Due
    );
    assert_eq!(
        Case::on(d(4, 10))
            .log(post(d(4, 1)))
            .state("spring_herbicide"),
        Done
    );
    assert_eq!(Case::on(d(5, 2)).state("spring_herbicide"), Passed);
    let gdd = |g| env(Some(60.0), g, Some(60.0));
    assert_eq!(
        Case::on(d(4, 10))
            .env(gdd(Some(200.0)))
            .state("spring_herbicide"),
        Passed
    );
    assert_eq!(
        Case::on(d(4, 10)).env(gdd(None)).state("spring_herbicide"),
        Passed
    );
    assert_eq!(
        Case::on(d(4, 10))
            .env(gdd(Some(100.0)))
            .state("spring_herbicide"),
        NotYet
    );
    assert_eq!(
        Case::on(d(3, 15))
            .env(env(None, Some(80.0), None))
            .state("spring_herbicide"),
        Unknown
    );
}

// @spec RULE-ENG-014
#[test]
fn fall_herbicide_state() {
    let post = |on| applied(ApplicationType::PostEmergent, on);
    assert_eq!(
        Case::on(d(9, 25))
            .feed(rec("broadleaf_fall"))
            .state("fall_herbicide"),
        Due
    );
    assert_eq!(
        Case::on(d(10, 5))
            .log(post(d(9, 28)))
            .state("fall_herbicide"),
        Done
    );
    assert_eq!(Case::on(d(11, 2)).state("fall_herbicide"), Passed);
    let blind = env(None, None, None);
    assert_eq!(
        Case::on(d(9, 25))
            .env(blind.clone())
            .state("fall_herbicide"),
        Unknown
    );
    assert_eq!(
        Case::on(d(9, 10)).env(blind).state("fall_herbicide"),
        NotYet
    );
}

// @spec RULE-ENG-015
#[test]
fn grub_preventative_state() {
    assert_eq!(
        Case::on(d(6, 1))
            .feed(rec("grub_control_2026"))
            .state("grub_preventative"),
        Due
    );
    assert_eq!(
        Case::on(d(6, 1))
            .feed(rec("grub_control_late_2026"))
            .state("grub_preventative"),
        Due
    );
    let insecticide = applied(ApplicationType::Insecticide, d(5, 20));
    assert_eq!(
        Case::on(d(6, 1))
            .log(insecticide)
            .state("grub_preventative"),
        Done
    );
    assert_eq!(
        Case::on(d(7, 10))
            .env(env(Some(70.0), None, Some(70.0)))
            .state("grub_preventative"),
        Passed
    );
    assert_eq!(
        Case::on(d(7, 10))
            .env(env(Some(70.0), Some(800.0), Some(70.0)))
            .state("grub_preventative"),
        NotYet,
        "the GDD-extended window is still open"
    );
    assert_eq!(
        Case::on(d(6, 1))
            .env(env(Some(65.0), None, None))
            .state("grub_preventative"),
        Unknown,
        "no live soil"
    );
    assert_eq!(
        Case::on(d(4, 10))
            .env(env(None, Some(100.0), None))
            .state("grub_preventative"),
        NotYet
    );
}
