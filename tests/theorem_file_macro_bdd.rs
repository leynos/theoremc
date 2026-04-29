//! Behavioural tests for real `theorem_file!` proc-macro expansion.

use std::sync::PoisonError;

use camino::Utf8Path;
use rstest_bdd_macros::{given, scenario, then};

#[path = "theorem_file_macro_bdd/cargo_runner.rs"]
mod cargo_runner;
#[path = "theorem_file_macro_bdd/fixture_crate.rs"]
mod fixture_crate;

use cargo_runner::{CargoGuard, FIXTURE_CARGO_LOCK};
use fixture_crate::{
    FIXTURE_BUILD_DEPENDENCIES, FixtureCrate, TheoremFixtureSpec, fixture_cargo_toml_for,
    invalid_fixture_lib_rs, run_valid_fixture_test,
};

const VALID_SINGLE_THEOREM: &str = concat!(
    "Theorem: SmokeMacro\n",
    "About: Single theorem macro coverage\n",
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

const VALID_MULTI_THEOREM: &str = concat!(
    "Theorem: FirstMacroDoc\n",
    "About: First theorem macro coverage\n",
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
    "---\n",
    "Theorem: SecondMacroDoc\n",
    "About: Second theorem macro coverage\n",
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

const INVALID_THEOREM: &str = concat!(
    "Theorem: BrokenMacro\n",
    "About: \"\"\n",
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

#[given("a fixture crate with one valid theorem file")]
fn given_a_fixture_crate_with_one_valid_theorem_file() {}

#[then("the fixture crate tests can refer to the generated private symbols")]
fn then_the_fixture_crate_tests_can_refer_to_the_generated_private_symbols() -> Result<(), String> {
    run_valid_fixture_test(&TheoremFixtureSpec {
        path: "theorems/single.theorem",
        names: &["SmokeMacro"],
        content: VALID_SINGLE_THEOREM,
    })
}

#[given("a fixture crate with one valid multi-document theorem file")]
fn given_a_fixture_crate_with_one_valid_multi_document_theorem_file() {}

#[then("the fixture crate tests can refer to all generated harness stubs")]
fn then_the_fixture_crate_tests_can_refer_to_all_generated_harness_stubs() -> Result<(), String> {
    run_valid_fixture_test(&TheoremFixtureSpec {
        path: "theorems/multi.theorem",
        names: &["FirstMacroDoc", "SecondMacroDoc"],
        content: VALID_MULTI_THEOREM,
    })
}

#[given("a fixture crate with one invalid theorem file")]
fn given_a_fixture_crate_with_one_invalid_theorem_file() {}

#[then("compiling the fixture crate fails with an actionable theorem diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_an_actionable_theorem_diagnostic()
-> Result<(), String> {
    let guard = CargoGuard::acquire();
    let theorem_path = "theorems/invalid.theorem";
    let fixture = FixtureCrate::new(invalid_fixture_lib_rs())?;
    fixture.write(Utf8Path::new(theorem_path), INVALID_THEOREM)?;
    let build_error = fixture
        .cargo_build(&guard)
        .err()
        .ok_or_else(|| "invalid theorem fixture unexpectedly compiled".to_owned())?;

    if !build_error.contains(theorem_path) {
        return Err(format!(
            "expected build failure to mention '{theorem_path}', got:\n{build_error}"
        ));
    }

    if !build_error.contains("About must be non-empty") {
        return Err(format!(
            "expected build failure to mention 'About must be non-empty', got:\n{build_error}"
        ));
    }

    Ok(())
}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A valid theorem file produces the expected generated symbol paths"
)]
fn a_valid_theorem_file_produces_the_expected_generated_symbol_paths() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A multi-document theorem file generates one harness stub per document"
)]
fn a_multi_document_theorem_file_generates_one_harness_stub_per_document() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "An invalid theorem file fails compilation during macro expansion"
)]
fn an_invalid_theorem_file_fails_compilation_during_macro_expansion() {}

#[test]
fn fixture_cargo_lock_recovers_from_poison() {
    // Poison the mutex by panicking while holding the guard.
    let result = std::panic::catch_unwind(|| {
        let _guard = FIXTURE_CARGO_LOCK
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        panic!("deliberate panic to poison the mutex");
    });
    assert!(result.is_err(), "catch_unwind should have caught the panic");

    // After poisoning, `CargoGuard::acquire` must still succeed.
    let guard = CargoGuard::acquire();
    // The guard is usable: this line compiles and runs without panic.
    drop(guard);
}

#[test]
fn fixture_cargo_toml_normalises_windows_paths() {
    // Simulate a Windows-style `CARGO_MANIFEST_DIR` with backslash separators.
    // `fixture_cargo_toml` reads `ROOT_MANIFEST_DIR`, which is set at compile
    // time, but the normalization logic is what this test needs to verify.
    let windows_path = r"C:\Users\user\projects\theoremc";
    let normalised = windows_path.replace('\\', "/");
    let toml = fixture_cargo_toml_for(&normalised);

    // Forward slashes must appear in the TOML; no backslashes.
    assert!(
        !toml.contains('\\'),
        "TOML must not contain backslashes after normalization; got:\n{toml}",
    );

    // The path must appear literally; single-quoted TOML strings are not
    // interpreted.
    assert!(
        toml.contains("'C:/Users/user/projects/theoremc'"),
        "expected normalized forward-slash path in TOML; got:\n{toml}",
    );
    assert!(toml.contains(FIXTURE_BUILD_DEPENDENCIES));
}
