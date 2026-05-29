//! Shared fixtures and builders for collision-detection unit tests.

use super::{LetBinding, Step, TheoremDoc};
use crate::schema::{
    ActionCall, Assertion, Evidence, KaniEvidence, KaniExpectation, LetCall, StepCall, TheoremName,
    WitnessCheck,
};
use indexmap::IndexMap;

/// Shared boilerplate fields required by every `TheoremDoc` in tests.
#[derive(Clone)]
pub(super) struct DocBoilerplate {
    pub(super) evidence: Evidence,
    pub(super) assertions: Vec<Assertion>,
    pub(super) witnesses: Vec<WitnessCheck>,
}

/// Minimal valid boilerplate for constructing test documents.
#[rstest::fixture]
pub(super) fn boilerplate() -> DocBoilerplate {
    DocBoilerplate {
        evidence: Evidence {
            kani: Some(KaniEvidence {
                unwind: 1,
                expect: KaniExpectation::Success,
                allow_vacuous: false,
                vacuity_because: None,
            }),
            verus: None,
            stateright: None,
        },
        assertions: vec![Assertion {
            assert_expr: "true".to_owned(),
            because: "trivial".to_owned(),
        }],
        witnesses: vec![WitnessCheck {
            cover: "true".to_owned(),
            because: "reachable".to_owned(),
        }],
    }
}

/// Builds an `ActionCall` with the given action name and empty args.
pub(super) fn action_call(name: &str) -> ActionCall {
    ActionCall {
        action: name.to_owned(),
        args: IndexMap::new(),
        as_binding: None,
    }
}

/// Builds a minimal valid `TheoremDoc` with custom bindings and steps.
pub(super) fn theorem_doc(
    name: &str,
    let_bindings: IndexMap<String, LetBinding>,
    do_steps: Vec<Step>,
    bp: &DocBoilerplate,
) -> TheoremDoc {
    TheoremDoc {
        schema: None,
        theorem: TheoremName::new(name.to_owned()).expect("valid theorem name"),
        about: "test theorem".to_owned(),
        tags: Vec::new(),
        given: Vec::new(),
        forall: IndexMap::new(),
        actions: IndexMap::new(),
        assume: Vec::new(),
        witness: bp.witnesses.clone(),
        let_bindings,
        do_steps,
        prove: bp.assertions.clone(),
        evidence: bp.evidence.clone(),
    }
}

/// Builds a `TheoremDoc` with the given action names in `Let` bindings.
pub(super) fn doc_with_let_actions(
    name: &str,
    actions: &[&str],
    bp: &DocBoilerplate,
) -> TheoremDoc {
    let mut let_bindings = IndexMap::new();
    for (i, action) in actions.iter().enumerate() {
        let_bindings.insert(
            format!("binding_{i}"),
            LetBinding::Call(LetCall {
                call: action_call(action),
            }),
        );
    }
    theorem_doc(name, let_bindings, Vec::new(), bp)
}

/// Builds a `TheoremDoc` with the given action names in `Do` steps.
pub(super) fn doc_with_do_actions(name: &str, actions: &[&str], bp: &DocBoilerplate) -> TheoremDoc {
    let steps: Vec<Step> = actions
        .iter()
        .map(|a| {
            Step::Call(StepCall {
                call: action_call(a),
            })
        })
        .collect();
    theorem_doc(name, IndexMap::new(), steps, bp)
}
