//! `xtask verify-release` — Validation-Patch G2 (Release-Hardening-Gate).
//!
//! Runs as the first step of `.github/workflows/release.yml`. If any check
//! fails, the release build does not start.
//!
//! Spec: `output/planning-artifacts/architecture.md` §4a Release-Hardening.
//!
//! Implemented checks:
//!   1. Forbidden Cargo features are not active in the workspace resolution:
//!        - `test-license`
//!        - `dev-*` (prefix)
//!      Rationale: v1 shipped test-licenses into prod; preventive gate scoped
//!      to active feature resolution (not declaration) so future `dev-*`
//!      feature introductions fail the release build unless explicitly
//!      disabled. Spec §4a + memory `project_api_key_os_keystore_mvp.md`.
//!
//!   2. Sentinel: `tracing-subscriber` must NOT be a resolved dependency yet.
//!      Rationale: the real check is "DEBUG/TRACE subscribers are not
//!      attached in release builds" (PII protection for Debug-Export-Zip —
//!      memory `project_no_remote_telemetry.md`). That check requires a
//!      subscriber to exist. Until it does, this sentinel fires so the
//!      Phase-1 session that wires up `tracing-subscriber` is forced to
//!      also implement the real check here and delete the sentinel.
//!
//! Deferred (TODO — present as comments in this file, NOT as silent stubs):
//!   - `obfstr`-key default-placeholder check. Prerequisite: `obfstr` crate
//!     in workspace + compile-time-constant Licensing-HMAC-Key. See §4 +
//!     §4a. Phase-4 licensing rollout.
//!   - `#[cfg(debug_assertions)]` release-code-path scan. Skipped by design:
//!     rustc already strips debug-only code under `--release`. An AST-based
//!     scan (see `lint_events.rs`) would only add value if a concrete
//!     divergence is observed. §4a calls this "redundant with Rust default".

use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
}

#[derive(Deserialize, Debug)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize, Debug)]
struct Node {
    id: String,
    features: Vec<String>,
}

pub fn run() -> ExitCode {
    let metadata = match load_metadata() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("xtask verify-release: {e}");
            return ExitCode::from(2);
        }
    };

    let mut failures: Vec<String> = Vec::new();
    failures.extend(check_forbidden_features(&metadata));
    if let Err(msg) = check_tracing_subscriber_sentinel(&metadata) {
        failures.push(msg);
    }

    // TODO(phase-4): obfstr key != default-placeholder (compile-time-constant).
    //   Spec: architecture.md §4 (HMAC) + §4a. Memory:
    //   reference_adr_directory.md. Activate when `obfstr` enters the
    //   workspace and the licensing key constant has a known symbol path.
    // TODO(skip-by-design): `#[cfg(debug_assertions)]` release-code-path scan.
    //   Rustc strips debug-only code in `--release`. If a concrete divergence
    //   is ever observed, implement a `syn`-based AST scan modeled after
    //   `lint_events.rs` (distinguish `cfg(debug_assertions)` from
    //   `cfg(not(debug_assertions))`).

    if failures.is_empty() {
        println!(
            "xtask verify-release: OK ({} resolved packages, {} resolve-nodes checked)",
            metadata.packages.len(),
            metadata.resolve.nodes.len()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "xtask verify-release: FAIL ({} violation(s))",
            failures.len()
        );
        for f in &failures {
            eprintln!("  - {f}");
        }
        ExitCode::from(1)
    }
}

fn locate_workspace_root() -> Option<PathBuf> {
    // xtask/Cargo.toml → workspace root is the parent of CARGO_MANIFEST_DIR.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map(Path::to_path_buf)
}

