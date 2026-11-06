//! Internal raw schema types with source-location capture.
//!
//! These types mirror the public schema shape but use `serde_saphyr::Spanned`
//! for selected fields so validation failures can be mapped back to line and
//! column coordinates deterministically.

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, de::Error};
use serde_saphyr::{Location, Spanned};

use super::newtypes::{ForallVar, TheoremName};
use super::raw_action::{RawLetBinding, RawStep};
use super::types::{Evidence, KaniEvidence, KaniExpectation, TheoremDoc};
use super::validation_reason::{IndexedValidationField, ValidationReasonKind};
use super::value::TheoremValue;

mod decode;

pub(crate) use decode::RawDocDecodeError;
use decode::{convert_let_bindings, convert_steps};

/// Raw theorem document with location-carrying fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTheoremDoc {
    #[serde(rename = "Schema", alias = "schema", default)]
    pub(crate) schema: Option<u32>,
    #[serde(rename = "Theorem", alias = "theorem")]
    pub(crate) theorem: Spanned<TheoremName>,
    #[serde(rename = "About", alias = "about")]
    pub(crate) about: Spanned<String>,
    #[serde(rename = "Tags", alias = "tags", default)]
    pub(crate) tags: Vec<String>,
    #[serde(rename = "Given", alias = "given", default)]
    pub(crate) given: Vec<String>,
    #[serde(rename = "Forall", alias = "forall", default)]
    pub(crate) forall: IndexMap<ForallVar, String>,
    #[serde(rename = "Actions", alias = "actions", default)]
    pub(crate) actions: IndexMap<String, super::types::ActionSignature>,
    #[serde(rename = "Assume", alias = "assume", default)]
    pub(crate) assume: Vec<RawAssumption>,
    #[serde(rename = "Witness", alias = "witness", default)]
    pub(crate) witness: Vec<RawWitnessCheck>,
    #[serde(rename = "Let", alias = "let", default)]
    pub(crate) let_bindings: IndexMap<String, RawLetBinding>,
    #[serde(rename = "Do", alias = "do", default)]
    pub(crate) do_steps: Vec<RawStep>,
    #[serde(rename = "Prove", alias = "prove")]
    pub(crate) prove: Vec<RawAssertion>,
    #[serde(rename = "Evidence", alias = "evidence")]
    pub(crate) evidence: RawEvidence,
}

/// Raw assumption with span-aware fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAssumption {
    #[serde(rename = "assume", alias = "expr")]
    pub(crate) expr: Spanned<String>,
    pub(crate) because: Spanned<String>,
}

/// Raw assertion with span-aware fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAssertion {
    #[serde(rename = "assert")]
    pub(crate) assert_expr: Spanned<String>,
    pub(crate) because: Spanned<String>,
}

/// Raw witness check with span-aware fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawWitnessCheck {
    pub(crate) cover: Spanned<String>,
    pub(crate) because: Spanned<String>,
}

/// Raw evidence container with span-aware Kani evidence fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawEvidence {
    #[serde(default)]
    pub(crate) kani: Option<RawKaniEvidence>,
    #[serde(default)]
    pub(crate) verus: Option<TheoremValue>,
    #[serde(default)]
    pub(crate) stateright: Option<TheoremValue>,
}

/// Raw Kani evidence with span-aware fields used in validation diagnostics.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawKaniEvidence {
    pub(crate) unwind: Spanned<u32>,
    pub(crate) expect: KaniExpectation,
    #[serde(default, deserialize_with = "deserialize_optional_allow_vacuous")]
    pub(crate) allow_vacuous: Option<Spanned<bool>>,
    #[serde(default)]
    pub(crate) vacuity_because: Option<Spanned<String>>,
}

impl RawTheoremDoc {
    /// Converts this raw document into the public theorem document type,
    /// decoding argument values from raw YAML into [`ArgValue`] variants.
    ///
    /// # Errors
    ///
    /// Returns [`RawDocDecodeError`] when an argument value fails
    /// decoding (e.g., an invalid `{ ref: ... }` target).
    pub(crate) fn to_theorem_doc(&self) -> Result<TheoremDoc, RawDocDecodeError> {
        let let_bindings = convert_let_bindings(&self.let_bindings)?;
        let do_steps = convert_steps(&self.do_steps)?;

        Ok(TheoremDoc {
            schema: self.schema,
            theorem: self.theorem.value.clone(),
            about: self.about.value.clone(),
            tags: self.tags.clone(),
            given: self.given.clone(),
            forall: self.forall.clone(),
            actions: self.actions.clone(),
            assume: self
                .assume
                .iter()
                .map(|a| super::types::Assumption {
                    expr: a.expr.value.clone(),
                    because: a.because.value.clone(),
                })
                .collect(),
            witness: self
                .witness
                .iter()
                .map(|w| super::types::WitnessCheck {
                    cover: w.cover.value.clone(),
                    because: w.because.value.clone(),
                })
                .collect(),
            let_bindings,
            do_steps,
            prove: self
                .prove
                .iter()
                .map(|p| super::types::Assertion {
                    assert_expr: p.assert_expr.value.clone(),
                    because: p.because.value.clone(),
                })
                .collect(),
            evidence: self.evidence.to_evidence(),
        })
    }

