//! Fixture crate construction for theorem-file macro behavioural tests.

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use theoremc::mangle::{mangle_module_path, mangle_theorem_harness};

use super::cargo_runner::{CargoGuard, CargoSubcommand, cargo_run};

const BUILD_SCRIPT_SOURCE: &str = include_str!("../../build.rs");
const BUILD_DISCOVERY_SOURCE: &str = include_str!("../../src/build_discovery.rs");
const BUILD_SUITE_SOURCE: &str = include_str!("../../src/build_suite.rs");
const ROOT_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
pub(crate) const FIXTURE_BUILD_DEPENDENCIES: &str = concat!(
    "camino = \"1.2.2\"\n",
    "cap-std = { version = \"4.0.2\", features = [\"fs_utf8\"] }\n",
    "thiserror = \"2.0.18\"\n",
);

pub(crate) struct TheoremFixtureSpec<'a> {
    pub(crate) path: &'a str,
    pub(crate) names: &'a [&'a str],
    pub(crate) content: &'a str,
}

pub(crate) struct FixtureCrate {
    _temp_dir: tempfile::TempDir,
    manifest_dir: Utf8PathBuf,
    dir: Dir,
}

impl FixtureCrate {
    pub(crate) fn new(lib_rs: &str) -> Result<Self, String> {
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

    pub(crate) fn write(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
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

    pub(crate) fn cargo_test(&self, guard: &CargoGuard<'_>) -> Result<(), String> {
        self.cargo_run(CargoSubcommand::Test, guard)
    }

    pub(crate) fn cargo_build(&self, guard: &CargoGuard<'_>) -> Result<(), String> {
        self.cargo_run(CargoSubcommand::Build, guard)
    }

    fn cargo_run(&self, subcommand: CargoSubcommand, guard: &CargoGuard<'_>) -> Result<(), String> {
        cargo_run(&self.manifest_dir, subcommand, guard)
    }
}

pub(crate) fn fixture_cargo_toml() -> String {
    let root_manifest_dir = ROOT_MANIFEST_DIR.replace('\\', "/");
    fixture_cargo_toml_for(&root_manifest_dir)
}

pub(crate) fn fixture_cargo_toml_for(root_manifest_dir: &str) -> String {
    let normalized_root_manifest_dir = root_manifest_dir.replace('\\', "/");
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

fn toml_basic_string_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

pub(crate) fn run_valid_fixture_test(spec: &TheoremFixtureSpec<'_>) -> Result<(), String> {
    let guard = CargoGuard::acquire();
    let fixture = FixtureCrate::new(&fixture_lib_rs(spec))?;
    fixture.write(Utf8Path::new(spec.path), spec.content)?;
    fixture.cargo_test(&guard)
}

pub(crate) const fn invalid_fixture_lib_rs() -> &'static str {
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
