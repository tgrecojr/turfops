//! The CI workflows, Renovate and the PostgreSQL test gate, checked as text.

const CI: &str = include_str!("../../../.github/workflows/ci.yml");
const SUPPLY_CHAIN: &str = include_str!("../../../.github/workflows/supply-chain.yml");
const PUBLISH: &str = include_str!("../../../.github/workflows/docker-publish.yml");
const RETENTION: &str = include_str!("../../../.github/workflows/ghcr-retention.yml");
const RENOVATE: &str = include_str!("../../../renovate.json5");
const PROTECTION: &str = include_str!("../../../scripts/github-protection.sh");

/// A JSON5 config written as JSON plus whole-line `//` comments, parsed as JSON.
fn json_without_comments(text: &str) -> serde_json::Value {
    let json: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::from_str(&json).expect("JSON once the comment lines are dropped")
}

/// The text of job `name` in a workflow (up to the next job), one `\n` per line.
fn job(workflow: &str, name: &str) -> String {
    let header = format!("  {name}:");
    let lines: Vec<&str> = workflow
        .lines()
        .skip_while(|l| *l != header)
        .skip(1)
        .take_while(|l| l.is_empty() || l.starts_with("   "))
        .collect();
    assert!(!lines.is_empty(), "job {name}");
    lines.join("\n") + "\n"
}

fn on_main(workflow: &str) -> bool {
    workflow.contains("  push:\n    branches: [main]")
        && workflow.contains("  pull_request:\n    branches: [main]")
}

// @spec DEPLOY-CI-001
#[test]
fn ci_gates_the_backend_with_postgres_and_the_frontend_on_node_26() {
    assert!(on_main(CI));
    let backend = job(CI, "backend");
    for step in [
        "run: cargo fmt --check",
        "run: cargo clippy -- -D warnings",
        "run: cargo test --features db-tests",
        "image: postgres:18\n",
        "DATABASE_URL: postgres://",
    ] {
        assert!(backend.contains(step), "backend: {step}");
    }
    let frontend = job(CI, "frontend");
    for step in ["node-version: 26", "run: npm ci\n", "run: npm run lint"] {
        assert!(frontend.contains(step), "frontend: {step}");
    }
}

// @spec DEPLOY-CI-002
#[test]
fn every_push_and_pr_runs_the_supply_chain_scan() {
    assert!(on_main(CI));
    assert!(job(CI, "supply-chain").contains("uses: ./.github/workflows/supply-chain.yml"));
    for step in [
        "run: npm ci --ignore-scripts",
        "run: npm audit signatures",
        "--org grecolabs",
        "./frontend\n",
        "uses: google/osv-scanner-action/",
        "--recursive",
        "run: cargo audit",
    ] {
        assert!(SUPPLY_CHAIN.contains(step), "{step}");
    }
}

// @spec DEPLOY-CI-003
#[test]
fn without_the_socket_secret_only_the_socket_step_is_skipped() {
    assert!(SUPPLY_CHAIN.contains("SOCKET_SECURITY_API_KEY:\n"));
    assert!(
        SUPPLY_CHAIN.contains("required: false"),
        "bot PRs have no secrets"
    );
    let conditions: Vec<&str> = SUPPLY_CHAIN
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("if:"))
        .collect();
    assert_eq!(
        conditions,
        ["if: ${{ env.SOCKET_SECURITY_API_KEY != '' }}"],
        "the one conditional step"
    );
    let socket = SUPPLY_CHAIN.find("Socket Security scan").unwrap();
    let condition = SUPPLY_CHAIN.find("if: ${{ env.SOCKET").unwrap();
    let next_step = SUPPLY_CHAIN[socket..].find("- name:").unwrap() + socket;
    assert!(socket < condition && condition < next_step);
}

// @spec DEPLOY-CI-004
#[test]
fn a_pr_builds_the_image_after_the_scan_without_pushing() {
    assert!(PUBLISH.contains("  pull_request:\n    branches: [main]"));
    let build = job(PUBLISH, "build-and-push");
    assert!(build.contains("needs: supply-chain"));
    assert!(job(PUBLISH, "supply-chain").contains("uses: ./.github/workflows/supply-chain.yml"));
    assert!(build.contains("push: ${{ github.event_name != 'pull_request' }}"));
    assert!(
        super::DOCKERFILE.contains("RUN npm run build"),
        "the image build includes the frontend build"
    );
}

