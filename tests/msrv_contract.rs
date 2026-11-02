//! Contract tests for the minimum supported Rust version (MSRV) gate.
//!
//! ADR-010 declares Rust 1.89 for every workspace package and widens the gate
//! to every target. These tests read the wiring as text: the exact `make msrv`
//! recipe, the declared floor in each manifest, the MSRV-aware resolver, the
//! locked `cargo-platform` release, and the CI step that runs the gate. The
//! gate itself runs in CI; these tests prove the repository still invokes it
//! as the adopted baseline requires.

const MAKEFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Makefile"));
const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
const LOCK: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock"));

/// The workspace members, each of which must declare the same floor.
const MEMBER_MANIFESTS: &[(&str, &str)] = &[
    (
        "crates/test-helpers",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/crates/test-helpers/Cargo.toml"
        )),
    ),
    (
        "crates/theoremc-build-support",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/crates/theoremc-build-support/Cargo.toml"
        )),
    ),
    (
        "crates/cargo-theorem",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/crates/cargo-theorem/Cargo.toml"
        )),
    ),
    (
        "crates/theoremc-core",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/crates/theoremc-core/Cargo.toml"
        )),
    ),
    (
        "crates/theoremc-macros",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/crates/theoremc-macros/Cargo.toml"
        )),
    ),
];

/// The recipe the `msrv` target must run, exactly as ADR-010 records it.
///
/// `RUSTFLAGS=""` displaces the development rustflags, which the stable Rust
/// 1.89 toolchain cannot accept. `--all-targets` and `--all-features` bring the
/// dev-only dependency graph inside the contract, and `--locked` fails the gate
/// if the checked graph would need to change.
const PINNED_RECIPE: &str = concat!(
    "RUSTFLAGS=\"\" $(CARGO) +1.89 check --workspace ",
    "--all-targets --all-features --locked",
);

/// Returns the recipe lines of a Makefile target, without the target line.
///
/// The leading tab is stripped so a caller compares the command text alone.
/// An absent target yields an empty list, which fails the equality assertion
/// rather than panicking.
fn recipe_lines<'a>(makefile: &'a str, target: &str) -> Vec<&'a str> {
    let header = format!("{target}:");
    let mut lines = makefile.lines();
    if !lines.any(|line| line.starts_with(&header)) {
        return Vec::new();
    }
    lines
        .take_while(|line| line.starts_with('\t'))
        .map(|line| line.trim_start_matches('\t'))
        .collect()
}

/// Returns every version of the named package in a `Cargo.lock` document.
///
/// A lockfile lists one `[[package]]` table per resolved package, and a package
/// can appear more than once when two dependency lines need different major
/// versions. Each matching table contributes its `version` key, so the caller
/// can assert on the whole set rather than on whichever table comes first.
fn locked_versions<'a>(lock: &'a str, package: &str) -> Vec<&'a str> {
    let name = format!("name = \"{package}\"");
    lock.split("[[package]]")
        .skip(1)
        .filter(|table| table.lines().any(|line| line.trim() == name))
        .filter_map(|table| {
            table
                .lines()
                .find_map(|line| line.trim().strip_prefix("version = \""))
                .and_then(|rest| rest.split('"').next())
        })
        .collect()
}

#[test]
fn msrv_target_runs_the_locked_rust_1_89_check() {
    assert_eq!(
        recipe_lines(MAKEFILE, "msrv"),
        vec![PINNED_RECIPE],
        "the msrv target must run the locked Rust 1.89 check over every target"
    );
}

#[test]
fn every_manifest_declares_the_same_floor() {
    assert!(
        MANIFEST.contains("rust-version = \"1.89\""),
        "the root package must declare Rust 1.89"
    );
    for (path, manifest) in MEMBER_MANIFESTS {
        assert!(
            manifest.contains("rust-version = \"1.89\""),
            "{path} must declare Rust 1.89"
        );
    }
}

#[test]
fn the_workspace_uses_the_msrv_aware_resolver() {
    assert!(
        MANIFEST.contains("resolver = \"3\""),
        "the workspace must use resolver 3 so Cargo prefers MSRV-compatible releases"
    );
}

#[test]
fn the_lockfile_selects_the_msrv_compatible_cargo_platform() {
    let versions = locked_versions(LOCK, "cargo-platform");
    assert!(
        versions.contains(&"0.3.2"),
        "the lockfile must keep the dev-only cargo-platform on 0.3.2, the newest release that builds on Rust 1.89; found {versions:?}"
    );
    assert!(
        !versions.contains(&"0.3.3"),
        "cargo-platform 0.3.3 requires Rust 1.91 and must not be locked; found {versions:?}"
    );
}

#[test]
fn ci_runs_the_msrv_gate() {
    assert!(CI.contains("run: make msrv"), "CI must run the MSRV gate");
}
