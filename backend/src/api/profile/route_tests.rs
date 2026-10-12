//! The ways a lawn profile can come to exist or go away, read from the source itself.

use std::path::{Path, PathBuf};

/// Every non-test Rust source under `src/`.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let name = path.file_name().unwrap().to_string_lossy();
            let test_only = name.ends_with("tests.rs") || name == "db_support.rs";
            if !test_only {
                out.push(path);
            }
        }
    }
}

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

    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
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
