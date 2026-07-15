//! Fixture crate construction for theorem-file macro behavioural tests.
//!
//! This module is the filesystem side of `theorem_file_macro_bdd.rs`: it
//! creates isolated Cargo projects that include the real build script and
//! generated-suite helpers from the root crate. It depends on `cargo_runner`
//! for serialized Cargo execution, keeping fixture layout separate from
//! process orchestration.

use camino::Utf8Path;
use theoremc_core::path_format::{normalize_path_separators, toml_basic_string_value};

use super::cargo_runner::{CargoGuard, CargoSubcommand, cargo_run, cargo_run_output};
use test_helpers::FixtureCrate as CommonFixtureCrate;

const ROOT_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
pub(crate) const FIXTURE_BUILD_DEPENDENCIES: &str = concat!(
    "camino = \"1.2.2\"\n",
    "cap-std = { version = \"4.0.2\", features = [\"fs_utf8\"] }\n",
    "thiserror = \"2.0.18\"\n",
);

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

    pub(crate) fn cargo_build(&self, guard: &CargoGuard<'_>) -> Result<(), String> {
        self.cargo_run(CargoSubcommand::Build, guard)
    }

    pub(crate) fn cargo_kani_list(&self, guard: &CargoGuard<'_>) -> Result<String, String> {
        cargo_run_output(self.inner.manifest_dir(), CargoSubcommand::KaniList, guard)
    }

    fn cargo_run(&self, subcommand: CargoSubcommand, guard: &CargoGuard<'_>) -> Result<(), String> {
        cargo_run(self.inner.manifest_dir(), subcommand, guard)
    }
}

pub(crate) fn fixture_cargo_toml() -> String {
    fixture_cargo_toml_for(ROOT_MANIFEST_DIR)
}

pub(crate) fn fixture_cargo_toml_for(root_manifest_dir: &str) -> String {
    let normalized_root_manifest_dir = normalize_path_separators(root_manifest_dir);
    let escaped_root_manifest_dir = toml_basic_string_value(&normalized_root_manifest_dir);
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
            "{build_dependencies}",
        ),
        root_manifest_dir = escaped_root_manifest_dir,
        build_dependencies = FIXTURE_BUILD_DEPENDENCIES
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

    use super::{FIXTURE_BUILD_DEPENDENCIES, fixture_cargo_toml_for};

    #[test]
    fn fixture_cargo_toml_normalizes_windows_paths() {
        // Simulate a Windows-style `CARGO_MANIFEST_DIR` with backslash
        // separators. `fixture_cargo_toml` reads `ROOT_MANIFEST_DIR`, which is
        // set at compile time, but this test only needs the normalization
        // logic.
        let windows_path = r"C:\Users\user\projects\theoremc";
        let toml = fixture_cargo_toml_for(windows_path);

        assert!(
            !toml.contains('\\'),
            "TOML must not contain backslashes after normalization; got:\n{toml}",
        );
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
}
