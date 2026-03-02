//! Raw serde-compatible action, step, and binding types.
//!
//! These types mirror the public `ActionCall`, `LetBinding`, `Step`,
//! and related types but use [`TheoremValue`] for argument values
//! (the raw YAML representation). After deserialization, the
//! conversion functions in this module decode each
//! [`TheoremValue`] into an [`ArgValue`] via
//! [`decode_arg_value`](super::arg_value::decode_arg_value).

use indexmap::IndexMap;
use serde::Deserialize;

use super::arg_value::decode_arg_value;
use super::types::{
    ActionCall, LetBinding, LetCall, LetMust, MaybeBlock, Step, StepCall, StepMaybe, StepMust,
};
use super::value::TheoremValue;

// ── Raw action call ─────────────────────────────────────────────────

/// A raw action call as deserialized from YAML, before argument
/// decoding.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawActionCall {
    /// Dot-separated action name.
    pub(crate) action: String,
    /// Raw YAML argument values, not yet decoded.
    pub(crate) args: IndexMap<String, TheoremValue>,
    /// Optional binding name for the action's return value.
    #[serde(rename = "as", default)]
    pub(crate) as_binding: Option<String>,
}

// ── Raw Let bindings ────────────────────────────────────────────────

/// Raw `LetBinding` as deserialized from YAML.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawLetBinding {
    /// Invoke an action and bind the result.
    Call(RawLetCall),
    /// Invoke an action, prove it cannot fail, and bind the unwrapped
    /// success value.
    Must(RawLetMust),
}

/// Raw wrapper for a `call` variant in a `Let` binding.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawLetCall {
    pub(crate) call: RawActionCall,
}

/// Raw wrapper for a `must` variant in a `Let` binding.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawLetMust {
    pub(crate) must: RawActionCall,
}

// ── Raw Steps ───────────────────────────────────────────────────────

/// Raw `Step` as deserialized from YAML.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawStep {
    /// Invoke an action.
    Call(RawStepCall),
    /// Invoke an action and prove it cannot fail.
    Must(RawStepMust),
    /// Symbolic branching.
    Maybe(RawStepMaybe),
}

/// Raw wrapper for a `call` variant in a `Do` step.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawStepCall {
    pub(crate) call: RawActionCall,
}

/// Raw wrapper for a `must` variant in a `Do` step.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawStepMust {
    pub(crate) must: RawActionCall,
}

/// Raw wrapper for a `maybe` variant in a `Do` step.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawStepMaybe {
    pub(crate) maybe: RawMaybeBlock,
}

/// Raw symbolic branching block with nested raw steps.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawMaybeBlock {
    /// Human-readable explanation of why this branch exists.
    pub(crate) because: String,
    /// The nested raw steps.
    #[serde(rename = "do")]
    pub(crate) do_steps: Vec<RawStep>,
}

// ── Conversion functions ────────────────────────────────────────────

/// Converts a [`RawActionCall`] into a public [`ActionCall`] by
/// decoding each argument value.
pub(crate) fn convert_action_call(raw: &RawActionCall) -> Result<ActionCall, String> {
    let mut args = IndexMap::with_capacity(raw.args.len());
    for (key, value) in &raw.args {
        let decoded = decode_arg_value(key, value.clone())?;
        args.insert(key.clone(), decoded);
    }
    Ok(ActionCall {
        action: raw.action.clone(),
        args,
        as_binding: raw.as_binding.clone(),
    })
}

/// Converts a [`RawLetBinding`] into a public [`LetBinding`].
pub(crate) fn convert_let_binding(raw: &RawLetBinding) -> Result<LetBinding, String> {
    match raw {
        RawLetBinding::Call(c) => {
            let call = convert_action_call(&c.call)?;
            Ok(LetBinding::Call(LetCall { call }))
        }
        RawLetBinding::Must(m) => {
            let must = convert_action_call(&m.must)?;
            Ok(LetBinding::Must(LetMust { must }))
        }
    }
}

/// Converts a [`RawStep`] into a public [`Step`], recursively
/// converting nested maybe blocks.
pub(crate) fn convert_step(raw: &RawStep) -> Result<Step, String> {
    match raw {
        RawStep::Call(c) => {
            let call = convert_action_call(&c.call)?;
            Ok(Step::Call(StepCall { call }))
        }
        RawStep::Must(m) => {
            let must = convert_action_call(&m.must)?;
            Ok(Step::Must(StepMust { must }))
        }
        RawStep::Maybe(m) => {
            let maybe = convert_maybe_block(&m.maybe)?;
            Ok(Step::Maybe(StepMaybe { maybe }))
        }
    }
}

/// Converts a [`RawMaybeBlock`] into a public [`MaybeBlock`],
/// recursively converting nested steps.
fn convert_maybe_block(raw: &RawMaybeBlock) -> Result<MaybeBlock, String> {
    let mut do_steps = Vec::with_capacity(raw.do_steps.len());
    for step in &raw.do_steps {
        do_steps.push(convert_step(step)?);
    }
    Ok(MaybeBlock {
        because: raw.because.clone(),
        do_steps,
    })
}
