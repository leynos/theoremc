//! Compile-pass fixture for valid `theorem_file!` expansion.

use theoremc_macros::theorem_file;

theorem_file!("tests/expand/valid_theorem.theorem");

/// Assert valid theorem files compile during ordinary Rust builds.
///
/// Kani harness symbols are gated behind `cfg(kani)`, so this fixture proves
/// that non-Kani builds do not need the Kani crate or Kani attributes.
fn _assert_structure() {
}

fn main() {
    _assert_structure();
}
