//! Behavioural tests for real `theorem_file!` proc-macro expansion.

use std::process::Command;
use std::sync::{Mutex, PoisonError};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest_bdd_macros::{given, scenario, then};
use theoremc::mangle::{mangle_module_path, mangle_theorem_harness};

struct TheoremFixtureSpec<'a> {
    path: &'a str,
    names: &'a [&'a str],
    content: &'a str,
}

#[derive(Clone, Copy)]
enum CargoSubcommand {
    Build,
    Test,
}

impl CargoSubcommand {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Test => "test",
        }
    }
}

/// Proof that `FIXTURE_CARGO_LOCK` is held by the current thread.
///
/// Pass a `CargoGuard<'_>` to [`FixtureCrate::cargo_run`] to enforce at compile
/// time that no caller bypasses the serialisation contract.
struct CargoGuard<'a> {
    _guard: std::sync::MutexGuard<'a, ()>,
}

impl CargoGuard<'_> {
    fn acquire() -> Self {
        Self {
            _guard: FIXTURE_CARGO_LOCK
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        }
    }
}

const BUILD_SCRIPT_SOURCE: &str = include_str!("../build.rs");
const BUILD_DISCOVERY_SOURCE: &str = include_str!("../src/build_discovery.rs");
const BUILD_SUITE_SOURCE: &str = include_str!("../src/build_suite.rs");
const ROOT_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
const FIXTURE_BUILD_DEPENDENCIES: &str = concat!(
    "camino = \"1.2.2\"\n",
    "cap-std = { version = \"4.0.2\", features = [\"fs_utf8\"] }\n",
    "thiserror = \"2.0.18\"\n",
);
/// Serializes all fixture `cargo` invocations.
///
/// `cargo` writes lock files, registry caches, and incremental artefacts to
/// paths derived from the manifest directory. When multiple fixture crates run
/// `cargo build` concurrently they race on the shared `~/.cargo` registry
/// cache and on any workspace-level `Cargo.lock`, producing spurious build
/// failures even with isolated `CARGO_TARGET_DIR` values. A single global
/// mutex is the minimal correct serialization mechanism for in-process test
/// parallelism; the alternative--spawning each fixture in a fully isolated
/// toolchain environment--would require per-test network access and is
/// disproportionate for a test suite this size.
static FIXTURE_CARGO_LOCK: Mutex<()> = Mutex::new(());

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

struct FixtureCrate {
    _temp_dir: tempfile::TempDir,
    manifest_dir: Utf8PathBuf,
    dir: Dir,
}

impl FixtureCrate {
    fn new(lib_rs: &str) -> Result<Self, String> {
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let manifest_dir = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| "temp dir path is not valid UTF-8".to_owned())?
            .to_path_buf();
        let dir = Dir::open_ambient_dir(&manifest_dir, ambient_authority())
            .map_err(|error| error.to_string())?;
        let fixture = Self {
            _temp_dir: temp_dir,
            manifest_dir,
            dir,
        };

        fixture.write(Utf8Path::new("Cargo.toml"), &fixture_cargo_toml())?;
        fixture.write(Utf8Path::new("build.rs"), BUILD_SCRIPT_SOURCE)?;
        fixture.write(Utf8Path::new("src/lib.rs"), lib_rs)?;
        fixture.write(
            Utf8Path::new("src/build_discovery.rs"),
            BUILD_DISCOVERY_SOURCE,
        )?;
        fixture.write(Utf8Path::new("src/build_suite.rs"), BUILD_SUITE_SOURCE)?;

