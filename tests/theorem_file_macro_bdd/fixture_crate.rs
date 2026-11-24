//! Fixture crate construction for theorem-file macro behavioural tests.
//!
//! This module is the filesystem side of `theorem_file_macro_bdd.rs`: it
//! creates isolated Cargo projects that include the real build script and
//! generated-suite helpers from the root crate. It depends on `cargo_runner`
//! for serialized Cargo execution, keeping fixture layout separate from
//! process orchestration.

use camino::Utf8Path;
use theoremc_core::path_format::{normalize_path_separators, toml_basic_string_value};

use super::cargo_runner::{
    CargoGuard, CargoSubcommand, cargo_run_denying_warnings, cargo_run_output,
};
use test_helpers::FixtureCrate as CommonFixtureCrate;

const ROOT_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
pub(crate) struct TheoremFixtureSpec<'a> {
    pub(crate) path: &'a str,
    pub(crate) content: &'a str,
}

/// Selects whether a fixture consumer registers `cfg(kani)`.
///
/// A consumer that registers it proves the generated gate stays silent when
/// the `cfg` is expected. A consumer that does not proves the generated gate
/// still suppresses the resulting `unexpected_cfgs` warning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConsumerCfgRegistration {
    /// The consumer declares `check-cfg` for `cfg(kani)`.
    Registered,
    /// The consumer declares no `cfg(kani)` registration.
    Unregistered,
}

/// Selects whether a fixture Cargo invocation denies warnings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DenyWarnings {
    /// Keep the invocation's own lint levels.
    No,
    /// Deny warnings in the child build.
    Yes,
}

pub(crate) struct FixtureCrate {
    inner: CommonFixtureCrate,
}

impl FixtureCrate {
    pub(crate) fn new(lib_rs: &str) -> Result<Self, String> {
        Self::new_for_consumer(lib_rs, ConsumerCfgRegistration::Unregistered)
    }

    /// Creates a fixture whose manifest matches the requested registration.
    pub(crate) fn new_for_consumer(
        lib_rs: &str,
        registration: ConsumerCfgRegistration,
    ) -> Result<Self, String> {
        let cargo_toml = fixture_cargo_toml_for_consumer(ROOT_MANIFEST_DIR, registration);
        Ok(Self {
            inner: CommonFixtureCrate::new(&cargo_toml, lib_rs)?,
        })
    }

    pub(crate) fn write(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        self.inner.write(path, contents)
    }

    /// Builds the fixture with `-D warnings` in the child Cargo process.
    pub(crate) fn cargo_build_denying_warnings(
        &self,
        guard: &CargoGuard<'_>,
    ) -> Result<(), String> {
        self.run(CargoSubcommand::Build, DenyWarnings::Yes, guard)
            .map(|_| ())
    }

    pub(crate) fn cargo_kani_list(&self, guard: &CargoGuard<'_>) -> Result<String, String> {
        self.run(CargoSubcommand::KaniList, DenyWarnings::No, guard)
    }

    pub(crate) fn cargo_kani_verify(&self, guard: &CargoGuard<'_>) -> Result<String, String> {
        self.run(CargoSubcommand::Kani, DenyWarnings::No, guard)
    }

    pub(crate) fn cargo_build(&self, guard: &CargoGuard<'_>) -> Result<(), String> {
        self.run(CargoSubcommand::Build, DenyWarnings::No, guard)
            .map(|_| ())
    }

    /// Runs one subcommand against this fixture, so every call site shares one
    /// statement of the manifest directory, target directory, and guard.
    fn run(
        &self,
        subcommand: CargoSubcommand,
        denial: DenyWarnings,
        guard: &CargoGuard<'_>,
    ) -> Result<String, String> {
        let manifest_dir = self.inner.manifest_dir();
        let target_dir = self.inner.target_dir();
        match denial {
            DenyWarnings::Yes => {
                cargo_run_denying_warnings(manifest_dir, target_dir, subcommand, guard)
                    .map(|()| String::new())
            }
            DenyWarnings::No => cargo_run_output(manifest_dir, target_dir, subcommand, guard),
        }
    }
}

pub(crate) fn fixture_cargo_toml_for(root_manifest_dir: &str) -> String {
    fixture_cargo_toml_for_consumer(root_manifest_dir, ConsumerCfgRegistration::Unregistered)
}

