//! Shared helpers for integration and macro tests.

#[path = "../../../tests/common/mod.rs"]
mod integration;

pub use integration::{
    BARRIER_BUILD_SCRIPT, BARRIER_READY_FILE, BARRIER_RELEASE_FILE, BUILD_DISCOVERY_SOURCE,
    BUILD_SCRIPT_SOURCE, BUILD_SUITE_SOURCE, BuildLog, ExpectedFragment, FIXTURES_DIR,
    FixtureCrate, FixtureName, RunningBuild, TRIVIAL_THEOREM, assert_diagnostic_failure,
    assert_fixture_error_contains, assert_fixture_fails, assert_fixture_loads, await_path,
    build_support_fixture_manifest, fixture_error_message, fixture_source_id, hermetic_cargo,
    install_barrier, load_fixture, load_fixture_docs, load_fixture_text, precompile_barrier,
    toml_section,
};
pub use theoremc_core::schema::StrNewtype;
