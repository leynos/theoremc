//! Public domain types for `.theorem` documents.
//!
//! This module owns theorem documents and their conditions. Private child
//! modules group action signature, evidence, and decoded invocation shapes;
//! their public types are re-exported here for stable schema facade paths.
//! YAML aliases, defaults, source locations, and raw-to-domain conversion
//! belong to `raw` and `raw_action`.

use indexmap::IndexMap;
use serde::Deserialize;

use super::newtypes::{ForallVar, TheoremName};

#[path = "types_evidence.rs"]
mod evidence;
#[path = "types_invocation.rs"]
mod invocation;
#[path = "types_signature.rs"]
mod signature;

pub use evidence::{Evidence, KaniEvidence, KaniExpectation};
pub use invocation::{
    ActionCall, LetBinding, LetCall, LetMust, MaybeBlock, Step, StepCall, StepMaybe, StepMust,
};
pub use signature::ActionSignature;

// ── Top-level document ──────────────────────────────────────────────

/// A single theorem document parsed from a `.theorem` YAML file.
///
/// Each document describes one theorem with its assumptions, steps,
/// proof obligations, and evidence configuration. A `.theorem` file
/// may contain multiple documents separated by `---`.
///
/// # Examples
///
///     use theoremc_core::schema::load_theorem_docs;
///
///     let yaml = r#"
///     Theorem: MyTheorem
///     About: A simple example
///     Prove:
///       - assert: "x > 0"
///         because: "x is positive"
///     Evidence:
///       kani:
///         unwind: 10
///         expect: SUCCESS
///     Witness:
///       - cover: "x == 1"
///         because: "at least one positive value"
///     "#;
///     let docs = load_theorem_docs(yaml).unwrap();
///     assert_eq!(docs.len(), 1);
#[derive(Debug, Clone, PartialEq)]
pub struct TheoremDoc {
    /// Schema version for forwards compatibility.
    ///
    /// When omitted in the YAML source the field is `None`, indicating
    /// "unspecified — treat as current default".
    pub schema: Option<u32>,

    /// Unique theorem name (must be a valid Rust identifier, not a
    /// reserved keyword). Validated at deserialization time.
    pub theorem: TheoremName,

    /// Human-readable description of the theorem's intent.
    pub about: String,

    /// Metadata tags for filtering, ownership, and reporting.
    pub tags: Vec<String>,

    /// Narrative context (no codegen impact).
    pub given: Vec<String>,

    /// Symbolic quantified variables mapped to Rust types.
    pub forall: IndexMap<ForallVar, String>,

    /// Expected Rust signatures for referenced theorem actions.
    pub actions: IndexMap<String, ActionSignature>,

    /// Constraints on symbolic inputs.
    pub assume: Vec<Assumption>,

    /// Non-vacuity witnesses (required unless vacuity is explicitly
    /// allowed).
    pub witness: Vec<WitnessCheck>,

    /// Named fixtures and derived constants.
    pub let_bindings: IndexMap<String, LetBinding>,

    /// Ordered sequence of theorem steps.
    pub do_steps: Vec<Step>,

    /// Proof obligations (must be non-empty).
    pub prove: Vec<Assertion>,

    /// Backend evidence configuration.
    pub evidence: Evidence,
}

// ── Assumption ──────────────────────────────────────────────────────

/// A constraint on symbolic inputs.
///
/// Each assumption provides a Rust expression and a human-readable
/// explanation of why the constraint is necessary.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assumption {
    /// A Rust expression that must hold (parsed as `syn::Expr` in
    /// later validation stages).
    pub expr: String,
    /// Human-readable justification for this assumption.
    pub because: String,
}

// ── Assertion ───────────────────────────────────────────────────────

/// A proof obligation that the theorem must satisfy.
///
/// The `assert` field contains a Rust boolean expression; `because`
/// provides a human-readable explanation.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    /// A Rust boolean expression to assert.
    #[serde(rename = "assert")]
    pub assert_expr: String,
    /// Human-readable justification for this assertion.
    pub because: String,
}

// ── Witness ─────────────────────────────────────────────────────────

/// A non-vacuity witness that ensures the theorem exercises at least
/// one meaningful execution path.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessCheck {
    /// A Rust expression used as a coverage marker.
    pub cover: String,
    /// Human-readable justification for this witness.
    pub because: String,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
