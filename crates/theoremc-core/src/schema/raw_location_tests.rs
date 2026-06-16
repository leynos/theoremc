//! Unit tests for typed validation reason source-location lookup.

use super::RawTheoremDoc;
use crate::schema::validation_reason::{IndexedValidationField, ValidationReasonKind};
use rstest::rstest;

const LOCATION_FIXTURE: &str = "\
Theorem: T
About: ''
Assume:
  - expr: ''
    because: ''
Prove:
  - assert: ''
    because: ''
Witness:
  - cover: ''
    because: ''
Evidence:
  kani:
    unwind: 0
    expect: SUCCESS
    allow_vacuous: true
    vacuity_because: ''
";

fn raw_doc() -> RawTheoremDoc {
    let docs: Vec<RawTheoremDoc> =
        serde_saphyr::from_multiple(LOCATION_FIXTURE).expect("fixture should deserialize");
    docs.into_iter()
        .next()
        .expect("fixture should contain one theorem document")
}

#[rstest]
#[case::about(ValidationReasonKind::AboutEmpty, 2)]
#[case::assume_expr(
    ValidationReasonKind::Assume {
        index: 0,
        field: IndexedValidationField::Value,
    },
    4
)]
#[case::assume_because(
    ValidationReasonKind::Assume {
        index: 0,
        field: IndexedValidationField::Because,
    },
    5
)]
#[case::prove_assert(
    ValidationReasonKind::Prove {
        index: 0,
        field: IndexedValidationField::Value,
    },
    7
)]
#[case::prove_because(
    ValidationReasonKind::Prove {
        index: 0,
        field: IndexedValidationField::Because,
    },
    8
)]
#[case::witness_cover(
    ValidationReasonKind::Witness {
        index: 0,
        field: IndexedValidationField::Value,
    },
    10
)]
#[case::witness_because(
    ValidationReasonKind::Witness {
        index: 0,
        field: IndexedValidationField::Because,
    },
    11
)]
#[case::kani_unwind(ValidationReasonKind::KaniUnwind, 14)]
#[case::kani_missing_vacuity_reason(ValidationReasonKind::KaniAllowVacuousRequired, 16)]
#[case::kani_blank_vacuity_reason(ValidationReasonKind::KaniVacuityBecauseNonEmpty, 17)]
#[case::kani_witness_required(ValidationReasonKind::KaniWitnessRequired, 16)]
fn validation_reason_kind_selects_location_without_rendered_message(
    #[case] reason: ValidationReasonKind,
    #[case] expected_line: u64,
) {
    let location = raw_doc().location_for_validation_reason(reason);

    assert_eq!(location.line(), expected_line);
}
