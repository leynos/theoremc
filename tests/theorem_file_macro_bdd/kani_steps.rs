//! Kani-dependent BDD steps for `theorem_file_macro_bdd`.
//!
//! These steps run a real `cargo kani` build, so they carry the skip policy for
//! a host without a usable verifier: an unavailable Kani skips, a hung probe or
//! an overrun fails. They live apart from the compile-only steps because their
//! failure modes are theirs alone.

use rstest_bdd_macros::then;
use theoremc::mangle::mangle_theorem_harness;

use super::VALID_SINGLE_THEOREM;
use super::cargo_runner::{CargoRunFailure, KaniAvailability, kani_is_available, kani_is_required};
use super::fixture_crate::{
    TheoremFixtureSpec, build_fixture_and_list_kani_harnesses,
    build_fixture_and_verify_kani_harnesses,
};

/// Two theorems in one file with different unwind bounds, so the verified crate
/// carries a multi-harness module and two distinct `unwind` literals.
pub(super) const KANI_VERIFY_THEOREM: &str = concat!(
    "Theorem: VerifyFirst\n",
    "About: First theorem verified by cargo kani\n",
    "Witness:\n",
    "  - cover: \"true\"\n",
    "    because: \"reachable\"\n",
    "Prove:\n",
    "  - assert: \"true\"\n",
    "    because: \"trivial\"\n",
    "Evidence:\n",
    "  kani:\n",
    "    unwind: 3\n",
    "    expect: SUCCESS\n",
    "---\n",
    "Theorem: VerifySecond\n",
    "About: Second theorem verified by cargo kani\n",
    "Witness:\n",
    "  - cover: \"true\"\n",
    "    because: \"reachable\"\n",
    "Prove:\n",
    "  - assert: \"true\"\n",
    "    because: \"trivial\"\n",
    "Evidence:\n",
    "  kani:\n",
    "    unwind: 1\n",
    "    expect: SUCCESS\n",
);

#[then("cargo-kani lists the generated proof harness when installed")]
pub(super) fn then_kani_lists_the_generated_proof_harness() -> Result<(), String> {
    if let Some(reason) = kani_skip_reason("listing")? {
        return skip_without_kani(&reason, "listing");
    }

    let output = match build_fixture_and_list_kani_harnesses(&TheoremFixtureSpec {
        path: "theorems/single.theorem",
        content: VALID_SINGLE_THEOREM,
    }) {
        Ok(output) => output,
        // Only a completed, failing command can mean the environment is
        // unusable. A timeout fails the test whatever its partial log says.
        Err(CargoRunFailure::Failed(error)) if is_unusable_kani_environment(&error) => {
            return skip_without_kani(
                &format!("cargo-kani is installed but unusable: {error}"),
                "listing",
            );
        }
        Err(error) => return Err(error.to_string()),
    };
    let expected_mangled_harness = mangle_theorem_harness("theorems/single.theorem", "SmokeMacro");
    let expected_harness_identifier = expected_mangled_harness.identifier();

    if !is_expected_single_harness_listing(&output, expected_harness_identifier) {
        return Err(format!(
            "expected Kani list output to contain exactly the SmokeMacro harness, got:\n{output}"
        ));
    }

    Ok(())
}

#[then("cargo-kani verifies every generated proof harness")]
pub(super) fn then_kani_verifies_every_generated_proof_harness() -> Result<(), String> {
    if let Some(reason) = kani_skip_reason("verification")? {
        return skip_without_kani(&reason, "verification");
    }

    let output = match build_fixture_and_verify_kani_harnesses(&TheoremFixtureSpec {
        path: "theorems/verify.theorem",
        content: KANI_VERIFY_THEOREM,
    }) {
        Ok(output) => output,
        Err(CargoRunFailure::Failed(error)) if is_unusable_kani_environment(&error) => {
            return skip_without_kani(
                &format!("cargo-kani is installed but unusable: {error}"),
                "verification",
            );
        }
        Err(error) => return Err(error.to_string()),
    };

    // Two theorem documents, so two harnesses, each of which must verify. The
    // summary line is Kani's own; a rejected harness reports a failure count.
    // Kani also names each harness it checks, so the two generated identifiers
    // must appear, not just a count.
    let expected_harnesses = [
        mangle_theorem_harness("theorems/verify.theorem", "VerifyFirst"),
        mangle_theorem_harness("theorems/verify.theorem", "VerifySecond"),
    ];
    let summary = "Complete - 2 successfully verified harnesses, 0 failures, 2 total.";
    let names_both = expected_harnesses.iter().all(|harness| {
        output.lines().any(|line| {
            line.starts_with("Checking harness ") && line.contains(harness.identifier())
        })
    });
    if !output.contains(summary) || !names_both {
        return Err(format!(
            "expected Kani to verify both generated harnesses, got:\n{output}"
        ));
    }

    Ok(())
}

/// Returns the reason to skip a Kani check, or `None` when Kani is usable.
///
/// # Errors
///
/// Returns an error when the probe could not be run at all, and when it overran
/// its deadline. A probe that hangs is a defect in the environment this run must
/// name, not a reason to pass without checking anything.
fn kani_skip_reason(check: &str) -> Result<Option<String>, String> {
    match kani_is_available()? {
        KaniAvailability::Available => Ok(None),
        KaniAvailability::Unavailable(reason) => {
            Ok(Some(format!("cargo-kani is not usable: {reason}")))
        }
        KaniAvailability::TimedOut(deadline) => Err(format!(
            "the cargo-kani availability probe for the Kani {check} check did not finish \
             within {deadline:?}; a hung probe is a failure, not a skip"
        )),
        KaniAvailability::LockBusy(deadline) => Err(format!(
            "the cargo-kani availability probe for the Kani {check} check could not take \
             the fixture lock within {deadline:?}; another fixture command held it, so \
             Kani's availability is unknown"
        )),
    }
}

/// Skips a Kani check, or fails it when `THEOREMC_REQUIRE_KANI=1`.
///
/// A skip hides a missing verifier, so the proofs job sets the variable and
/// turns every would-be skip into a failure.
#[expect(
    clippy::print_stderr,
    reason = "BDD skip diagnostics should remain non-fallible"
)]
fn skip_without_kani(reason: &str, check: &str) -> Result<(), String> {
    if kani_is_required() {
        return Err(format!(
            "THEOREMC_REQUIRE_KANI=1 forbids skipping the Kani {check} check: {reason}"
        ));
    }
    eprintln!("{reason}; skipping Kani harness {check} check");
    Ok(())
}

/// Recognizes a Kani build failure that says the environment is unusable.
fn is_unusable_kani_environment(error: &str) -> bool {
    error.contains("error while loading shared libraries")
        || error.contains("cannot open shared object file")
        || error.contains("Broken pipe")
        || error.contains("unexpected output from `rustc --version`: \"\"")
}

/// Checks that the listing names exactly the expected harness.
fn is_expected_single_harness_listing(output: &str, expected_harness_identifier: &str) -> bool {
    output.contains("Standard Harnesses (#[kani::proof]):")
        && output.contains("No contracts or contract harnesses found.")
        && output.contains(expected_harness_identifier)
        && output.matches("theorem__").count() == 1
        && output.contains("| Total |")
        && output.contains("| 1")
}
