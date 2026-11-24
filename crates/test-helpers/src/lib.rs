//! Shared helpers for integration and macro tests.

#[path = "../../../tests/common/mod.rs"]
mod integration;

pub use integration::{
    BUILD_SCRIPT_SOURCE, BuildLog, CommandFailure, CommandOutput, DEFAULT_FIXTURE_COMMAND_TIMEOUT,
    DEFAULT_LOCK_WAIT, ExpectedFragment, FIXTURE_TIMEOUT_ENV, FIXTURES_DIR, FixtureCommandError,
    FixtureCrate, FixtureLock, FixtureName, FixtureOutcome, TRIVIAL_THEOREM,
    assert_diagnostic_failure, assert_fixture_error_contains, assert_fixture_fails,
    assert_fixture_loads, build_support_dependency, build_support_dependency_for,
    build_support_fixture_manifest, checkout_root, fixture_command_timeout, fixture_error_message,
    fixture_source_id, hermetic_cargo, load_fixture, load_fixture_docs, load_fixture_text,
    run_fixture_command, run_fixture_command_within, run_within_deadline, shared_target_dir,
    shared_target_sources_dir, toml_path, toml_section,
};
pub use theoremc_core::schema::StrNewtype;
