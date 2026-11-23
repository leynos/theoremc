//! Fixture crate construction for theorem-file macro behavioural tests.
//!
//! This module is the filesystem side of `theorem_file_macro_bdd.rs`: it
//! creates isolated Cargo projects that include the real build script and
//! generated-suite helpers from the root crate. It depends on `cargo_runner`
//! for serialised Cargo execution, keeping fixture layout separate from
//! process orchestration.

use camino::Utf8Path;
use test_helpers::{
    FixtureCrate as CommonFixtureCrate, build_support_dependency_for, checkout_root, toml_path,
};

use super::cargo_runner::{CargoSubcommand, cargo_run, cargo_run_output};

pub(crate) struct TheoremFixtureSpec<'a> {
    pub(crate) path: &'a str,
    pub(crate) content: &'a str,
}

pub(crate) struct FixtureCrate {
    inner: CommonFixtureCrate,
}

impl FixtureCrate {
    pub(crate) fn new(lib_rs: &str) -> Result<Self, String> {
        let cargo_toml = fixture_cargo_toml();
        Ok(Self {
            inner: CommonFixtureCrate::new(&cargo_toml, lib_rs)?,
        })
    }

    pub(crate) fn write(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        self.inner.write(path, contents)
    }

    pub(crate) fn cargo_build(&self) -> Result<(), String> {
        self.cargo_run(CargoSubcommand::Build)
    }

    pub(crate) fn cargo_kani_list(&self) -> Result<String, String> {
        cargo_run_output(
            self.inner.manifest_dir(),
            self.inner.target_dir(),
            CargoSubcommand::KaniList,
        )
    }

    pub(crate) fn cargo_kani_verify(&self) -> Result<String, String> {
        cargo_run_output(
            self.inner.manifest_dir(),
            self.inner.target_dir(),
            CargoSubcommand::Kani,
        )
    }

    fn cargo_run(&self, subcommand: CargoSubcommand) -> Result<(), String> {
        cargo_run(
            self.inner.manifest_dir(),
            self.inner.target_dir(),
            subcommand,
        )
    }
}

/// Builds the manifest a macro fixture uses.
///
/// The fixture is its own workspace root, and it names the checkout's
/// build-support crate and the root crate by path. Both paths are stable across
/// fixtures, so the shared target compiles each of those crates once.
pub(crate) fn fixture_cargo_toml() -> String {
    fixture_cargo_toml_for(&checkout_root())
}

/// Builds the manifest a macro fixture uses for `checkout_root`.
pub(crate) fn fixture_cargo_toml_for(checkout_root: &Utf8Path) -> String {
    format!(
        concat!(
            "[package]\n",
            "name = \"theorem_file_macro_fixture\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2024\"\n\n",
            "[workspace]\n\n",
            "[dependencies]\n",
            "theoremc = {{ path = {root}, features = [\"test-support\"] }}\n\n",
            "[dev-dependencies]\n",
            "theoremc = {{ path = {root}, features = [\"test-support\"] }}\n\n",
            "[build-dependencies]\n",
            "{build_support}\n",
        ),
        root = toml_path(checkout_root),
        build_support = build_support_dependency_for(checkout_root),
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
    let fixture = FixtureCrate::new(lib_rs)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_build()
}

/// Builds the fixture crate and runs `cargo kani` over every generated harness.
pub(crate) fn build_fixture_and_verify_kani_harnesses(
    spec: &TheoremFixtureSpec<'_>,
) -> Result<String, String> {
    let fixture = FixtureCrate::new(FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_kani_verify()
}

pub(crate) fn build_fixture_and_list_kani_harnesses(
    spec: &TheoremFixtureSpec<'_>,
) -> Result<String, String> {
    let fixture = FixtureCrate::new(FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_kani_list()
}

#[cfg(test)]
#[path = "fixture_crate_tests.rs"]
mod tests;
