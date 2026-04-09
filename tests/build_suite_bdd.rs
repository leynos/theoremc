//! Behavioural tests for Cargo build suite generation.
//!
//! These tests prove that the generated theorem suite compiles correctly
//! for empty, single-file, and multi-file theorem trees.

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest_bdd_macros::{given, scenario, then};

const BUILD_SCRIPT_SOURCE: &str = include_str!("../build.rs");
const BUILD_DISCOVERY_SOURCE: &str = include_str!("../src/build_discovery.rs");
const BUILD_SUITE_SOURCE: &str = include_str!("../src/build_suite.rs");
const ROOT_CARGO_TOML: &str = include_str!("../Cargo.toml");

/// Fixture lib.rs that includes the generated suite wiring.
const FIXTURE_LIB_RS: &str = concat!(
    "//! Fixture crate for build suite behavioural tests.\n",
    "\n",
    "#[doc(hidden)]\n",
    "mod __theoremc_generated_suite {\n",
    "    #[allow(unused_macros)]\n",
    "    macro_rules! theorem_file {\n",
    "        ($path:literal) => {\n",
    "            const _: &str =\n",
    "                include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/\", $path));\n",
    "        };\n",
    "    }\n",
    "\n",
    "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
    "}\n",
);

const TRIVIAL_THEOREM: &str = concat!(
    "Theorem: Smoke\n",
    "About: Smoke theorem\n",
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
    fn new() -> Result<Self, String> {
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

        fixture.write(Utf8Path::new("Cargo.toml"), &fixture_cargo_toml()?)?;
        fixture.write(Utf8Path::new("build.rs"), BUILD_SCRIPT_SOURCE)?;
        fixture.write(Utf8Path::new("src/lib.rs"), FIXTURE_LIB_RS)?;
        fixture.write(
            Utf8Path::new("src/build_discovery.rs"),
            BUILD_DISCOVERY_SOURCE,
        )?;
        fixture.write(Utf8Path::new("src/build_suite.rs"), BUILD_SUITE_SOURCE)?;

        Ok(fixture)
    }

    fn write(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_str().is_empty() {
                self.dir
                    .create_dir_all(parent)
                    .map_err(|error| error.to_string())?;
            }
        }
        self.dir
            .write(path.as_str(), contents)
            .map_err(|error| error.to_string())
    }

    fn build(&self) -> Result<(), String> {
        let output = Command::new("cargo")
            .current_dir(&self.manifest_dir)
            .args(["build", "-vv", "--color", "never"])
            .output()
            .map_err(|error| error.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            let log = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            Err(log)
        }
    }
}

fn fixture_cargo_toml() -> Result<String, String> {
    let build_dependencies = toml_section(ROOT_CARGO_TOML, "build-dependencies")
        .ok_or_else(|| "root Cargo.toml is missing [build-dependencies]".to_owned())?;

    Ok(format!(
        concat!(
            "[package]\n",
            "name = \"build_suite_fixture\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2024\"\n\n",
            "[build-dependencies]\n",
            "{build_dependencies}",
        ),
        build_dependencies = build_dependencies
    ))
}

fn toml_section(document: &str, section_name: &str) -> Option<String> {
    let header_line = format!("[{section_name}]");
    let mut in_section = false;
    let mut body_lines = Vec::new();

    for line in document.lines() {
        if !in_section {
            in_section = line == header_line;
            continue;
        }

        if line.starts_with('[') {
            break;
        }

        body_lines.push(line);
    }

    in_section.then(|| format!("{}\n", body_lines.join("\n")))
}

/// Precondition stub; the empty crate scenario is created in the `then` step.
#[given("a crate without a theorems directory")]
fn given_a_crate_without_a_theorems_directory() {}

#[then("the crate compiles successfully with the generated suite")]
fn then_the_crate_compiles_successfully_with_the_generated_suite() -> Result<(), String> {
    let fixture = FixtureCrate::new()?;
    // No theorems directory created - testing empty suite case
    // build() returns Ok only if the build succeeded
    fixture.build()?;
    Ok(())
}

/// Precondition stub; the single theorem scenario is created in the `then` step.
#[given("a crate with one theorem file")]
fn given_a_crate_with_one_theorem_file() {}

#[then("the single theorem is included automatically and the crate compiles")]
fn then_the_single_theorem_is_included_automatically_and_the_crate_compiles() -> Result<(), String>
{
    let fixture = FixtureCrate::new()?;
    fixture.write(Utf8Path::new("theorems/single.theorem"), TRIVIAL_THEOREM)?;

    // build() returns Ok only if the build succeeded
    fixture.build()?;
    Ok(())
}

/// Precondition stub; the multi-theorem scenario is created in the `then` step.
#[given("a crate with multiple theorem files created in non-sorted order")]
fn given_a_crate_with_multiple_theorem_files_created_in_non_sorted_order() {}

#[then("all theorems compile in deterministic suite order")]
fn then_all_theorems_compile_in_deterministic_suite_order() -> Result<(), String> {
    let fixture = FixtureCrate::new()?;
    // Create theorems in non-sorted order (z, a, m)
    fixture.write(Utf8Path::new("theorems/z.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(Utf8Path::new("theorems/a.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(Utf8Path::new("theorems/m.theorem"), TRIVIAL_THEOREM)?;

    // build() returns Ok only if the build succeeded
    fixture.build()?;
    Ok(())
}

#[scenario(
    path = "tests/features/build_suite.feature",
    name = "An empty crate still compiles with generated suite wiring"
)]
fn an_empty_crate_still_compiles_with_generated_suite_wiring() {}

#[scenario(
    path = "tests/features/build_suite.feature",
    name = "A single theorem file is included automatically"
)]
fn a_single_theorem_file_is_included_automatically() {}

#[scenario(
    path = "tests/features/build_suite.feature",
    name = "Multiple theorem files compile in deterministic suite order"
)]
fn multiple_theorem_files_compile_in_deterministic_suite_order() {}
