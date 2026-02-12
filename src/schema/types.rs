//! Strongly-typed schema structs for `.theorem` documents.
//!
//! These types mirror the YAML schema defined in
//! `docs/theorem-file-specification.md` section 8. Deserialization uses
//! `serde(deny_unknown_fields)` on all struct types and supports both
//! `TitleCase` (canonical) and lowercase key aliases.

use indexmap::IndexMap;
use serde::Deserialize;

use super::newtypes::{ForallVar, TheoremName};
use super::value::TheoremValue;

// ── Top-level document ──────────────────────────────────────────────

/// A single theorem document parsed from a `.theorem` YAML file.
///
/// Each document describes one theorem with its assumptions, steps,
/// proof obligations, and evidence configuration. A `.theorem` file
/// may contain multiple documents separated by `---`.
///
/// # Examples
///
///     use theoremc::schema::load_theorem_docs;
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheoremDoc {
    /// Schema version for forwards compatibility.
    ///
    /// When omitted in the YAML source the field is `None`, indicating
    /// "unspecified — treat as current default".
    #[serde(rename = "Schema", alias = "schema", default)]
    pub schema: Option<u32>,

    /// Unique theorem name (must be a valid Rust identifier, not a
    /// reserved keyword). Validated at deserialization time.
    #[serde(rename = "Theorem", alias = "theorem")]
    pub theorem: TheoremName,

    /// Human-readable description of the theorem's intent.
    #[serde(rename = "About", alias = "about")]
    pub about: String,

    /// Metadata tags for filtering, ownership, and reporting.
    #[serde(rename = "Tags", alias = "tags", default)]
    pub tags: Vec<String>,

    /// Narrative context (no codegen impact).
    #[serde(rename = "Given", alias = "given", default)]
    pub given: Vec<String>,

    /// Symbolic quantified variables mapped to Rust types.
    #[serde(rename = "Forall", alias = "forall", default)]
    pub forall: IndexMap<ForallVar, String>,

    /// Constraints on symbolic inputs.
    #[serde(rename = "Assume", alias = "assume", default)]
    pub assume: Vec<Assumption>,

    /// Non-vacuity witnesses (required unless vacuity is explicitly
    /// allowed).
    #[serde(rename = "Witness", alias = "witness", default)]
    pub witness: Vec<WitnessCheck>,

    /// Named fixtures and derived constants.
    #[serde(rename = "Let", alias = "let", default)]
    pub let_bindings: IndexMap<String, LetBinding>,

    /// Ordered sequence of theorem steps.
    #[serde(rename = "Do", alias = "do", default)]
    pub do_steps: Vec<Step>,

    /// Proof obligations (must be non-empty).
    #[serde(rename = "Prove", alias = "prove")]
    pub prove: Vec<Assertion>,

    /// Backend evidence configuration.
    #[serde(rename = "Evidence", alias = "evidence")]
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

// ── Let bindings ────────────────────────────────────────────────────

/// A named value binding computed before `Do` steps execute.
///
/// Only `call` and `must` forms are allowed in `Let` bindings. The
/// `maybe` form is disallowed because conditional existence of
/// bindings creates scoping complexity.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum LetBinding {
    /// Invoke an action and bind the result.
    Call(LetCall),
    /// Invoke an action, prove it cannot fail, and bind the unwrapped
    /// success value.
    Must(LetMust),
}

/// Wrapper for a `call` variant in a `Let` binding.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LetCall {
    /// The action call to execute.
    pub call: ActionCall,
}

/// Wrapper for a `must` variant in a `Let` binding.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LetMust {
    /// The action call to execute and prove infallible.
    pub must: ActionCall,
}

// ── Steps ───────────────────────────────────────────────────────────

/// A single step in a theorem's `Do` sequence.
///
/// Each step is exactly one of `call` (invoke), `must` (invoke and
/// prove infallible), or `maybe` (symbolic branching).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Step {
    /// Invoke an action.
    Call(StepCall),
    /// Invoke an action and prove it cannot fail.
    Must(StepMust),
    /// Symbolic branching — both branches are explored by the model
    /// checker.
    Maybe(StepMaybe),
}

/// Wrapper for a `call` variant in a `Do` step.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepCall {
    /// The action call to execute.
    pub call: ActionCall,
}

/// Wrapper for a `must` variant in a `Do` step.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepMust {
    /// The action call to execute and prove infallible.
    pub must: ActionCall,
}

/// Wrapper for a `maybe` variant in a `Do` step.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepMaybe {
    /// The maybe block with a reason and nested steps.
    pub maybe: MaybeBlock,
}

// ── Maybe block ─────────────────────────────────────────────────────

/// A symbolic branching block within a `Do` sequence.
///
/// The model checker explores both the branch where the nested steps
/// execute and the branch where they do not.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaybeBlock {
    /// Human-readable explanation of why this branch exists.
    pub because: String,
    /// The nested steps to execute in the "taken" branch.
    #[serde(rename = "do")]
    pub do_steps: Vec<Step>,
}

// ── Action call ─────────────────────────────────────────────────────

/// An invocation of a theorem action with arguments and optional
/// result binding.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCall {
    /// Dot-separated action name (e.g., `hnsw.attach_node`).
    pub action: String,
    /// Arguments passed to the action, keyed by parameter name.
    pub args: IndexMap<String, TheoremValue>,
    /// Optional binding name for the action's return value.
    #[serde(rename = "as", default)]
    pub as_binding: Option<String>,
}

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