    /// Returns the canonical theorem-level fallback location.
    #[must_use]
    pub(crate) const fn theorem_location(&self) -> Location {
        self.theorem.referenced
    }

    /// Returns the best-effort field location for a validation error reason.
    #[must_use]
    pub(crate) fn location_for_validation_reason(&self, reason: ValidationReasonKind) -> Location {
        self.location_for_reason(reason)
            .unwrap_or_else(|| self.theorem_location())
    }

    fn location_for_reason(&self, reason: ValidationReasonKind) -> Option<Location> {
        match reason {
            ValidationReasonKind::AboutEmpty => Some(self.about.referenced),
            ValidationReasonKind::Prove { index, field } => {
                let prove = self.prove.get(index)?;
                Some(location_for_indexed_field(
                    field,
                    prove.assert_expr.referenced,
                    prove.because.referenced,
                ))
            }
            ValidationReasonKind::Assume { index, field } => {
                let assume = self.assume.get(index)?;
                Some(location_for_indexed_field(
                    field,
                    assume.expr.referenced,
                    assume.because.referenced,
                ))
            }
            ValidationReasonKind::Witness { index, field } => {
                let witness = self.witness.get(index)?;
                Some(location_for_indexed_field(
                    field,
                    witness.cover.referenced,
                    witness.because.referenced,
                ))
            }
            ValidationReasonKind::KaniUnwind => self
                .evidence
                .kani
                .as_ref()
                .map(|kani| kani.unwind.referenced),
            ValidationReasonKind::KaniAllowVacuousRequired => {
                self.evidence.kani.as_ref().and_then(|kani| {
                    kani.allow_vacuous
                        .as_ref()
                        .map(|allow_vacuous| allow_vacuous.referenced)
                })
            }
            ValidationReasonKind::KaniVacuityBecauseNonEmpty => {
                self.evidence.kani.as_ref().and_then(|kani| {
                    kani.vacuity_because
                        .as_ref()
                        .map(|vacuity_because| vacuity_because.referenced)
                })
            }
            ValidationReasonKind::KaniWitnessRequired => {
                self.evidence.kani.as_ref().and_then(|kani| {
                    kani.allow_vacuous
                        .as_ref()
                        .map(|allow_vacuous| allow_vacuous.referenced)
                })
            }
        }
    }
}

const fn location_for_indexed_field(
    field: IndexedValidationField,
    value: Location,
    because: Location,
) -> Location {
    match field {
        IndexedValidationField::Value => value,
        IndexedValidationField::Because => because,
    }
}

impl RawEvidence {
    fn to_evidence(&self) -> Evidence {
        Evidence {
            kani: self.kani.as_ref().map(RawKaniEvidence::to_kani_evidence),
            verus: self.verus.clone(),
            stateright: self.stateright.clone(),
        }
    }
}

impl RawKaniEvidence {
    fn to_kani_evidence(&self) -> KaniEvidence {
        KaniEvidence {
            unwind: self.unwind.value,
            expect: self.expect,
            allow_vacuous: self
                .allow_vacuous
                .as_ref()
                .is_some_and(|allow_vacuous| allow_vacuous.value),
            vacuity_because: self
                .vacuity_because
                .as_ref()
                .map(|vacuity_because| vacuity_because.value.clone()),
        }
    }
}

/// Deserializes optional `allow_vacuous` values as `Option<Spanned<bool>>`.
///
/// This helper is used with `#[serde(default)]`, so omitted fields deserialize
/// as `None` before this function runs. Explicit YAML `null` values are
/// rejected, while present values must deserialize as booleans.
fn deserialize_optional_allow_vacuous<'de, D>(
    deserializer: D,
) -> Result<Option<Spanned<bool>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Spanned<bool>>::deserialize(deserializer)?.map_or_else(
        || {
            Err(D::Error::custom(
                "allow_vacuous must be a boolean when provided",
            ))
        },
        |value| Ok(Some(value)),
    )
}

#[cfg(test)]
#[path = "../raw_location_tests.rs"]
mod location_tests;
