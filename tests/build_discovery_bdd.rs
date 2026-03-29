//! Behavioural tests for Cargo build discovery of theorem files.
use std::process::Command;
use std::thread;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest_bdd_macros::{given, scenario, then};

const BUILD_SCRIPT_SOURCE: &str = include_str!("../build.rs");
const BUILD_DISCOVERY_SOURCE: &str = include_str!("../src/build_discovery.rs");
const FIXTURE_CARGO_TOML: &str = r#"[package]
name = "build_discovery_fixture"
version = "0.1.0"
edition = "2024"

[build-dependencies]
camino = "1.2.2"
cap-std = { version = "4.0.2", features = ["fs_utf8"] }
thiserror = "2.0.18"
"#;
const FIXTURE_LIB_RS: &str = "//! Fixture crate for build discovery behavioural tests.\n";
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

        fixture.write("Cargo.toml", FIXTURE_CARGO_TOML)?;
        fixture.write("build.rs", BUILD_SCRIPT_SOURCE)?;
        fixture.write("src/lib.rs", FIXTURE_LIB_RS)?;
        fixture.write("src/build_discovery.rs", BUILD_DISCOVERY_SOURCE)?;

        Ok(fixture)
    }

    fn write(&self, path: &str, contents: &str) -> Result<(), String> {
        if let Some(parent) = Utf8Path::new(path).parent() {
            if !parent.as_str().is_empty() {
                self.dir
                    .create_dir_all(parent)
                    .map_err(|error| error.to_string())?;
            }
        }
        self.dir
            .write(path, contents)
            .map_err(|error| error.to_string())
    }

    fn overwrite_in_place(&self, path: &str, contents: &str) -> Result<(), String> {
        std::fs::write(self.manifest_dir.join(path), contents).map_err(|error| error.to_string())
    }

    fn build(&self) -> Result<String, String> {
        let output = Command::new("cargo")
            .current_dir(&self.manifest_dir)
            .args(["build", "-vv", "--color", "never"])
            .output()
            .map_err(|error| error.to_string())?;
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );

        if output.status.success() {
            Ok(log)
        } else {
            Err(log)
        }
    }
}

fn pause_for_timestamp_tick() {
    thread::sleep(Duration::from_secs(1));
}

fn build_script_ran(log: &str) -> bool {
    log.contains("cargo::rerun-if-changed=theorems")
}

fn log_contains(log: &str, needle: &str) -> Result<(), String> {
    if log.contains(needle) {
        Ok(())
    } else {
        Err(format!(
            "expected build log to contain '{needle}', got:\n{log}"
        ))
    }
}

fn log_omits(log: &str, needle: &str) -> Result<(), String> {
    if log.contains(needle) {
        Err(format!(
            "expected build log to omit '{needle}', got:\n{log}"
        ))
    } else {
        Ok(())
    }
}

#[given("a crate with nested theorem files")]
fn given_a_crate_with_nested_theorem_files() {}

#[then("building twice stays fresh and editing a theorem reruns the build script")]
fn then_building_twice_stays_fresh_and_editing_a_theorem_reruns_the_build_script()
-> Result<(), String> {
    let fixture = FixtureCrate::new()?;
    fixture.write("theorems/root.theorem", TRIVIAL_THEOREM)?;
    fixture.write("theorems/nested/alpha.theorem", TRIVIAL_THEOREM)?;

    let first_build = fixture.build()?;
    if !build_script_ran(&first_build) {
        return Err(format!(
            "first build should run the build script, got:\n{first_build}"
        ));
    }
    log_contains(
        &first_build,
        "cargo::rerun-if-changed=theorems/nested/alpha.theorem",
    )?;
    log_contains(&first_build, "cargo::rerun-if-changed=theorems/nested")?;

    let second_build = fixture.build()?;
    if build_script_ran(&second_build) {
        return Err(format!(
            "second unchanged build should stay fresh, got:\n{second_build}"
        ));
    }

    pause_for_timestamp_tick();
    fixture.overwrite_in_place(
        "theorems/nested/alpha.theorem",
        &format!("{TRIVIAL_THEOREM}\n# edited\n"),
    )?;

    let third_build = fixture.build()?;
    if !build_script_ran(&third_build) {
        return Err(format!(
            "editing a theorem file should rerun the build script, got:\n{third_build}"
        ));
    }
    Ok(())
}

#[given("a crate with ignored non-theorem files under theorems")]
fn given_a_crate_with_ignored_non_theorem_files_under_theorems() {}

#[then("the build script emits only theorem inputs")]
fn then_the_build_script_emits_only_theorem_inputs() -> Result<(), String> {
    let fixture = FixtureCrate::new()?;
    fixture.write("theorems/kept.theorem", TRIVIAL_THEOREM)?;
    fixture.write("theorems/ignored.txt", "not a theorem")?;

    let first_build = fixture.build()?;
    if !build_script_ran(&first_build) {
        return Err(format!(
            "first build should run the build script, got:\n{first_build}"
        ));
    }
    log_omits(&first_build, "cargo::rerun-if-changed=theorems/ignored.txt")?;
    log_contains(
        &first_build,
        "cargo::rerun-if-changed=theorems/kept.theorem",
    )?;
    Ok(())
}

#[given("a crate without a theorems directory")]
fn given_a_crate_without_a_theorems_directory() {}

#[then("creating theorems later reruns the build script without manual seeding")]
fn then_creating_theorems_later_reruns_the_build_script_without_manual_seeding()
-> Result<(), String> {
    let fixture = FixtureCrate::new()?;

    let first_build = fixture.build()?;
    if !build_script_ran(&first_build) {
        return Err(format!(
            "first build should run the build script, got:\n{first_build}"
        ));
    }
    log_contains(&first_build, "cargo::rerun-if-changed=theorems")?;

    pause_for_timestamp_tick();
    fixture.write("theorems/first.theorem", TRIVIAL_THEOREM)?;

    let second_build = fixture.build()?;
    if !build_script_ran(&second_build) {
        return Err(format!(
            "creating theorems after the first build should rerun the build script, got:\n{second_build}"
        ));
    }
    log_contains(
        &second_build,
        "cargo::rerun-if-changed=theorems/first.theorem",
    )?;
    Ok(())
}

#[scenario(
    path = "tests/features/build_discovery.feature",
    name = "Existing theorem files are discovered recursively"
)]
fn existing_theorem_files_are_discovered_recursively() {}

#[scenario(
    path = "tests/features/build_discovery.feature",
    name = "Non-theorem files do not participate in discovery"
)]
fn non_theorem_files_do_not_participate_in_discovery() {}

#[scenario(
    path = "tests/features/build_discovery.feature",
    name = "Missing theorem directory is handled without manual seeding"
)]
fn missing_theorem_directory_is_handled_without_manual_seeding() {}
