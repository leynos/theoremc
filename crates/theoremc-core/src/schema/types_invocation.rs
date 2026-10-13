//! Decoded invocation shapes shared by `Let` bindings and `Do` steps.
//!
//! These models represent actions after raw decoding; `raw_action` owns
//! decoding and conversion.

use crate::schema::arg_value::ArgValue;
use indexmap::IndexMap;

// ── Let bindings ────────────────────────────────────────────────────

/// A named value binding computed before `Do` steps execute.
///
/// Only `call` and `must` forms are allowed in `Let` bindings. The
/// `maybe` form is disallowed because conditional existence of
/// bindings creates scoping complexity.
#[derive(Debug, Clone, PartialEq)]
pub enum LetBinding {
    /// Invoke an action and bind the result.
    Call(LetCall),
    /// Invoke an action, prove it cannot fail, and bind the unwrapped
    /// success value.
    Must(LetMust),
}

/// Wrapper for a `call` variant in a `Let` binding.
#[derive(Debug, Clone, PartialEq)]
pub struct LetCall {
    /// The action call to execute.
    pub call: ActionCall,
}

/// Wrapper for a `must` variant in a `Let` binding.
#[derive(Debug, Clone, PartialEq)]
pub struct LetMust {
    /// The action call to execute and prove infallible.
    pub must: ActionCall,
}

// ── Steps ───────────────────────────────────────────────────────────

/// A single step in a theorem's `Do` sequence.
///
/// Each step is exactly one of `call` (invoke), `must` (invoke and
/// prove infallible), or `maybe` (symbolic branching).
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct StepCall {
    /// The action call to execute.
    pub call: ActionCall,
}

/// Wrapper for a `must` variant in a `Do` step.
#[derive(Debug, Clone, PartialEq)]
pub struct StepMust {
    /// The action call to execute and prove infallible.
    pub must: ActionCall,
}

/// Wrapper for a `maybe` variant in a `Do` step.
#[derive(Debug, Clone, PartialEq)]
pub struct StepMaybe {
    /// The maybe block with a reason and nested steps.
    pub maybe: MaybeBlock,
}

// ── Maybe block ─────────────────────────────────────────────────────

/// A symbolic branching block within a `Do` sequence.
///
/// The model checker explores both the branch where the nested steps
/// execute and the branch where they do not.
#[derive(Debug, Clone, PartialEq)]
pub struct MaybeBlock {
    /// Human-readable explanation of why this branch exists.
    pub because: String,
    /// The nested steps to execute in the "taken" branch.
    pub do_steps: Vec<Step>,
}

// ── Action call ─────────────────────────────────────────────────────

/// An invocation of a theorem action with semantically decoded
/// arguments and optional result binding.
///
/// Arguments are decoded from raw YAML values into [`ArgValue`]
/// variants during the raw-to-public conversion step. Plain YAML
/// strings are always string literals; variable references require
/// the explicit `{ ref: <name> }` wrapper (`TFS-5` section 5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct ActionCall {
    /// Dot-separated action name (e.g., `hnsw.attach_node`).
    pub action: String,
    /// Semantically decoded arguments, keyed by parameter name.
    pub args: IndexMap<String, ArgValue>,
    /// Optional binding name for the action's return value.
    pub as_binding: Option<String>,
}
