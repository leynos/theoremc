//! Contract test for the development build footprint rule.
//!
//! The Concordat `rust-dev-build-footprint` rule needs the default development
//! profile to disable incremental compilation and debug information, and the
//! Cargo configuration to disable metadata embedding through the native key.
//! This contract reads those settings from the files, starts no Cargo process,
//! and refuses the raw rustc metadata flags that broke build scripts.

/// Root manifest holding the development profile.
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
/// Cargo configuration holding the native metadata key.
const CARGO_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo/config.toml"));

/// Returns the non-comment lines of the named top-level TOML table.
fn table_lines<'a>(document: &'a str, table: &str) -> Vec<&'a str> {
    let header = format!("[{table}]");
    document
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter(|line| !line.is_empty())
        .collect()
}

/// Returns the value assigned to `key` in a table's lines.
fn value<'a>(lines: &[&'a str], key: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        let (name, assigned) = line.split_once('=')?;
        (name.trim() == key).then_some(assigned.trim())
    })
}

/// Scenario: the default development profile.
///
/// Invariant: it keeps no incremental state and no debug information.
#[test]
fn the_development_profile_keeps_the_build_footprint_settings() {
    let profile = table_lines(MANIFEST, "profile.dev");
    assert_eq!(
        value(&profile, "incremental"),
        Some("false"),
        "profile.dev must set incremental = false"
    );
    assert!(
        matches!(value(&profile, "debug"), Some("0" | "false")),
        "profile.dev must set debug = 0 or false, found {:?}",
        value(&profile, "debug")
    );
}

/// Scenario: the Cargo configuration.
///
/// Invariant: it sets the native `embed-metadata = false` key, not the older
/// spelling.
#[test]
fn the_cargo_configuration_disables_metadata_embedding_natively() {
    let unstable = table_lines(CARGO_CONFIG, "unstable");
    assert_eq!(
        value(&unstable, "embed-metadata"),
        Some("false"),
        "[unstable] must set embed-metadata = false"
    );
    assert_eq!(
        value(&unstable, "no-embed-metadata"),
        None,
        "the older no-embed-metadata spelling is wrong on the current pin"
    );
}

/// Scenario: every `rustflags` source in the Cargo configuration.
///
/// Invariant: none carries a raw rustc metadata flag.
#[test]
fn no_rustflags_source_carries_a_raw_metadata_flag() {
    let flags: Vec<&str> = CARGO_CONFIG
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter(|line| line.starts_with("rustflags"))
        .collect();
    assert!(!flags.is_empty(), "expected at least one rustflags source");
    assert!(
        flags.iter().all(|line| !line.contains("embed-metadata")),
        "no raw embed-metadata flag may appear in rustflags: {flags:?}"
    );
}
