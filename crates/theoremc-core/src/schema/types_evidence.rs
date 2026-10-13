//! Backend evidence configuration data shapes for theorem documents.
//!
//! This module models the configured evidence. Unwind, vacuity, and witness
//! policy remains in `validate_evidence`; YAML spelling, aliases, and defaults
//! belong to `raw_evidence`, which converts its raw mirrors into these domain
//! forms.

use crate::schema::value::TheoremValue;

// ── Evidence ────────────────────────────────────────────────────────

/// Backend evidence configuration for a theorem.
///
/// At least one backend must be specified. For v1, Kani is the primary
/// backend; `verus` and `stateright` are placeholders for future use.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    /// Kani model-checking backend configuration.
    pub kani: Option<KaniEvidence>,
    /// Verus proof backend configuration (placeholder).
    pub verus: Option<TheoremValue>,
    /// Stateright model-checking backend configuration (placeholder).
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
#[derive(Debug, Clone, PartialEq)]
pub struct KaniEvidence {
    /// Loop unwinding bound (`#[kani::unwind(n)]`).
    pub unwind: u32,
    /// Expected verification outcome.
    pub expect: KaniExpectation,
    /// Whether vacuous success is permitted (raw YAML defaults to `false`).
    pub allow_vacuous: bool,
    /// Justification required when `allow_vacuous` is `true`.
    pub vacuity_because: Option<String>,
}

/// Expected outcome of a Kani verification run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaniExpectation {
    /// The proof harness is expected to succeed.
    Success,
    /// The proof harness is expected to find a counterexample.
    Failure,
    /// The proof harness is expected to be unreachable.
    Unreachable,
    /// The verification outcome is undetermined.
    Undetermined,
}