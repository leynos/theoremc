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

#[derive(Debug)]
struct BackendCase {
    has_kani: bool,
    has_verus: bool,
    has_stateright: bool,
    expected: bool,
}

impl BackendCase {
    const NO_BACKEND: Self = Self {
        has_kani: false,
        has_verus: false,
        has_stateright: false,
        expected: false,
    };

    const KANI_ONLY: Self = Self {
        has_kani: true,
        has_verus: false,
        has_stateright: false,
        expected: true,
    };

    const VERUS_ONLY: Self = Self {
        has_kani: false,
        has_verus: true,
        has_stateright: false,
        expected: true,
    };

    const STATERIGHT_ONLY: Self = Self {
        has_kani: false,
        has_verus: false,
        has_stateright: true,
        expected: true,
    };
}

#[rstest]
#[case::no_backend(BackendCase::NO_BACKEND)]
#[case::kani_only(BackendCase::KANI_ONLY)]
#[case::verus_only(BackendCase::VERUS_ONLY)]
#[case::stateright_only(BackendCase::STATERIGHT_ONLY)]
fn evidence_reports_each_supported_backend(
    #[case] backends: BackendCase,
    kani_evidence: schema::KaniEvidence,
) {
    let evidence = schema::Evidence {
        kani: backends.has_kani.then_some(kani_evidence),
        verus: backends
            .has_verus
            .then_some(schema::TheoremValue::Bool(true)),
        stateright: backends
            .has_stateright
            .then_some(schema::TheoremValue::Bool(true)),
    };

    assert_eq!(evidence.has_any_backend(), backends.expected);
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