/// Builds the fixture manifest for the requested `cfg(kani)` registration.
pub(crate) fn fixture_cargo_toml_for_consumer(
    root_manifest_dir: &str,
    registration: ConsumerCfgRegistration,
) -> String {
    let normalized_root_manifest_dir = normalize_path_separators(root_manifest_dir);
    let escaped_root_manifest_dir = toml_basic_string_value(&normalized_root_manifest_dir);
    let build_support_dir = format!("{normalized_root_manifest_dir}/crates/theoremc-build-support");
    let escaped_build_support_dir = toml_basic_string_value(&build_support_dir);
    let lint_table = match registration {
        ConsumerCfgRegistration::Registered => concat!(
            "[lints.rust]\n",
            "unexpected_cfgs = { level = \"warn\", check-cfg = [\"cfg(kani)\"] }\n",
        ),
        ConsumerCfgRegistration::Unregistered => "",
    };
    format!(
        concat!(
            "[package]\n",
            "name = \"theorem_file_macro_fixture\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2024\"\n\n",
            "[dependencies]\n",
            "theoremc = {{ path = \"{root_manifest_dir}\", features = [\"test-support\"] }}\n\n",
            "[dev-dependencies]\n",
            "theoremc = {{ path = \"{root_manifest_dir}\", features = [\"test-support\"] }}\n\n",
            "[build-dependencies]\n",
            "theoremc-build-support = {{ path = \"{build_support_dir}\" }}\n\n",
            "{lint_table}",
        ),
        root_manifest_dir = escaped_root_manifest_dir,
        build_support_dir = escaped_build_support_dir,
        lint_table = lint_table,
    )
}

pub(crate) const FIXTURE_LIB_RS: &str = concat!(
    "//! Fixture crate for theorem_file macro behavioural tests.\n\n",
    "#[doc(hidden)]\n",
    "mod __theoremc_generated_suite {\n",
    "    #[cfg(theoremc_has_theorems)]\n",
    "    use theoremc::theorem_file;\n",
    "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
    "}\n",
);

pub(crate) fn run_valid_fixture_build(spec: &TheoremFixtureSpec<'_>) -> Result<(), String> {
    run_fixture_build(FIXTURE_LIB_RS, spec)
}

pub(crate) fn run_fixture_build(lib_rs: &str, spec: &TheoremFixtureSpec<'_>) -> Result<(), String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new(lib_rs)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_build(&guard)
}

/// Builds an independent consumer with warnings denied.
///
/// `registration` decides whether the consumer declares `check-cfg` for
/// `cfg(kani)`. The generated gate must compile under both configurations, so
/// a strict-warning build is the proof rather than a plain build.
pub(crate) fn run_fixture_build_denying_warnings(
    lib_rs: &str,
    registration: ConsumerCfgRegistration,
    spec: &TheoremFixtureSpec<'_>,
) -> Result<(), String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new_for_consumer(lib_rs, registration)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_build_denying_warnings(&guard)
}

/// Builds the fixture crate and runs `cargo kani` over every generated harness.
pub(crate) fn build_fixture_and_verify_kani_harnesses(
    spec: &TheoremFixtureSpec<'_>,
) -> Result<String, String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new(FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_kani_verify(&guard)
}

pub(crate) fn build_fixture_and_list_kani_harnesses(
    spec: &TheoremFixtureSpec<'_>,
) -> Result<String, String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new(FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_kani_list(&guard)
}

mod tests {
    //! Tests for Cargo TOML fixture generation.

    use rstest::rstest;

    use super::{ConsumerCfgRegistration, fixture_cargo_toml_for, fixture_cargo_toml_for_consumer};

    /// `fixture_cargo_toml` reads `ROOT_MANIFEST_DIR`, which is set at compile
    /// time, so these cases pass a path directly to the builder.
    #[rstest]
    #[case::windows_backslashes(
        r"C:\Users\user\projects\theoremc",
        "\"C:/Users/user/projects/theoremc\"",
        Some('\\')
    )]
    #[case::nested_quotes(
        "/home/user/project's/\"theoremc\"",
        "path = \"/home/user/project's/\\\"theoremc\\\"\"",
        None
    )]
    fn fixture_cargo_toml_renders_paths_safely(
        #[case] root: &str,
        #[case] expected: &str,
        #[case] forbidden: Option<char>,
    ) {
        let toml = fixture_cargo_toml_for(root);

        assert!(
            toml.contains(expected),
            "expected {expected:?} in the fixture manifest; got:\n{toml}",
        );
        if let Some(character) = forbidden {
            assert!(
                !toml.contains(character),
                "TOML must not contain {character:?} after normalization; got:\n{toml}",
            );
        }
        assert!(
            toml.contains("theoremc-build-support = { path = "),
            "the build-support dependency should be present; got:\n{toml}",
        );
    }

    /// Asserts whether the manifest carries a `cfg(kani)` registration.
    fn declares_cfg_kani(registration: ConsumerCfgRegistration) -> bool {
        fixture_cargo_toml_for_consumer("/repo", registration)
            .contains("check-cfg = [\"cfg(kani)\"]")
    }

    #[rstest]
    #[case::registered(ConsumerCfgRegistration::Registered, true)]
    #[case::unregistered(ConsumerCfgRegistration::Unregistered, false)]
    fn consumer_manifest_matches_its_cfg_kani_registration(
        #[case] registration: ConsumerCfgRegistration,
        #[case] expected: bool,
    ) {
        assert_eq!(
            declares_cfg_kani(registration),
            expected,
            "consumer manifest should {}a cfg(kani) registration",
            if expected { "carry " } else { "omit " },
        );
    }
}
