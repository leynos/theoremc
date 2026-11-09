//! Post-deserialization structural validation for step lists, `Step`,
//! `LetBinding`, `MaybeBlock`, and `ActionCall` shapes.
//!
//! These checks enforce constraints that `serde` attributes cannot express,
//! such as "maybe.do must contain at least one step". `validate_step_list`
//! walks a top-level `Do` list or a nested `maybe.do` list in order, so a
//! failure names the list and the position that produced it. Canonical
//! action-name validation happens during raw-to-domain conversion, before
//! [`ActionCall`](super::types::ActionCall) values are constructed. The
//! functions return `Result<(), String>` so the caller in [`super::validate`]
//! can attach theorem-level context when constructing
//! [`super::error::SchemaError`].

use std::fmt;

use super::types::Step;

/// Diagnostic breadcrumb identifying the step list being validated.
///
/// The root names the list (for example `Do step`); each nested `maybe.do`
/// list extends its parent with the position that led into it, so an error
/// reads as `Do step 2: maybe.do step 1`. Rendering is deferred until an
/// error is actually raised.
#[derive(Clone)]
struct StepPath(String);

impl StepPath {
    /// Root breadcrumb for a named step list.
    fn root(label: &str) -> Self {
        Self(label.to_owned())
    }

    /// Breadcrumb for the `maybe.do` list of the step at `position`.
    fn nested(&self, position: usize) -> Self {
        Self(format!("{self} {position}: maybe.do step"))
    }
}

impl fmt::Display for StepPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validates a list of steps, used for both top-level `Do` and nested
/// `maybe.do` sequences.
///
/// `label` names the list in diagnostics (for example `Do step`). Each step is
/// validated in order using [`validate_step`].
pub(crate) fn validate_step_list(steps: &[Step], label: &str) -> Result<(), String> {
    validate_steps_at(steps, &StepPath::root(label))
}

/// Validates a step list under an existing diagnostic breadcrumb.
fn validate_steps_at(steps: &[Step], path: &StepPath) -> Result<(), String> {
    for (i, step) in steps.iter().enumerate() {
        validate_step(step, path, i + 1)?;
    }
    Ok(())
}

/// Validates a single step's structural constraints.
///
/// `call` and `must` steps carry no structural constraint of their own: the
/// action name they reference was already validated during raw-to-domain
/// conversion, and their argument values were decoded at the same boundary.
/// A `maybe` step validates that `because` is non-empty after trimming, that
/// `do` contains at least one step, and recursively validates each nested
/// step.
///
/// `pos` is the 1-based position within the current step list.
fn validate_step(step: &Step, path: &StepPath, pos: usize) -> Result<(), String> {
    match step {
        Step::Call(_) | Step::Must(_) => {}
        Step::Maybe(m) => validate_maybe_block(&m.maybe, path, pos)?,
    }
    Ok(())
}

/// Validates a `MaybeBlock`'s structural constraints: non-empty
/// `because`, non-empty `do`, and recursive step validation.
fn validate_maybe_block(
    maybe: &super::types::MaybeBlock,
    path: &StepPath,
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
    validate_steps_at(&maybe.do_steps, &path.nested(pos))
}

#[cfg(test)]
mod tests {
    //! Unit tests for step and action call structural validation.
    use super::*;
    use crate::canonical_action_name::CanonicalActionName;
    use crate::schema::types::{ActionCall, MaybeBlock, Step, StepCall, StepMaybe, StepMust};
    use indexmap::IndexMap;
    use rstest::{fixture, rstest};

    /// Fixture: a valid `ActionCall` with a non-empty dotted action name.
    #[fixture]
    fn valid_action() -> ActionCall {
        action(&canonical("a.b"))
    }

    /// Parses a test action name, failing loudly when it is not canonical.
    fn canonical(name: &str) -> CanonicalActionName {
        CanonicalActionName::new(name).expect("test action name must be canonical")
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
    fn action(name: &CanonicalActionName) -> ActionCall {
        ActionCall {
            action: name.clone(),
            args: IndexMap::new(),
            as_binding: None,
        }
    }

    /// Builder: a `Step::Call` with a custom action name.
    fn call_step(name: &CanonicalActionName) -> Step {
        Step::Call(StepCall { call: action(name) })
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

    // ── Step list validation ──────────────────────────────────────

    #[rstest]
    fn valid_steps_pass(valid_call: Step, valid_must: Step) {
        for step in [valid_call, valid_must] {
            assert!(validate_step_list(&[step], "Do step").is_ok());
        }
    }

    #[rstest]
    fn valid_maybe_step_passes(valid_call: Step) {
        let steps = vec![maybe_step("optional branch", vec![valid_call])];
        assert!(validate_step_list(&steps, "Do step").is_ok());
    }

    #[rstest]
    #[case("")]
    #[case("   ")]
    fn maybe_step_with_invalid_because_fails(#[case] because: &str) {
        let steps = vec![maybe_step(because, vec![call_step(&canonical("a.b"))])];
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
    #[case::blank_because("", vec![call_step(&canonical("a.b"))], "maybe.do step 1: maybe.because must be non-empty")]
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
        let steps = vec![
            valid_call,
            maybe_step("", vec![call_step(&canonical("a.b"))]),
        ];
        let err = validate_step_list(&steps, "Do step").expect_err("should fail");
        assert!(
            err.contains("Do step 2: maybe.because must be non-empty"),
            "got: {err}"
        );
    }
}
