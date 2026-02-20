//! Post-deserialization structural validation for `Step`, `LetBinding`,
//! `MaybeBlock`, and `ActionCall` shapes.
//!
//! These checks enforce constraints that `serde` attributes cannot express,
//! such as "action name must be non-empty" and "maybe.do must contain at
//! least one step". The functions return `Result<(), String>` so the
//! caller in [`super::validate`] can attach theorem-level context when
//! constructing [`super::error::SchemaError`].

use super::types::{ActionCall, Step};

/// Validates that an action call's `action` field is non-empty after
/// trimming.
///
/// Returns `Ok(())` if valid, or `Err(reason)` with a human-readable
/// reason string.
///
/// # Examples
///
///     use theoremc::schema::ActionCall;
///     use indexmap::IndexMap;
///
///     let good = ActionCall {
///         action: "account.deposit".to_owned(),
///         args: IndexMap::new(),
///         as_binding: None,
///     };
///     // A well-formed action call passes validation.
pub(crate) fn validate_action_call(action_call: &ActionCall) -> Result<(), String> {
    if action_call.action.trim().is_empty() {
        return Err("action must be non-empty after trimming".to_owned());
    }
    Ok(())
}

/// Validates a list of steps, used for both top-level `Do` and nested
/// `maybe.do` sequences.
///
/// Each step is validated in order using [`validate_step`]. The `path`
/// parameter provides context for error messages (e.g., `"Do step"`).
pub(crate) fn validate_step_list(steps: &[Step], path: &str) -> Result<(), String> {
    for (i, step) in steps.iter().enumerate() {
        validate_step(step, path, i + 1)?;
    }
    Ok(())
}

/// Validates a single step's structural constraints.
///
/// For `call` and `must` steps, validates the inner `ActionCall`. For
/// `maybe` steps, validates that `because` is non-empty after trimming,
/// `do` contains at least one step, and recursively validates each
/// nested step.
///
/// The `path` parameter provides context for error messages (e.g.,
/// `"Do step"`). The `pos` parameter is the 1-based position within
/// the current step list.
fn validate_step(step: &Step, path: &str, pos: usize) -> Result<(), String> {
    match step {
        Step::Call(c) => {
            validate_action_call(&c.call).map_err(|reason| format!("{path} {pos}: {reason}"))?;
        }
        Step::Must(m) => {
            validate_action_call(&m.must).map_err(|reason| format!("{path} {pos}: {reason}"))?;
        }
        Step::Maybe(m) => validate_maybe_block(&m.maybe, path, pos)?,
    }
    Ok(())
}

/// Validates a `MaybeBlock`'s structural constraints: non-empty
/// `because`, non-empty `do`, and recursive step validation.
fn validate_maybe_block(
    maybe: &super::types::MaybeBlock,
    path: &str,
    pos: usize,
) -> Result<(), String> {
    if maybe.because.trim().is_empty() {
        return Err(format!(
            concat!(
                "{path} {pos}: maybe.because must be ",
                "non-empty after trimming"
            ),
            path = path,
            pos = pos
        ));
    }
    if maybe.do_steps.is_empty() {
        return Err(format!(
            concat!("{path} {pos}: maybe.do must contain ", "at least one step"),
            path = path,
            pos = pos
        ));
    }
    let nested_path = format!("{path} {pos}: maybe.do step");
    validate_step_list(&maybe.do_steps, &nested_path)
}

#[cfg(test)]
mod tests {
    //! Unit tests for step and action call structural validation.
    use super::*;
    use crate::schema::types::{ActionCall, MaybeBlock, Step, StepCall, StepMaybe, StepMust};
    use indexmap::IndexMap;
    use rstest::{fixture, rstest};

    /// Fixture: a valid `ActionCall` with a non-empty dotted action name.
    #[fixture]
    fn valid_action() -> ActionCall {
        ActionCall {
            action: "a.b".to_owned(),
            args: IndexMap::new(),
            as_binding: None,
        }
    }

    /// Fixture: a valid `Step::Call` wrapping the default valid action.
    #[fixture]
    fn valid_call(valid_action: ActionCall) -> Step {
        Step::Call(StepCall { call: valid_action })
    }

    /// Fixture: a valid `Step::Must` wrapping the default valid action.
    #[fixture]
    fn valid_must(valid_action: ActionCall) -> Step {
        Step::Must(StepMust { must: valid_action })
    }

