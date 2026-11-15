//! Internal raw schema types with source-location capture.
//!
//! These types mirror the public schema shape but use `serde_saphyr::Spanned`
//! for selected fields so validation failures can be mapped back to line and
//! column coordinates deterministically.

use indexmap::IndexMap;
use serde::Deserialize;
use serde_saphyr::{Location, Spanned};

use super::newtypes::{ForallVar, TheoremName};
use super::raw_action::{RawLetBinding, RawStep};
use super::raw_evidence::{RawActionSignature, RawEvidence, RawKaniEvidence};
use super::types::TheoremDoc;
use super::validation_reason::{IndexedValidationField, ValidationReasonKind};

mod decode;
mod raw_convert;

pub(crate) use decode::RawDocDecodeError;

/// Raw theorem document with location-carrying fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTheoremDoc {
    /// Declared schema version, when the document supplies one.
    #[serde(rename = "Schema", alias = "schema", default)]
    pub(crate) schema: Option<u32>,
    /// Theorem name with its source location.
    #[serde(rename = "Theorem", alias = "theorem")]
    pub(crate) theorem: Spanned<TheoremName>,
    /// One-line description with its source location.
    #[serde(rename = "About", alias = "about")]
    pub(crate) about: Spanned<String>,
    /// Free-form discovery tags.
    #[serde(rename = "Tags", alias = "tags", default)]
    pub(crate) tags: Vec<String>,
    /// Narrative givens recorded verbatim.
    #[serde(rename = "Given", alias = "given", default)]
    pub(crate) given: Vec<String>,
    /// Typed universally quantified variables keyed by name.
    #[serde(rename = "Forall", alias = "forall", default)]
    pub(crate) forall: IndexMap<ForallVar, String>,
    /// Declared action signatures keyed by canonical name.
    #[serde(rename = "Actions", alias = "actions", default)]
    pub(crate) actions: IndexMap<String, RawActionSignature>,
    /// Assumptions that guard the proof.
    #[serde(rename = "Assume", alias = "assume", default)]
    pub(crate) assume: Vec<RawAssumption>,
    /// Witness checks that must be satisfied.
    #[serde(rename = "Witness", alias = "witness", default)]
    pub(crate) witness: Vec<RawWitnessCheck>,
    /// Named bindings available to later steps.
    #[serde(rename = "Let", alias = "let", default)]
    pub(crate) let_bindings: IndexMap<String, RawLetBinding>,
    /// Imperative steps executed before the assertions.
    #[serde(rename = "Do", alias = "do", default)]
    pub(crate) do_steps: Vec<RawStep>,
    /// Assertions the theorem must establish.
    #[serde(rename = "Prove", alias = "prove")]
    pub(crate) prove: Vec<RawAssertion>,
    /// Verification evidence blocks.
    #[serde(rename = "Evidence", alias = "evidence")]
    pub(crate) evidence: RawEvidence,
}

/// Raw assumption with span-aware fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAssumption {
    /// Assumed expression with its source location.
    #[serde(rename = "assume", alias = "expr")]
    pub(crate) expr: Spanned<String>,
    /// Justification with its source location.
    pub(crate) because: Spanned<String>,
}

/// Raw assertion with span-aware fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAssertion {
    /// Asserted expression with its source location.
    #[serde(rename = "assert")]
    pub(crate) assert_expr: Spanned<String>,
    /// Justification with its source location.
    pub(crate) because: Spanned<String>,
}

/// Raw witness check with span-aware fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawWitnessCheck {
    /// Witness expression with its source location.
    pub(crate) cover: Spanned<String>,
    /// Justification with its source location.
    pub(crate) because: Spanned<String>,
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
        let actions = raw_convert::convert_actions(&self.actions)?;
        let let_bindings = raw_convert::convert_let_bindings(&self.let_bindings)?;
        let do_steps = raw_convert::convert_steps(&self.do_steps)?;

        Ok(TheoremDoc {
            schema: self.schema,
            theorem: self.theorem.value.clone(),
            about: self.about.value.clone(),
            tags: self.tags.clone(),
            given: self.given.clone(),
            forall: self.forall.clone(),
            actions,
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

    /// Finds the most specific source location for a validation reason.
    fn location_for_reason(&self, reason: ValidationReasonKind) -> Option<Location> {
        match reason {
            ValidationReasonKind::AboutEmpty => Some(self.about.referenced),
            ValidationReasonKind::Prove { index, field } => self.prove_location(index, field),
            ValidationReasonKind::Assume { index, field } => self.assume_location(index, field),
            ValidationReasonKind::Witness { index, field } => self.witness_location(index, field),
            ValidationReasonKind::KaniUnwind => {
                self.kani_location(|kani| Some(kani.unwind.referenced))
            }
            ValidationReasonKind::KaniAllowVacuousRequired
            | ValidationReasonKind::KaniWitnessRequired => {
                self.kani_location(allow_vacuous_location)
            }
            ValidationReasonKind::KaniVacuityBecauseNonEmpty => {
                self.kani_location(vacuity_because_location)
            }
        }
    }

    /// Locates the field of the `index`th `Prove` assertion, if it exists.
    fn prove_location(&self, index: usize, field: IndexedValidationField) -> Option<Location> {
        let prove = self.prove.get(index)?;
        Some(location_for_indexed_field(
            field,
            prove.assert_expr.referenced,
            prove.because.referenced,
        ))
    }

    /// Locates the field of the `index`th `Assume` entry, if it exists.
    fn assume_location(&self, index: usize, field: IndexedValidationField) -> Option<Location> {
        let assume = self.assume.get(index)?;
        Some(location_for_indexed_field(
            field,
            assume.expr.referenced,
            assume.because.referenced,
        ))
    }

    /// Locates the field of the `index`th `Witness` check, if it exists.
    fn witness_location(&self, index: usize, field: IndexedValidationField) -> Option<Location> {
        let witness = self.witness.get(index)?;
        Some(location_for_indexed_field(
            field,
            witness.cover.referenced,
            witness.because.referenced,
        ))
    }

    /// Applies `select` to the Kani evidence block, when one is present.
    fn kani_location(
        &self,
        select: impl FnOnce(&RawKaniEvidence) -> Option<Location>,
    ) -> Option<Location> {
        self.evidence.kani.as_ref().and_then(select)
    }
}

/// Selects the `allow_vacuous` location of a Kani evidence block.
fn allow_vacuous_location(kani: &RawKaniEvidence) -> Option<Location> {
    kani.allow_vacuous
        .as_ref()
        .map(|allow_vacuous| allow_vacuous.referenced)
}

/// Selects the `vacuity_because` location of a Kani evidence block.
fn vacuity_because_location(kani: &RawKaniEvidence) -> Option<Location> {
    kani.vacuity_because
        .as_ref()
        .map(|vacuity_because| vacuity_because.referenced)
}

/// Selects the value or justification location for an indexed check.
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

#[cfg(test)]
#[path = "../raw_location_tests.rs"]
mod location_tests;

#[cfg(test)]
#[path = "../raw_action_name_tests.rs"]
mod action_name_tests;
