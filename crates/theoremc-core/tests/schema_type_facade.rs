//! Integration guards for public schema type paths and the raw adapter's
//! spelling contracts.
//!
//! The domain types under `schema::types` carry no serde derives (ADR-003), so
//! defaults and unknown-field rejection are asserted through the public loader
//! that owns those rules rather than by deserializing the domain types.

use indexmap::IndexMap;
use rstest::{fixture, rstest};
use theoremc_core::canonical_action_name::CanonicalActionName;
use theoremc_core::schema;

#[test]
fn schema_facade_exposes_all_moved_type_shapes() {
    let action_call = schema::ActionCall {
        action: CanonicalActionName::new("counter.increment")
            .expect("the fixture action name should be canonical"),
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

    assert_eq!(
        evidence.has_any_backend(),
        case.expected_any_backend,
        "the predicate should report whether any backend is configured"
    );
}

/// A signature and Kani block that omit every optional key load with the
/// documented defaults, so the moved shapes keep their observable behaviour.
#[test]
fn moved_types_keep_their_documented_defaults() {
    let yaml = concat!(
        "Theorem: Defaults\n",
        "About: Omits optional keys\n",
        "Actions:\n",
        "  account.deposit:\n",
        "    params: {}\n",
        "Let:\n",
        "  updated:\n",
        "    call:\n",
        "      action: account.deposit\n",
        "      args: {}\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: always reachable\n",
    );

    let docs = schema::load_theorem_docs(yaml).expect("the defaults fixture should load");
    let document = docs.first().expect("the fixture should load one document");
    let signature = document
        .actions
        .get("account.deposit")
        .expect("the declared signature should be present");
    let kani = document
        .evidence
        .kani
        .as_ref()
        .expect("the Kani evidence should be present");

    assert_eq!(signature.returns, "()", "an omitted return type is unit");
    assert!(
        signature.params.is_empty(),
        "an empty params map should stay empty"
    );
    assert!(!kani.allow_vacuous, "vacuity should default to disallowed");
    assert_eq!(
        kani.vacuity_because, None,
        "an omitted vacuity reason should stay absent"
    );
}

/// Unknown keys inside the raw mirrors are rejected rather than ignored.
#[test]
fn moved_types_reject_unknown_raw_fields() -> Result<(), String> {
    let signature_error = load_fixture_error(concat!(
        "Theorem: UnknownSignatureKey\n",
        "About: Declares an unknown signature key\n",
        "Actions:\n",
        "  account.deposit:\n",
        "    unexpected: true\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: always reachable\n",
    ))?;
    if !signature_error.contains("unknown field") {
        return Err(format!(
            "the signature fixture should be rejected for its unknown key, got: {signature_error}"
        ));
    }

    let evidence_error = load_fixture_error(concat!(
        "Theorem: UnknownEvidenceKey\n",
        "About: Declares an unknown evidence key\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "    unexpected: true\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: always reachable\n",
    ))?;
    if !evidence_error.contains("unknown field") {
        return Err(format!(
            "the evidence fixture should be rejected for its unknown key, got: {evidence_error}"
        ));
    }
    Ok(())
}

/// Loads a fixture that must fail and returns the rendered error.
fn load_fixture_error(yaml: &str) -> Result<String, String> {
    match schema::load_theorem_docs(yaml) {
        Ok(_) => Err("the fixture should have been rejected".to_owned()),
        Err(error) => Ok(error.to_string()),
    }
}
