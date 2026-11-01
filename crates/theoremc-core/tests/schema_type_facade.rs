//! Integration guards for public schema type paths and serde contracts.

use indexmap::IndexMap;
use rstest::{fixture, rstest};
use theoremc_core::schema;

#[test]
fn schema_facade_exposes_all_moved_type_shapes() {
    let action_call = schema::ActionCall {
        action: "counter.increment".to_owned(),
        args: IndexMap::new(),
        as_binding: None,
    };
    let action_signature = schema::ActionSignature {
        params: IndexMap::new(),
        returns: "()".to_owned(),
    };
    let kani_evidence = schema::KaniEvidence {
        unwind: 1,
        expect: schema::KaniExpectation::Success,
        allow_vacuous: false,
        vacuity_because: None,
    };
    let evidence = schema::Evidence {
        kani: Some(kani_evidence.clone()),
        verus: None,
        stateright: None,
    };
    let maybe_block = schema::MaybeBlock {
        because: "an optional execution path".to_owned(),
        do_steps: Vec::new(),
    };

    let _public_shapes = (
        action_call.clone(),
        action_signature,
        evidence,
        kani_evidence,
        schema::LetBinding::Call(schema::LetCall {
            call: action_call.clone(),
        }),
        schema::LetBinding::Must(schema::LetMust {
            must: action_call.clone(),
        }),
        schema::Step::Call(schema::StepCall {
            call: action_call.clone(),
        }),
        schema::Step::Must(schema::StepMust { must: action_call }),
        schema::Step::Maybe(schema::StepMaybe { maybe: maybe_block }),
    );
}

#[fixture]
fn kani_evidence() -> schema::KaniEvidence {
    schema::KaniEvidence {
        unwind: 1,
        expect: schema::KaniExpectation::Success,
        allow_vacuous: false,
        vacuity_because: None,
    }
}

#[derive(Debug, Clone, Copy)]
enum Backend {
    Kani,
    Verus,
    Stateright,
}

#[derive(Debug, Clone, Copy)]
struct BackendCase {
    backend: Option<Backend>,
    expected_any_backend: bool,
}

#[rstest]
#[case::no_backend(BackendCase { backend: None, expected_any_backend: false })]
#[case::kani_only(BackendCase { backend: Some(Backend::Kani), expected_any_backend: true })]
#[case::verus_only(BackendCase { backend: Some(Backend::Verus), expected_any_backend: true })]
#[case::stateright_only(BackendCase { backend: Some(Backend::Stateright), expected_any_backend: true })]
fn evidence_reports_each_supported_backend(
    #[case] case: BackendCase,
    kani_evidence: schema::KaniEvidence,
) {
    let evidence = schema::Evidence {
        kani: matches!(case.backend, Some(Backend::Kani)).then_some(kani_evidence),
        verus: matches!(case.backend, Some(Backend::Verus))
            .then_some(schema::TheoremValue::Bool(true)),
        stateright: matches!(case.backend, Some(Backend::Stateright))
            .then_some(schema::TheoremValue::Bool(true)),
    };

    assert_eq!(evidence.has_any_backend(), case.expected_any_backend);
}

#[test]
fn moved_types_keep_their_direct_serde_defaults() {
    let signature: schema::ActionSignature =
        serde_saphyr::from_str("{}").expect("empty signature should use its defaults");
    assert!(signature.params.is_empty());
    assert_eq!(signature.returns, "()");

    let kani: schema::KaniEvidence = serde_saphyr::from_str("unwind: 1\nexpect: SUCCESS\n")
        .expect("Kani evidence should use its optional field defaults");
    assert!(!kani.allow_vacuous);
    assert_eq!(kani.vacuity_because, None);
}

#[test]
fn moved_types_reject_unknown_serde_fields() {
    assert!(serde_saphyr::from_str::<schema::ActionSignature>("unexpected: true\n").is_err());
    assert!(
        serde_saphyr::from_str::<schema::KaniEvidence>(
            "unwind: 1\nexpect: SUCCESS\nunexpected: true\n"
        )
        .is_err()
    );
}