    /// Builder: an `ActionCall` with a custom action name.
    fn action(name: &str) -> ActionCall {
        ActionCall {
            action: name.to_owned(),
            args: IndexMap::new(),
            as_binding: None,
        }
    }

    /// Builder: a `Step::Call` with a custom action name.
    fn call_step(name: &str) -> Step {
        Step::Call(StepCall { call: action(name) })
    }

    /// Builder: a `Step::Must` with a custom action name.
    fn must_step(name: &str) -> Step {
        Step::Must(StepMust { must: action(name) })
    }

    /// Builder: a `Step::Maybe` with custom because and steps.
    fn maybe_step(because: &str, steps: Vec<Step>) -> Step {
        Step::Maybe(StepMaybe {
            maybe: MaybeBlock {
                because: because.to_owned(),
                do_steps: steps,
            },
        })
    }

    // ── ActionCall validation ─────────────────────────────────────

    #[rstest]
    #[case::non_empty("account.deposit")]
    #[case::dotted("hnsw.attach_node")]
    #[case::single_segment("deposit")]
    fn action_call_with_valid_action_passes(#[case] name: &str) {
        let ac = action(name);
        assert!(validate_action_call(&ac).is_ok());
    }

    #[rstest]
    #[case::empty("")]
    #[case::whitespace_only("   ")]
    #[case::tab_only("\t")]
    fn action_call_with_blank_action_fails(#[case] name: &str) {
        let ac = action(name);
        let err = validate_action_call(&ac).expect_err("should fail");
        assert!(
            err.contains("action must be non-empty"),
            "expected 'action must be non-empty', got: {err}"
        );
    }

    // ── Step list validation ──────────────────────────────────────

    #[rstest]
    fn valid_call_step_passes(valid_call: Step) {
        let steps = vec![valid_call];
        assert!(validate_step_list(&steps, "Do step").is_ok());
    }

    #[rstest]
    fn valid_must_step_passes(valid_must: Step) {
        let steps = vec![valid_must];
        assert!(validate_step_list(&steps, "Do step").is_ok());
    }

    #[rstest]
    fn valid_maybe_step_passes(valid_call: Step) {
        let steps = vec![maybe_step("optional branch", vec![valid_call])];
        assert!(validate_step_list(&steps, "Do step").is_ok());
    }

    #[rstest]
    #[case::call_empty(call_step(""))]
    #[case::call_whitespace(call_step("  "))]
    #[case::must_empty(must_step(""))]
    #[case::must_whitespace(must_step("  "))]
    fn step_with_blank_action_fails(#[case] step: Step) {
        let steps = vec![step];
        let err = validate_step_list(&steps, "Do step").expect_err("should fail");
        assert!(
            err.contains("Do step 1: action must be non-empty"),
            "got: {err}"
        );
    }

    #[rstest]
    #[case("")]
    #[case("   ")]
    fn maybe_step_with_invalid_because_fails(#[case] because: &str) {
        let steps = vec![maybe_step(because, vec![call_step("a.b")])];
        let err = validate_step_list(&steps, "Do step").expect_err("should fail");
        assert!(
            err.contains("maybe.because must be non-empty"),
            "got: {err}"
        );
    }

    #[test]
    fn maybe_step_with_empty_do_fails() {
        let steps = vec![maybe_step("reason", vec![])];
        let err = validate_step_list(&steps, "Do step").expect_err("should fail");
        assert!(
            err.contains("maybe.do must contain at least one step"),
            "got: {err}"
        );
    }

    #[rstest]
    #[case::blank_because("", vec![call_step("a.b")], "maybe.do step 1: maybe.because must be non-empty")]
    #[case::empty_do("inner reason", vec![], "maybe.do step 1: maybe.do must contain at least one step")]
    fn nested_maybe_validation_errors(
        #[case] inner_because: &str,
        #[case] inner_do: Vec<Step>,
        #[case] expected_error: &str,
    ) {
        let inner = maybe_step(inner_because, inner_do);
        let outer = maybe_step("outer reason", vec![inner]);
        let steps = vec![outer];
        let err = validate_step_list(&steps, "Do step").expect_err("should fail");
        assert!(err.contains(expected_error), "got: {err}");
    }

    #[rstest]
    fn second_step_error_reports_correct_position(valid_call: Step) {
        let steps = vec![valid_call, call_step("")];
        let err = validate_step_list(&steps, "Do step").expect_err("should fail");
        assert!(
            err.contains("Do step 2: action must be non-empty"),
            "got: {err}"
        );
    }
}
