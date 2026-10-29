//! The Dockerfile and `.dockerignore`, checked as text.

use super::DOCKERFILE;

const DOCKERIGNORE: &str = include_str!("../../../.dockerignore");

/// The Dockerfile's build stages, each starting at its `FROM` line.
fn stages() -> Vec<&'static str> {
    let starts: Vec<usize> = DOCKERFILE
        .match_indices("\nFROM ")
        .map(|(i, _)| i + 1)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(n, &start)| &DOCKERFILE[start..*starts.get(n + 1).unwrap_or(&DOCKERFILE.len())])
        .collect()
}

fn runtime() -> &'static str {
    stages().pop().unwrap()
}

/// The instruction lines (comments and blanks dropped) of `stage`.
fn instructions(stage: &str) -> Vec<&str> {
    stage
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

// @spec DEPLOY-IMAGE-001
#[test]
fn three_stages_each_on_a_digest_pinned_base() {
    let stages = stages();
    assert_eq!(stages.len(), 3, "frontend, backend, runtime");
    let from = |n: usize| stages[n].lines().next().unwrap();
    assert!(from(0).starts_with("FROM node:") && from(0).ends_with("AS frontend-build"));
    assert!(from(1).starts_with("FROM rust:") && from(1).ends_with("AS backend-build"));
    assert!(from(2).starts_with("FROM cgr.dev/chainguard/glibc-dynamic:"));
    for n in 0..3 {
        assert!(
            from(n).contains("@sha256:"),
            "pinned by digest: {}",
            from(n)
        );
    }
}

// @spec DEPLOY-IMAGE-002, DEPLOY-TZ-001
#[test]
fn the_runtime_holds_only_the_binary_the_spa_and_the_zone_database() {
    let lines = instructions(runtime());
    let copies: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| l.starts_with("COPY"))
        .collect();
    assert_eq!(copies.len(), 3, "{copies:?}");
    assert!(copies[0].contains("/app/target/release/turfops-backend ./turfops-backend"));
    assert!(copies[1].contains("/app/frontend/dist ./static"));
    assert!(copies[2].contains("--from=backend-build /usr/share/zoneinfo /usr/share/zoneinfo"));
    assert!(
        !lines.iter().any(|l| l.starts_with("RUN")),
        "no shell or package manager to run anything with"
    );
    assert!(
        stages()[1].contains("--no-install-recommends g++ tzdata"),
        "the zone database comes from the build stage's tzdata"
    );
}

// @spec DEPLOY-IMAGE-003
#[test]
fn the_runtime_base_supplies_the_shared_libraries() {
    let from = runtime().lines().next().unwrap();
    assert!(
        from.starts_with("FROM cgr.dev/chainguard/glibc-dynamic:"),
        "glibc-dynamic ships glibc, libstdc++.so.6 and libgcc_s.so.1: {from}"
    );
    let copied_libraries: Vec<&str> = instructions(runtime())
        .into_iter()
        .filter(|l| l.starts_with("COPY") && (l.contains(".so") || l.contains("/lib/")))
        .collect();
    assert!(
        copied_libraries.is_empty(),
        "no shared library of its own: {copied_libraries:?}"
    );
}

// @spec DEPLOY-IMAGE-004
#[test]
fn the_mcp_docs_reach_the_backend_build() {
    let backend = stages()[1];
    let copy = backend
        .find("COPY docs/timing-windows.md docs/agronomy-methods.md /docs/")
        .expect("both docs copied to /docs/");
    assert!(copy < backend.rfind("RUN cargo build --release").unwrap());
    for pattern in DOCKERIGNORE.lines().map(str::trim) {
        assert!(
            !(pattern.contains("**") || pattern.starts_with("docs")),
            "{pattern} would drop docs/*.md from the context"
        );
    }
    assert!(
        DOCKERIGNORE.lines().any(|l| l == "*.md"),
        "root-anchored *.md only"
    );
}

// @spec DEPLOY-IMAGE-005
#[test]
fn the_backend_runs_as_uid_65532_and_owns_its_files() {
    let lines = instructions(runtime());
    for target in ["./turfops-backend", "./static"] {
        let copy = lines
            .iter()
            .find(|l| l.starts_with("COPY") && l.ends_with(target))
            .unwrap();
        assert!(copy.contains("--chown=65532:65532"), "{copy}");
    }
    assert!(lines.contains(&"USER 65532:65532"));
}

// @spec DEPLOY-IMAGE-006
#[test]
fn the_runtime_serves_the_spa_on_3000_through_the_binary() {
    let lines = instructions(runtime());
    for expected in [
        "ENV STATIC_DIR=/app/static",
        "EXPOSE 3000",
        r#"ENTRYPOINT ["./turfops-backend"]"#,
    ] {
        assert!(lines.contains(&expected), "{expected}");
    }
}

// @spec DEPLOY-IMAGE-007
#[test]
fn no_environment_file_enters_the_build_context() {
    let patterns: Vec<&str> = DOCKERIGNORE.lines().map(str::trim).collect();
    for expected in [".env", ".env.*", "!.env.example"] {
        assert!(patterns.contains(&expected), "{expected}");
    }
    assert!(
        !DOCKERFILE.contains("COPY . "),
        "only named paths are copied"
    );
}

// @spec DEPLOY-TZ-002
#[test]
fn the_image_sets_no_timezone_of_its_own() {
    for line in instructions(DOCKERFILE) {
        assert!(!line.contains("TZ="), "{line}");
        assert!(!line.starts_with("ENV TZ"), "{line}");
    }
    assert!(
        super::COMPOSE.contains("TZ: ${TZ:-"),
        "the example deployment supplies one"
    );
}
