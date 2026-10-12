//! The ways a lawn profile can come to exist or go away, read from the source itself.

use std::path::{Path, PathBuf};

/// The production module tree: every file reachable from `main.rs` through `mod` items
/// that are not gated on `test`. Test modules and test helpers are left out by how they are
/// declared, whatever their file names.
fn sources(file: &Path, out: &mut Vec<PathBuf>) {
    out.push(file.to_path_buf());
    let text = std::fs::read_to_string(file).unwrap();
    let stem = file.file_stem().unwrap().to_string_lossy();
    let dir = if matches!(stem.as_ref(), "main" | "lib" | "mod") {
        file.parent().unwrap().to_path_buf()
    } else {
        file.with_extension("")
    };
    let mut test_gated = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("#[cfg(") {
            test_gated = line.contains("test");
            continue;
        }
        let decl = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub(super) "))
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        if let Some(name) = decl.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
            if !test_gated {
                let flat = dir.join(format!("{name}.rs"));
                let nested = dir.join(name).join("mod.rs");
                sources(if flat.exists() { &flat } else { &nested }, out);
            }
        }
        if !line.starts_with("#[") {
            test_gated = false;
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
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"),
        &mut files,
    );
    assert!(files.len() > 50, "walked the module tree: {}", files.len());
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
