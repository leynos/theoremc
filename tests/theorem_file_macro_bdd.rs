//! Behavioural tests for real `theorem_file!` proc-macro expansion.

use camino::Utf8Path;
use rstest_bdd_macros::{given, scenario, then};
use theoremc::mangle::mangle_theorem_harness;

/// Cargo process helpers used by the fixture crate module and BDD steps.
#[path = "theorem_file_macro_bdd/cargo_runner.rs"]
mod cargo_runner;
/// Temporary crate builder that lets these BDD scenarios compile real macros.
#[path = "theorem_file_macro_bdd/fixture_crate.rs"]
mod fixture_crate;

use cargo_runner::CargoGuard;
use fixture_crate::{
    FIXTURE_BUILD_DEPENDENCIES, FIXTURE_LIB_RS, FixtureCrate, TheoremFixtureSpec,
    build_fixture_and_list_kani_harnesses, fixture_cargo_toml_for, run_valid_fixture_build,
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

const PARTIAL_KANI_EVIDENCE_THEOREM: &str = concat!(
    "Theorem: CompleteKaniMacro\n",
    "About: Complete Kani evidence macro coverage\n",
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
    "Theorem: PartialKaniMacro\n",
    "About: Partial Kani evidence macro coverage\n",
    "Witness:\n",
    "  - cover: \"true\"\n",
    "    because: \"reachable\"\n",
    "Prove:\n",
    "  - assert: \"true\"\n",
    "    because: \"trivial\"\n",
    "Evidence:\n",
    "  verus: \"future backend\"\n",
);

const REFERENCED_TYPES_THEOREM: &str = concat!(
    "Theorem: ReferencedTypeSmoke\n",
    "About: Referenced type probe coverage\n",
    "Forall:\n",
    "  account: crate::Account\n",
    "Actions:\n",
    "  account.deposit:\n",
    "    params:\n",
    "      command: crate::DepositCommand\n",
    "    returns: crate::DepositOutcome\n",
    "Do:\n",
    "  - call:\n",
    "      action: account.deposit\n",
    "      args:\n",
    "        command:\n",
    "          amount: 10\n",
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

const MISSING_FORALL_TYPE_THEOREM: &str = concat!(
    "Theorem: MissingForallType\n",
    "About: Missing Forall type probe coverage\n",
    "Forall:\n",
    "  account: crate::MissingAccount\n",
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

const MOVED_ACTION_TYPE_THEOREM: &str = concat!(
    "Theorem: MovedActionType\n",
    "About: Moved Actions type probe coverage\n",
    "Actions:\n",
    "  account.deposit:\n",
    "    params:\n",
    "      command: crate::old::DepositCommand\n",
    "    returns: crate::DepositOutcome\n",
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

const REFERENCED_TYPES_LIB_RS: &str = concat!(
    "//! Fixture crate for referenced-type behavioural tests.\n\n",
    "pub struct Account;\n",
    "pub struct DepositCommand {\n",
    "    pub amount: u64,\n",
    "}\n",
    "pub struct DepositOutcome;\n\n",
    "pub mod theorem_actions {\n",
    "    #[allow(non_snake_case)]\n",
    "    pub fn account__deposit__h05158894bfb4(\n",
    "        _command: crate::DepositCommand,\n",
    "    ) -> crate::DepositOutcome {\n",
    "        crate::DepositOutcome\n",
    "    }\n",
    "}\n\n",
    "#[doc(hidden)]\n",
    "mod __theoremc_generated_suite {\n",
    "    #[cfg(theoremc_has_theorems)]\n",
    "    use theoremc::theorem_file;\n",
    "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
    "}\n",
);

struct ExpectedBuildFailure<'a> {
    lib_rs: &'a str,
    theorem_path: &'a str,
    theorem_content: &'a str,
    unexpected_success_msg: &'a str,
    expected_fragments: &'a [&'a str],
}

const EMPTY_TYPES_LIB_RS: &str = concat!(
    "//! Fixture crate with no theorem-owned type definitions.\n\n",
    "#[doc(hidden)]\n",
    "mod __theoremc_generated_suite {\n",
    "    #[cfg(theoremc_has_theorems)]\n",
    "    use theoremc::theorem_file;\n",
    "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
    "}\n",
);

const MOVED_ACTION_TYPE_LIB_RS: &str = concat!(
    "//! Fixture crate where an action type moved modules.\n\n",
    "pub mod new {\n",
    "    pub struct DepositCommand;\n",
    "}\n",
    "pub struct DepositOutcome;\n\n",
    "#[doc(hidden)]\n",
    "mod __theoremc_generated_suite {\n",
    "    #[cfg(theoremc_has_theorems)]\n",
    "    use theoremc::theorem_file;\n",
    "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
    "}\n",
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

#[then("cargo-kani lists the generated proof harness when installed")]
#[expect(
    clippy::print_stderr,
    reason = "BDD skip diagnostics should remain non-fallible"
)]
fn then_kani_lists_the_generated_proof_harness() -> Result<(), String> {
    if !cargo_runner::kani_is_installed() {
        eprintln!("cargo-kani not installed; skipping Kani harness listing check");
        return Ok(());
    }

    let output = match build_fixture_and_list_kani_harnesses(&TheoremFixtureSpec {
        path: "theorems/single.theorem",
        content: VALID_SINGLE_THEOREM,
    }) {
        Ok(output) => output,
        Err(error) if is_unusable_kani_environment(&error) => {
            eprintln!("cargo-kani is installed but unusable; skipping Kani harness listing check");
            return Ok(());
        }
        Err(error) => return Err(error),
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

fn is_unusable_kani_environment(error: &str) -> bool {
    error.contains("error while loading shared libraries")
        || error.contains("cannot open shared object file")
        || error.contains("Broken pipe")
}

fn is_expected_single_harness_listing(output: &str, expected_harness_identifier: &str) -> bool {
    output.contains("Standard Harnesses (#[kani::proof]):")
        && output.contains("No contracts or contract harnesses found.")
        && output.contains(expected_harness_identifier)
        && output.matches("theorem__").count() == 1
        && output.contains("| Total |")
        && output.contains("| 1")
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

#[given("a fixture crate with a multi-document theorem file missing one Kani evidence block")]
fn given_a_fixture_crate_with_a_multi_document_theorem_file_missing_one_kani_evidence_block() {}

#[given("a fixture crate with declared theorem types")]
fn given_a_fixture_crate_with_declared_theorem_types() {}

#[then("the fixture crate builds referenced type probes without a Kani dependency")]
fn then_the_fixture_crate_builds_referenced_type_probes_without_a_kani_dependency()
-> Result<(), String> {
    run_fixture_build(
        REFERENCED_TYPES_LIB_RS,
        &TheoremFixtureSpec {
            path: "theorems/referenced-types.theorem",
            content: REFERENCED_TYPES_THEOREM,
        },
    )
}

#[given("a fixture crate with a missing Forall type")]
fn given_a_fixture_crate_with_a_missing_forall_type() {}

#[then("compiling the fixture crate fails with a missing Forall type diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_a_missing_forall_type_diagnostic()
-> Result<(), String> {
    assert_fixture_build_fails_with_lib(&ExpectedBuildFailure {
        lib_rs: EMPTY_TYPES_LIB_RS,
        theorem_path: "theorems/missing-forall-type.theorem",
        theorem_content: MISSING_FORALL_TYPE_THEOREM,
        unexpected_success_msg: "theorem with missing Forall type unexpectedly compiled",
        expected_fragments: &[
            "cannot find type `MissingAccount` in the crate root",
            "MissingAccount",
        ],
    })
}

#[given("a fixture crate with a moved Actions type")]
fn given_a_fixture_crate_with_a_moved_actions_type() {}

#[then("compiling the fixture crate fails with a moved Actions type diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_a_moved_actions_type_diagnostic()
-> Result<(), String> {
    assert_fixture_build_fails_with_lib(&ExpectedBuildFailure {
        lib_rs: MOVED_ACTION_TYPE_LIB_RS,
        theorem_path: "theorems/moved-action-type.theorem",
        theorem_content: MOVED_ACTION_TYPE_THEOREM,
        unexpected_success_msg: "theorem with moved Actions type unexpectedly compiled",
        expected_fragments: &["could not find `old` in the crate root", "old"],
    })
}

/// Asserts that a fixture build using the given theorem file fails and that the
/// build error contains every string in `expected_fragments`.
fn assert_fixture_build_fails_with(
    theorem_path: &str,
    theorem_content: &str,
    unexpected_success_msg: &str,
    expected_fragments: &[&str],
) -> Result<(), String> {
    assert_fixture_build_fails_with_lib(&ExpectedBuildFailure {
        lib_rs: FIXTURE_LIB_RS,
        theorem_path,
        theorem_content,
        unexpected_success_msg,
        expected_fragments,
    })
}

fn assert_fixture_build_fails_with_lib(expected: &ExpectedBuildFailure<'_>) -> Result<(), String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new(expected.lib_rs)?;
    fixture.write(
        Utf8Path::new(expected.theorem_path),
        expected.theorem_content,
    )?;
    let build_error = fixture
        .cargo_build(&guard)
        .err()
        .ok_or_else(|| expected.unexpected_success_msg.to_owned())?;
    for fragment in expected.expected_fragments {
        if !build_error.contains(fragment) {
            return Err(format!(
                "expected build failure to contain {fragment:?}, got:\n{build_error}"
            ));
        }
    }
    Ok(())
}

fn run_fixture_build(lib_rs: &str, spec: &TheoremFixtureSpec<'_>) -> Result<(), String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new(lib_rs)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_build(&guard)
}

#[then("compiling the fixture crate fails with an actionable theorem diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_an_actionable_theorem_diagnostic()
-> Result<(), String> {
    assert_fixture_build_fails_with(
        "theorems/invalid.theorem",
        INVALID_THEOREM,
        "invalid theorem fixture unexpectedly compiled",
        &["theorems/invalid.theorem", "About must be non-empty"],
    )
}

#[then("compiling the fixture crate fails with a missing Kani evidence diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_a_missing_kani_evidence_diagnostic()
-> Result<(), String> {
    assert_fixture_build_fails_with(
        "theorems/missing-kani.theorem",
        MISSING_KANI_EVIDENCE_THEOREM,
        "theorem without Kani evidence unexpectedly compiled",
        &[
            "MissingKaniMacro",
            "does not declare required `Evidence.kani` configuration",
        ],
    )
}

#[then("compiling the fixture crate fails with the partial Kani evidence diagnostic")]
fn then_compiling_the_fixture_crate_fails_with_the_partial_kani_evidence_diagnostic()
-> Result<(), String> {
    assert_fixture_build_fails_with(
        "theorems/partial-kani.theorem",
        PARTIAL_KANI_EVIDENCE_THEOREM,
        "multi-document theorem with partial Kani evidence unexpectedly compiled",
        &[
            "PartialKaniMacro",
            "does not declare required `Evidence.kani` configuration",
        ],
    )
}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A valid theorem file compiles without Kani installed"
)]
fn a_valid_theorem_file_compiles_without_kani_installed() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A valid theorem file exposes a Kani proof harness when cargo-kani is installed"
)]
fn a_valid_theorem_file_exposes_a_kani_proof_harness() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A multi-document theorem file compiles without Kani installed"
)]
fn a_multi_document_theorem_file_compiles_without_kani_installed() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "Referenced Forall and Actions types are checked during ordinary builds"
)]
fn referenced_forall_and_actions_types_are_checked_during_ordinary_builds() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A missing Forall type fails ordinary compilation"
)]
fn a_missing_forall_type_fails_ordinary_compilation() {}

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A moved Actions type fails ordinary compilation"
)]
fn a_moved_actions_type_fails_ordinary_compilation() {}

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

#[scenario(
    path = "tests/features/theorem_file_macro.feature",
    name = "A multi-document theorem file with partial Kani evidence fails macro expansion"
)]
fn a_multi_document_theorem_file_with_partial_kani_evidence_fails_macro_expansion() {}

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