        Ok(fixture)
    }

    fn write(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        if let Some(parent) = path.parent()
            && !parent.as_str().is_empty()
        {
            self.dir
                .create_dir_all(parent)
                .map_err(|error| error.to_string())?;
        }
        self.dir
            .write(path.as_str(), contents)
            .map_err(|error| error.to_string())
    }

    fn cargo_test(&self, guard: &CargoGuard<'_>) -> Result<(), String> {
        self.cargo_run(CargoSubcommand::Test, guard)
    }

    fn cargo_build(&self, guard: &CargoGuard<'_>) -> Result<(), String> {
        self.cargo_run(CargoSubcommand::Build, guard)
    }

    /// Runs a Cargo subcommand in the fixture crate directory.
    ///
    /// `CARGO_TARGET_DIR` is set to a per-fixture subdirectory to avoid
    /// artefact collisions. The caller must supply a [`CargoGuard`] proving
    /// that `FIXTURE_CARGO_LOCK` is held, because registry cache access and
    /// workspace-level lock files are still shared across invocations.
    ///
    /// # Timeout
    ///
    /// `Command::output()` blocks until the subprocess exits. No explicit
    /// timeout is imposed: `cargo` is a first-party trusted tool, and
    /// introducing cross-platform `wait_timeout` polling would add significant
    /// complexity disproportionate to this test suite's scope. If a runaway
    /// `cargo` process causes a stall in CI, the job-level timeout enforced by
    /// the CI runtime (e.g. GitHub Actions' `timeout-minutes`) provides the
    /// outer bound.
    fn cargo_run(
        &self,
        subcommand: CargoSubcommand,
        _guard: &CargoGuard<'_>,
    ) -> Result<(), String> {
        let target_dir = self.manifest_dir.join("target");
        let output = Command::new("cargo")
            .current_dir(&self.manifest_dir)
            .env("CARGO_TARGET_DIR", target_dir.as_str())
            .args([subcommand.as_str(), "--color", "never"])
            .output()
            .map_err(|error| error.to_string())?;
        command_result(&output)
    }
}

fn command_result(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ))
}

fn fixture_cargo_toml() -> String {
    let root_manifest_dir = ROOT_MANIFEST_DIR.replace('\\', "/");
    format!(
        concat!(
            "[package]\n",
            "name = \"theorem_file_macro_fixture\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2024\"\n\n",
            "[dependencies]\n",
            "theoremc = {{ path = '{root_manifest_dir}', features = [\"test-support\"] }}\n\n",
            "[dev-dependencies]\n",
            "theoremc = {{ path = '{root_manifest_dir}', features = [\"test-support\"] }}\n\n",
            "[build-dependencies]\n",
            "{build_dependencies}",
        ),
        root_manifest_dir = root_manifest_dir,
        build_dependencies = FIXTURE_BUILD_DEPENDENCIES
    )
}

fn fixture_lib_rs(spec: &TheoremFixtureSpec<'_>) -> String {
    let module_name = mangle_module_path(spec.path).module_name().to_owned();
    let harnesses: Vec<String> = spec
        .names
        .iter()
        .map(|theorem| {
            mangle_theorem_harness(spec.path, theorem)
                .identifier()
                .to_owned()
        })
        .collect();
    let harness_assertions = harnesses
        .iter()
        .map(|harness| format!("                super::{module_name}::kani::{harness},"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        concat!(
            "//! Fixture crate for theorem_file macro behavioural tests.\n\n",
            "#[doc(hidden)]\n",
            "mod __theoremc_generated_suite {{\n",
            "    #[cfg(theoremc_has_theorems)]\n",
            "    use theoremc::theorem_file;\n",
            "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
            "\n",
            "    #[cfg(test)]\n",
            "    mod generated_symbol_tests {{\n",
            "        #[test]\n",
            "        fn generated_symbols_exist() {{\n",
            "            let _: [fn(); {count}] = [\n",
            "{harness_assertions}\n",
            "            ];\n",
            "        }}\n",
            "    }}\n",
            "}}\n",
        ),
        count = harnesses.len(),
        harness_assertions = harness_assertions
    )
}

fn run_valid_fixture_test(spec: &TheoremFixtureSpec<'_>) -> Result<(), String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new(&fixture_lib_rs(spec))?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_test(&guard)
}

const fn invalid_fixture_lib_rs() -> &'static str {
    concat!(
        "//! Fixture crate for theorem_file macro behavioural tests.\n\n",
        "#[doc(hidden)]\n",
        "mod __theoremc_generated_suite {\n",
        "    #[cfg(theoremc_has_theorems)]\n",
        "    use theoremc::theorem_file;\n",
        "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
        "}\n",
    )
}

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
