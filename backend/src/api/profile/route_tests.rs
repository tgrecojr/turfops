//! The ways a lawn profile can come to exist or go away, read from the source itself.

use crate::test_support::production_sources;

// @spec APP-PROF-011
#[test]
fn the_api_never_deletes_the_profile_and_creates_it_only_on_save_or_startup() {
    let main = include_str!("../../main.rs");
    let route = main
        .split(".route(")
        .find(|r| r.contains("\"/api/v1/profile\""))
        .expect("the profile route");
    assert!(
        route.contains("get(api::profile::get_profile).put(api::profile::update_profile)"),
        "{route}"
    );
    assert!(
        !route.contains("delete(") && !route.contains("post("),
        "{route}"
    );

    let files = production_sources();
    let mut creators = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let lower = text.to_lowercase();
        assert!(
            !lower.contains("delete from lawn_profiles")
                && !lower.contains("truncate lawn_profiles")
                && !lower.contains("truncate table lawn_profiles"),
            "{} removes lawn profiles",
            file.display()
        );
        if text.contains("create_lawn_profile(")
            && !text.contains("pub async fn create_lawn_profile")
        {
            creators.push(file.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    creators.sort();
    assert_eq!(creators, vec!["main.rs", "profile.rs"]);
}
