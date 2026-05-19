//! Behavioural tests for real `theorem_file!` proc-macro expansion.

use camino::Utf8Path;
use rstest_bdd_macros::{given, scenario, then};

#[path = "theorem_file_macro_bdd/cargo_runner.rs"]
mod cargo_runner;
#[path = "theorem_file_macro_bdd/fixture_crate.rs"]
mod fixture_crate;

use cargo_runner::CargoGuard;
use fixture_crate::{
    FIXTURE_BUILD_DEPENDENCIES, FIXTURE_LIB_RS, FixtureCrate, TheoremFixtureSpec,
    fixture_cargo_toml_for, list_kani_harnesses, run_valid_fixture_build,
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

const MISSING_KANI_EVIDENCE_THEOREM: &str = concat!(
    "Theorem: MissingKaniMacro\n",
    "About: Missing Kani evidence macro coverage\n",
    "Witness:\n",
    "  - cover: \"true\"\n",
    "    because: \"reachable\"\n",
    "Prove:\n",
    "  - assert: \"true\"\n",
    "    because: \"trivial\"\n",
    "Evidence:\n",
    "  verus: \"future backend\"\n",
);

#[given("a fixture crate with one valid theorem file")]
fn given_a_fixture_crate_with_one_valid_theorem_file() {}

#[then("the fixture crate builds without a Kani dependency")]
fn then_the_fixture_crate_builds_without_a_kani_dependency() -> Result<(), String> {
    run_valid_fixture_build(&TheoremFixtureSpec {
        path: "theorems/single.theorem",
        content: VALID_SINGLE_THEOREM,
    })
}

#[then("Kani lists the generated proof harness")]
fn then_kani_lists_the_generated_proof_harness() -> Result<(), String> {
    if !cargo_runner::kani_is_installed() {
        return Ok(());
    }

    let output = list_kani_harnesses(&TheoremFixtureSpec {
        path: "theorems/single.theorem",
        content: VALID_SINGLE_THEOREM,
    })?;

    if !output.contains("theorem__smoke_macro__h") {
        return Err(format!(
            "expected Kani list output to include SmokeMacro harness, got:\n{output}"
        ));
    }

    Ok(())
}

#[given("a fixture crate with one valid multi-document theorem file")]
fn given_a_fixture_crate_with_one_valid_multi_document_theorem_file() {}

#[then("the fixture crate builds all generated theorem entries without a Kani dependency")]
fn then_the_fixture_crate_builds_all_generated_theorem_entries_without_a_kani_dependency()
-> Result<(), String> {
    run_valid_fixture_build(&TheoremFixtureSpec {
        path: "theorems/multi.theorem",
        content: VALID_MULTI_THEOREM,
    })
}

#[given("a fixture crate with one invalid theorem file")]
fn given_a_fixture_crate_with_one_invalid_theorem_file() {}

#[given("a fixture crate with one theorem file missing Kani evidence")]
fn given_a_fixture_crate_with_one_theorem_file_missing_kani_evidence() {}

#[then("compiling the fixture crate fails with an actionable theorem diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_an_actionable_theorem_diagnostic()
-> Result<(), String> {
    let guard = CargoGuard::acquire();
    let theorem_path = "theorems/invalid.theorem";
    let fixture = FixtureCrate::new(FIXTURE_LIB_RS)?;
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

#[then("compiling the fixture crate fails with a missing Kani evidence diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_a_missing_kani_evidence_diagnostic()
-> Result<(), String> {
    let guard = CargoGuard::acquire();
    let theorem_path = "theorems/missing-kani.theorem";
    let fixture = FixtureCrate::new(FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new(theorem_path), MISSING_KANI_EVIDENCE_THEOREM)?;
    let build_error = fixture
        .cargo_build(&guard)
        .err()
        .ok_or_else(|| "theorem without Kani evidence unexpectedly compiled".to_owned())?;

    if !build_error.contains("MissingKaniMacro") {
        return Err(format!(
            "expected build failure to mention MissingKaniMacro, got:\n{build_error}"
        ));
    }

    if !build_error.contains("does not declare required `Evidence.kani` configuration") {
        return Err(format!(
            "expected build failure to mention missing Kani evidence, got:\n{build_error}"
        ));
    }

    Ok(())
}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A valid theorem file compiles without Kani installed"
)]
fn a_valid_theorem_file_compiles_without_kani_installed() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A valid theorem file exposes a Kani proof harness"
)]
fn a_valid_theorem_file_exposes_a_kani_proof_harness() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A multi-document theorem file compiles without Kani installed"
)]
fn a_multi_document_theorem_file_compiles_without_kani_installed() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "An invalid theorem file fails compilation during macro expansion"
)]
fn an_invalid_theorem_file_fails_compilation_during_macro_expansion() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A theorem file without Kani evidence fails macro expansion"
)]
fn a_theorem_file_without_kani_evidence_fails_macro_expansion() {}

#[test]
fn fixture_cargo_lock_acquires_without_poisoning() {
    let guard = CargoGuard::acquire();
    drop(guard);
}

#[test]
fn fixture_cargo_toml_normalizes_windows_paths() {
    // Simulate a Windows-style `CARGO_MANIFEST_DIR` with backslash separators.
    // `fixture_cargo_toml` reads `ROOT_MANIFEST_DIR`, which is set at compile
    // time, but the normalization logic is what this test needs to verify.
    let windows_path = r"C:\Users\user\projects\theoremc";
    let toml = fixture_cargo_toml_for(windows_path);

    // Forward slashes must appear in the TOML; no backslashes.
    assert!(
        !toml.contains('\\'),
        "TOML must not contain backslashes after normalization; got:\n{toml}",
    );

    // The path must appear as a TOML basic string after normalization.
    assert!(
        toml.contains("\"C:/Users/user/projects/theoremc\""),
        "expected normalized forward-slash path in TOML; got:\n{toml}",
    );
    assert!(toml.contains(FIXTURE_BUILD_DEPENDENCIES));
}

#[test]
fn fixture_cargo_toml_escapes_basic_string_paths() {
    let checkout_path = "/home/user/project's/\"theoremc\"";
    let toml = fixture_cargo_toml_for(checkout_path);

    assert!(
        toml.contains("path = \"/home/user/project's/\\\"theoremc\\\"\""),
        "expected escaped TOML basic string path, got:\n{toml}",
    );
    assert!(toml.contains(FIXTURE_BUILD_DEPENDENCIES));
}
