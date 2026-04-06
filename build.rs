//! Cargo build-script entrypoint for theorem file discovery and suite generation.

#[path = "src/build_discovery.rs"]
mod build_discovery;

#[path = "src/build_suite.rs"]
mod build_suite;

use camino::Utf8PathBuf;
use cap_std::fs_utf8::Dir as Utf8Dir;

/// Scans `CARGO_MANIFEST_DIR/theorems`, generates `OUT_DIR/theorem_suite.rs`,
/// and emits `cargo::rerun-if-changed` lines for discovered theorem files
/// and watched directories.
fn main() {
    let manifest_dir = Utf8PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|error| panic!("CARGO_MANIFEST_DIR is not set: {error}")),
    );
    let out_dir_path = Utf8PathBuf::from(
        std::env::var("OUT_DIR").unwrap_or_else(|error| panic!("OUT_DIR is not set: {error}")),
    );

    let discovery = build_discovery::discover_theorem_inputs(&manifest_dir)
        .unwrap_or_else(|error| panic!("failed to discover theorem build inputs: {error}"));

    // Generate OUT_DIR/theorem_suite.rs
    let out_dir = Utf8Dir::open_ambient_dir(&out_dir_path, cap_std::ambient_authority())
        .unwrap_or_else(|error| panic!("failed to open OUT_DIR: {error}"));
    build_suite::write_theorem_suite(&out_dir, &discovery)
        .unwrap_or_else(|error| panic!("failed to write theorem suite: {error}"));

    // Emit rerun-if-changed lines for Cargo
    for path in discovery.rerun_paths() {
        println!("cargo::rerun-if-changed={}", path.as_str());
    }
}
