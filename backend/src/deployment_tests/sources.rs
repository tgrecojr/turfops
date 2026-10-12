//! Properties of the backend's own sources: how database tests are gated and how queries
//! are written.

const CARGO_TOML: &str = include_str!("../../Cargo.toml");

/// Every `.rs` file under `backend/src`, with its path relative to `src`.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out
}

/// Whether the `mod` declaration of the module in `rel` is gated on `db-tests`.
fn gated_on_db_tests(rel: &str, files: &[(String, String)]) -> bool {
    let path = std::path::Path::new(rel);
    let stem = path.file_stem().unwrap().to_string_lossy();
    let dir = path.parent().unwrap().to_string_lossy().into_owned();
    let parents = if dir.is_empty() {
        vec!["main.rs".to_string()]
    } else {
        vec![format!("{dir}.rs"), format!("{dir}/mod.rs")]
    };
    let declaration = format!("mod {stem};");
    files
        .iter()
        .filter(|(p, _)| parents.contains(p))
        .flat_map(|(_, text)| {
            let lines: Vec<&str> = text.lines().collect();
            let at = lines
                .iter()
                .position(|l| l.trim_end().ends_with(&declaration));
            at.map(|i| {
                lines[..i]
                    .iter()
                    .rev()
                    .take_while(|l| {
                        l.trim_start().starts_with("#[") || l.trim_start().starts_with("//")
                    })
                    .any(|l| l.contains(r#"feature = "db-tests""#))
            })
        })
        .any(|gated| gated)
}

/// Whether `text` gates its database tests on `db-tests` inline, before the first one.
fn gated_inline(text: &str, marker: &str) -> bool {
    let first = text.find(marker).unwrap();
    text[..first].contains(r#"feature = "db-tests""#)
}

// @spec DEPLOY-CI-010
#[test]
fn postgres_tests_compile_only_with_db_tests_on_a_migrated_database() {
    assert!(CARGO_TOML.contains("\ndb-tests = []\n"));
    let marker = concat!("#[sqlx", "::test");
    let files = sources();
    let db_files: Vec<&(String, String)> =
        files.iter().filter(|(_, t)| t.contains(marker)).collect();
    assert!(!db_files.is_empty());
    for (rel, text) in db_files {
        assert!(
            gated_on_db_tests(rel, &files) || gated_inline(text, marker),
            "{rel} is not gated on db-tests"
        );
        for attr in text.lines().filter(|l| l.trim_start().starts_with(marker)) {
            assert!(
                attr.contains(r#"migrations = "src/db/migrations""#),
                "{rel}: {attr} must apply every migration"
            );
        }
    }
}

// @spec DEPLOY-START-005
#[test]
fn queries_use_the_runtime_api_only() {
    let macros: Vec<String> = [
        "query",
        "query_as",
        "query_scalar",
        "query_file",
        "query_file_as",
    ]
    .iter()
    .map(|m| format!("{m}!("))
    .collect();
    for (rel, text) in sources() {
        for m in &macros {
            assert!(!text.contains(&format!("sqlx::{m}")), "{rel}: sqlx::{m}");
            assert!(!text.contains(&format!(" {m}")), "{rel}: {m}");
        }
    }
    let offline = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".sqlx");
    assert!(!offline.exists(), "no offline query metadata");
}
