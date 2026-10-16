//! Backend evidence configuration data shapes for theorem documents.
//!
//! This module models the configured evidence. Unwind, vacuity, and witness
//! policy remains in `validate_evidence`.

use crate::schema::value::TheoremValue;
use serde::Deserialize;

// ── Evidence ────────────────────────────────────────────────────────

/// Backend evidence configuration for a theorem.
///
/// At least one backend must be specified. For v1, Kani is the primary
/// backend; `verus` and `stateright` are placeholders for future use.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Kani model-checking backend configuration.
    #[serde(default)]
    pub kani: Option<KaniEvidence>,
    /// Verus proof backend configuration (placeholder).
    #[serde(default)]
    pub verus: Option<TheoremValue>,
    /// Stateright model-checking backend configuration (placeholder).
    #[serde(default)]
    pub stateright: Option<TheoremValue>,
}

impl Evidence {
    /// Returns `true` if at least one backend is configured.
    #[must_use]
    pub const fn has_any_backend(&self) -> bool {
        self.kani.is_some() || self.verus.is_some() || self.stateright.is_some()
    }
}

// ── Kani evidence ───────────────────────────────────────────────────

/// Configuration for the Kani model-checking backend.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KaniEvidence {
    /// Loop unwinding bound (`#[kani::unwind(n)]`).
    pub unwind: u32,
    /// Expected verification outcome.
    pub expect: KaniExpectation,
    /// Whether vacuous success is permitted (default: `false`).
    #[serde(default)]
    pub allow_vacuous: bool,
    /// Justification required when `allow_vacuous` is `true`.
    #[serde(default)]
    pub vacuity_because: Option<String>,
}

/// Expected outcome of a Kani verification run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum KaniExpectation {
    /// The proof harness is expected to succeed.
    #[serde(rename = "SUCCESS")]
    Success,
    /// The proof harness is expected to find a counterexample.
    #[serde(rename = "FAILURE")]
    Failure,
    /// The proof harness is expected to be unreachable.
    #[serde(rename = "UNREACHABLE")]
    Unreachable,
    /// The verification outcome is undetermined.
    #[serde(rename = "UNDETERMINED")]
    Undetermined,
}