// @spec DEPLOY-CI-005
#[test]
fn main_and_tags_push_a_signed_attested_image_to_ghcr() {
    assert!(PUBLISH.contains("    branches: [main]\n    tags: [\"v*\"]"));
    assert!(
        PUBLISH.contains("REGISTRY: ghcr.io")
            && PUBLISH.contains("IMAGE_NAME: ${{ github.repository }}")
    );
    let build = job(PUBLISH, "build-and-push");
    for expected in [
        "type=raw,value=latest,enable={{is_default_branch}}",
        "type=semver,pattern={{version}}",
        "type=sha",
        "id-token: write",
        "run: cosign sign --yes \"${IMAGE}@${DIGEST}\"",
        "format: spdx-json",
        "uses: actions/attest@",
        "sbom-path: sbom.spdx.json",
        "uses: actions/attest-build-provenance@",
    ] {
        assert!(build.contains(expected), "{expected}");
    }
    let pushed_only = build
        .split("- name:")
        .skip(1)
        .filter(|step| step.contains("cosign") || step.contains("attest") || step.contains("SBOM"))
        .all(|step| step.contains("if: github.event_name != 'pull_request'"));
    assert!(
        pushed_only,
        "signing and attestations only for pushed images"
    );
}

// @spec DEPLOY-CI-006
#[test]
fn retention_keeps_latest_and_five_tagged_versions_weekly() {
    assert!(RETENTION.contains("workflow_dispatch:"));
    assert!(RETENTION.contains("- cron: '0 6 * * 1'"));
    for expected in [
        "keep-n-tagged: 5",
        "exclude-tags: latest",
        "delete-untagged: true",
    ] {
        assert!(RETENTION.contains(expected), "{expected}");
    }
}

// @spec DEPLOY-CI-007
#[test]
fn renovate_pins_groups_soaks_and_labels_as_agreed() {
    let config = json_without_comments(RENOVATE);
    assert!(
        RENOVATE.contains("@spec DEPLOY-CI-007"),
        "the config carries its spec"
    );
    let rules = config["packageRules"].as_array().unwrap();
    let rule = |pred: &dyn Fn(&serde_json::Value) -> bool| {
        rules
            .iter()
            .find(|r| pred(r))
            .unwrap_or_else(|| panic!("no matching rule"))
    };
    let types = |r: &serde_json::Value| -> Vec<String> {
        serde_json::from_value(r["matchUpdateTypes"].clone()).unwrap_or_default()
    };
    let minor_patch = |r: &serde_json::Value| types(r) == ["minor", "patch"];

    assert_eq!(
        rule(&|r| r["pinDigests"] == true)["matchCategories"][0],
        "docker"
    );
    let group = rule(&|r| minor_patch(r) && r.get("groupName").is_some());
    assert_eq!(group["automerge"], true);
    assert_eq!(
        rule(&|r| minor_patch(r) && r.get("minimumReleaseAge").is_some())["minimumReleaseAge"],
        "3 days"
    );
    let common = rule(&|r| {
        r["matchPackageNames"]
            .as_array()
            .is_some_and(|n| n.contains(&"postgres".into()))
    });
    assert_eq!(common["minimumReleaseAge"], "1 day");
    let digest = rule(&|r| types(r) == ["digest"]);
    assert_eq!(
        (digest["automerge"].clone(), digest["schedule"][0].clone()),
        (true.into(), "at any time".into())
    );
    let major = rule(&|r| types(r) == ["major"]);
    assert_eq!(major["automerge"], false);
    assert_eq!(major["labels"][0], "major-update");
    assert_eq!(config["prConcurrentLimit"], 3);
    assert_eq!(config["lockFileMaintenance"]["enabled"], true);
    assert_eq!(
        config["lockFileMaintenance"]["schedule"][0],
        "before 4am on monday"
    );
    assert_eq!(config["vulnerabilityAlerts"]["labels"][0], "security");
}

// @spec DEPLOY-CI-008
#[test]
fn the_branch_protection_script_states_the_agreed_settings() {
    for setting in [
        // A PR is required, with no approvals (the owner is the only reviewer).
        r#""required_approving_review_count": 0"#,
        // GitGuardian stays a required, up-to-date check.
        r#""context": "GitGuardian Security Checks", "app_id": 46505"#,
        r#""strict": true"#,
        // The owner keeps an emergency path; force-pushes and deleting main stay off.
        r#""enforce_admins": false"#,
        r#""allow_force_pushes": false"#,
        r#""allow_deletions": false"#,
        // Merged PR branches are deleted.
        r#""delete_branch_on_merge": true"#,
    ] {
        assert!(PROTECTION.contains(setting), "missing {setting}");
    }
    assert!(PROTECTION.contains("check)") && PROTECTION.contains("apply)"));
    assert!(PROTECTION.contains(r#"REPO="tgrecojr/turfops""#));
    assert!(PROTECTION.contains("repos/$REPO/branches/main/protection"));
    assert!(
        !PROTECTION.contains("GITHUB_TOKEN="),
        "no token in the script"
    );
}
