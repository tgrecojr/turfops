//! The container files and supply-chain lists, checked as text.

const DOCKERFILE: &str = include_str!("../../Dockerfile");
const COMPOSE: &str = include_str!("../../docker-compose.yml");
const AUDIT: &str = include_str!("../.cargo/audit.toml");
const OSV: &str = include_str!("../osv-scanner.toml");
const LOCK: &str = include_str!("../Cargo.lock");

mod binary;
mod ci;
mod compose;
mod image;
#[cfg(unix)]
mod shutdown;
mod sources;
mod startup;
#[cfg(feature = "db-tests")]
mod startup_db;

// @spec DEPLOY-IMAGE-008
#[test]
fn the_image_declares_a_health_check_the_binary_runs() {
    assert!(
        DOCKERFILE.contains(r#"HEALTHCHECK"#)
            && DOCKERFILE.contains(r#""./turfops-backend", "--healthcheck""#),
        "exec-form health check through the binary (no shell or curl in the runtime)"
    );
}

// @spec DEPLOY-IMAGE-009
#[test]
fn the_runtime_user_is_declared() {
    let runtime = DOCKERFILE.rsplit("FROM ").next().unwrap();
    assert!(runtime.contains("USER 65532:65532"));
}

// @spec DEPLOY-IMAGE-010
#[test]
fn the_frontend_installs_without_lifecycle_scripts() {
    assert!(DOCKERFILE.contains("npm ci --ignore-scripts"));
}

// @spec DEPLOY-COMPOSE-007
#[test]
fn postgres_is_published_on_loopback_only() {
    assert!(COMPOSE.contains(r#""127.0.0.1:5433:5432""#));
    assert!(COMPOSE.contains(r#""3000:3000""#));
}

// @spec DEPLOY-COMPOSE-009
#[test]
fn the_volume_mounts_where_postgres_18_keeps_its_data() {
    assert!(COMPOSE.contains("turfops_data:/var/lib/postgresql\n"));
    assert!(!COMPOSE.contains("/var/lib/postgresql/data"));
}

/// Advisory ids in an ignore list, each with the crate it names in a comment line
/// `# <ID> (crate <name>): <justification>`.
// @spec DEPLOY-COMPOSE-010
#[test]
fn compose_supplies_no_stand_in_coordinates() {
    assert!(
        COMPOSE.contains("OWM_LATITUDE: ${OWM_LATITUDE:-}\n"),
        "no 0 default"
    );
    assert!(
        COMPOSE.contains("OWM_LONGITUDE: ${OWM_LONGITUDE:-}\n"),
        "no 0 default"
    );
}

fn ignored(list: &str) -> Vec<(String, Option<String>)> {
    list.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(|l| {
            l.split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|id| id.starts_with("RUSTSEC-") || id.starts_with("GHSA-"))
        .map(|id| {
            let marker = format!("# {id} (crate ");
            let krate = list.lines().find_map(|l| {
                l.trim_start()
                    .strip_prefix(&marker)
                    .and_then(|rest| rest.split(')').next())
                    .map(str::to_string)
            });
            (id, krate)
        })
        .collect()
}

// @spec DEPLOY-CI-009
#[test]
fn an_advisory_is_ignored_only_while_its_crate_is_locked() {
    for (list, name) in [(AUDIT, "audit.toml"), (OSV, "osv-scanner.toml")] {
        for (id, krate) in ignored(list) {
            let krate = krate.unwrap_or_else(|| panic!("{name}: {id} names no crate"));
            assert!(
                LOCK.contains(&format!("name = \"{krate}\"\n")),
                "{name}: {id} ignores {krate}, which is no longer in Cargo.lock"
            );
        }
    }
}