fn load_metadata() -> Result<Metadata, String> {
    let root = locate_workspace_root().ok_or("could not locate workspace root")?;
    let manifest = root.join("Cargo.toml");
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output()
        .map_err(|e| format!("failed to spawn `cargo metadata`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` exited with {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("could not parse `cargo metadata` output: {e}"))
}

fn check_forbidden_features(metadata: &Metadata) -> Vec<String> {
    let mut out = Vec::new();
    for node in &metadata.resolve.nodes {
        for feat in &node.features {
            if is_forbidden_feature(feat) {
                out.push(format!(
                    "package `{}` has forbidden feature `{}` active in release resolution (spec §4a)",
                    package_name_from_id(&node.id),
                    feat
                ));
            }
        }
    }
    out
}

fn is_forbidden_feature(name: &str) -> bool {
    name == "test-license" || name.starts_with("dev-")
}

fn package_name_from_id(id: &str) -> &str {
    // cargo metadata package-id format: "<name> <version> (<source>)" or
    // the new PackageIdSpec "<scheme>+<url>#<name>@<version>". We only need
    // a human-readable name for error messages — take the first whitespace
    // token and, if it contains a '#', keep what follows it up to '@'.
    let head = id.split_whitespace().next().unwrap_or(id);
    if let Some((_, after_hash)) = head.split_once('#') {
        after_hash.split('@').next().unwrap_or(after_hash)
    } else {
        head
    }
}

fn check_tracing_subscriber_sentinel(metadata: &Metadata) -> Result<(), String> {
    let present = metadata
        .packages
        .iter()
        .any(|p| p.name == "tracing-subscriber");
    if present {
        Err(
            "`tracing-subscriber` is now a resolved dependency. Implement the \
             real DEBUG/TRACE release-filter check in \
             xtask/src/verify_release.rs::check_tracing_subscriber_sentinel \
             and delete this sentinel. Spec: architecture.md §4 Telemetrie, \
             §4a. Memory: project_no_remote_telemetry.md."
                .into(),
        )
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, feats: &[&str]) -> Node {
        Node {
            id: id.into(),
            features: feats.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    fn fixture(pkgs: &[&str], nodes: Vec<Node>) -> Metadata {
        Metadata {
            packages: pkgs
                .iter()
                .map(|n| Package {
                    name: (*n).to_string(),
                })
                .collect(),
            resolve: Resolve { nodes },
        }
    }

    #[test]
    fn is_forbidden_feature_matches_spec() {
        assert!(is_forbidden_feature("test-license"));
        assert!(is_forbidden_feature("dev-plain-keystore"));
        assert!(is_forbidden_feature("dev-anything"));
        assert!(is_forbidden_feature("dev-"));
        assert!(!is_forbidden_feature("default"));
        assert!(!is_forbidden_feature("gpu-cuda"));
        assert!(!is_forbidden_feature("develop")); // no false-prefix match
        assert!(!is_forbidden_feature("test-licensed")); // exact match only
        assert!(!is_forbidden_feature(""));
    }

    #[test]
    fn package_name_from_id_legacy_format() {
        assert_eq!(
            package_name_from_id("klarvo-core 0.0.1 (path+file:///tmp/klarvo)"),
            "klarvo-core"
        );
    }

    #[test]
    fn package_name_from_id_spec_format() {
        assert_eq!(
            package_name_from_id("path+file:///tmp/klarvo#klarvo-core@0.0.1"),
            "klarvo-core"
        );
    }

    #[test]
    fn forbidden_features_empty_resolution_passes() {
        let m = fixture(&[], vec![node("klarvo-core 0.0.1", &["default"])]);
        assert!(check_forbidden_features(&m).is_empty());
    }

    #[test]
    fn forbidden_features_flags_test_license() {
        let m = fixture(
            &[],
            vec![node("klarvo-core 0.0.1", &["default", "test-license"])],
        );
        let v = check_forbidden_features(&m);
        assert_eq!(v.len(), 1, "expected exactly one violation: {v:?}");
        assert!(v[0].contains("test-license"));
        assert!(v[0].contains("klarvo-core"));
    }

    #[test]
    fn forbidden_features_flags_dev_prefix_in_multiple_packages() {
        let m = fixture(
            &[],
            vec![
                node("klarvo-core 0.0.1", &["dev-plain-keystore"]),
                node("klarvo-plugin-x 0.0.1", &["dev-mock-upstream"]),
            ],
        );
        let v = check_forbidden_features(&m);
        assert_eq!(v.len(), 2);
        assert!(v.iter().any(|s| s.contains("dev-plain-keystore")));
        assert!(v.iter().any(|s| s.contains("dev-mock-upstream")));
    }

    #[test]
    fn forbidden_features_passes_ordinary_features() {
        let m = fixture(
            &[],
            vec![node(
                "klarvo-core 0.0.1",
                &["default", "gpu-cuda", "windows-only"],
            )],
        );
        assert!(check_forbidden_features(&m).is_empty());
    }

    #[test]
    fn tracing_subscriber_sentinel_absent_passes() {
        let m = fixture(&["tracing", "serde"], vec![]);
        assert!(check_tracing_subscriber_sentinel(&m).is_ok());
    }

    #[test]
    fn tracing_subscriber_sentinel_present_fails_with_guidance() {
        let m = fixture(&["tracing", "tracing-subscriber", "serde"], vec![]);
        let err = check_tracing_subscriber_sentinel(&m).unwrap_err();
        assert!(err.contains("tracing-subscriber"));
        assert!(err.contains("verify_release"));
    }

    #[test]
    fn deserializes_minimal_cargo_metadata_json() {
        let raw = r#"{
            "packages": [{"name": "klarvo-core"}, {"name": "serde"}],
            "resolve": {
                "nodes": [
                    {"id": "klarvo-core 0.0.1 (path+file:///x)", "features": ["default", "test-license"]}
                ]
            },
            "workspace_root": "/tmp",
            "target_directory": "/tmp/target"
        }"#;
        let m: Metadata = serde_json::from_str(raw).expect("parse");
        assert_eq!(m.packages.len(), 2);
        assert_eq!(m.resolve.nodes.len(), 1);
        let v = check_forbidden_features(&m);
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("test-license"));
    }
}
