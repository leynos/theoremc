//! Cargo build-script entrypoint for theorem file discovery.

#[path = "src/build_discovery.rs"]
mod build_discovery;

use camino::Utf8PathBuf;

fn main() {
    let manifest_dir = Utf8PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|error| panic!("CARGO_MANIFEST_DIR is not set: {error}")),
    );
    let discovery = build_discovery::discover_theorem_inputs(&manifest_dir)
        .unwrap_or_else(|error| panic!("failed to discover theorem build inputs: {error}"));

    for path in discovery.rerun_paths() {
        println!("cargo::rerun-if-changed={}", path.as_str());
    }
}
