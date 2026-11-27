//! trybuild compile-error snapshot tests for the `theorem_file!` proc-macro.
//!
//! trybuild resolves its project directory from `cargo metadata`'s
//! `target_directory`. This harness resolves the same value once, through the
//! same Cargo invocation, and passes it explicitly to the staging step, so the
//! fixtures land where the trybuild child Cargo looks and no test needs to read
//! `CARGO_TARGET_DIR` or mutate the process environment.

use camino::Utf8PathBuf;

#[path = "expand_support/staging.rs"]
mod staging;

use staging::{stage_fixtures, target_directory};

/// Fixture files staged under the trybuild project directory, in staging order.
///
/// The `.theorem` documents are copied; the `.rs` cases and `.stderr` snapshots
/// are read in place by the trybuild child. `missing_theorem` is deliberately
/// absent: it exists to prove a missing theorem document is reported.
const STAGED_FIXTURES: [&str; 11] = [
    "invalid_theorem.theorem",
    "missing_kani_evidence.theorem",
    "missing_action_export.theorem",
    "conflicting_action_signatures.theorem",
    "equivalent_action_signatures.theorem",
    "signature_drift.theorem",
    "missing_referenced_type.theorem",
    "moved_referenced_type.theorem",
    "typed_action_probe.theorem",
    "valid_theorem.theorem",
    "zero_unwind.theorem",
];

#[test]
fn compile_errors() -> Result<(), Box<dyn std::error::Error>> {
    let crate_root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = target_directory(&crate_root)?;
    stage_fixtures(&crate_root, &target, &STAGED_FIXTURES)?;

    let t = trybuild::TestCases::new();
    t.pass("tests/expand/typed_action_probe.rs");
    t.pass("tests/expand/valid_theorem.rs");
    t.pass("tests/expand/equivalent_action_signatures.rs");
    t.compile_fail("tests/expand/conflicting_action_signatures.rs");
    t.compile_fail("tests/expand/invalid_theorem.rs");
    t.compile_fail("tests/expand/missing_action_export.rs");
    t.compile_fail("tests/expand/missing_kani_evidence.rs");
    t.compile_fail("tests/expand/missing_referenced_type.rs");
    t.compile_fail("tests/expand/missing_theorem.rs");
    t.compile_fail("tests/expand/moved_referenced_type.rs");
    t.compile_fail("tests/expand/signature_drift.rs");
    t.compile_fail("tests/expand/zero_unwind.rs");
    Ok(())
}
