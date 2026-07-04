//! Shared helpers for integration and macro tests.

use std::env;
use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, PoisonError};

#[path = "../../../tests/common/mod.rs"]
mod integration;

pub use integration::{
    BUILD_DISCOVERY_SOURCE, BUILD_SCRIPT_SOURCE, BUILD_SUITE_SOURCE, BuildLog, FixtureCrate,
    TRIVIAL_THEOREM, assert_diagnostic_failure, assert_fixture_error_contains,
    assert_fixture_fails, assert_fixture_loads, fixture_error_message, load_fixture,
    load_fixture_docs, load_fixture_text, toml_section,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Guard that restores a process environment variable when dropped.
///
/// Holding this guard also holds the shared environment mutex, serializing
/// tests that mutate process-global environment state.
#[must_use]
pub struct EnvGuard {
    variable: &'static str,
    previous: Option<OsString>,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: `EnvGuard` holds the global `ENV_LOCK` mutex, guaranteeing
        // exclusive access to process environment mutations performed by this
        // helper. No other thread or test can call `env::set_var` or
        // `env::remove_var` through this helper while the lock is held. Under
        // Rust 2024, concurrent environment mutation is UB; restoring
        // `variable` from `previous` here is sound because the mutex protects
        // these operations.
        unsafe {
            match self.previous.as_deref() {
                Some(value) => env::set_var(self.variable, value),
                None => env::remove_var(self.variable),
            }
        }
    }
}

/// Sets `CARGO_MANIFEST_DIR` for the lifetime of the returned guard.
///
/// Passing [`None`] removes the variable until the guard is dropped. Dropping
/// the guard restores the previous value, if any.
pub fn set_cargo_manifest_dir_for_test(value: Option<&str>) -> EnvGuard {
    set_env_var_for_test("CARGO_MANIFEST_DIR", value)
}

fn set_env_var_for_test(variable: &'static str, value: Option<&str>) -> EnvGuard {
    let guard = ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let previous = env::var_os(variable);
    // SAFETY: `EnvGuard` retains `ENV_LOCK`, so environment mutation through
    // this helper is serialized across tests that use it.
    unsafe {
        match value {
            Some(new_value) => env::set_var(variable, new_value),
            None => env::remove_var(variable),
        }
    }
    EnvGuard {
        variable,
        previous,
        _guard: guard,
    }
}
