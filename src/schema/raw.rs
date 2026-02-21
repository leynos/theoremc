//! Internal raw schema types with source-location capture.
//!
//! These types mirror the public schema shape but use `serde_saphyr::Spanned`
//! for selected fields so validation failures can be mapped back to line and
//! column coordinates deterministically.

use indexmap::IndexMap;
use serde::Deserialize;
use serde_saphyr::{Location, Spanned};

use super::newtypes::{ForallVar, TheoremName};
use super::types::{Evidence, KaniEvidence, KaniExpectation, LetBinding, Step, TheoremDoc};
use super::value::TheoremValue;

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
    #[serde(rename = "Assume", alias = "assume", default)]
    pub(crate) assume: Vec<RawAssumption>,
    #[serde(rename = "Witness", alias = "witness", default)]
    pub(crate) witness: Vec<RawWitnessCheck>,
    #[serde(rename = "Let", alias = "let", default)]
    pub(crate) let_bindings: IndexMap<String, LetBinding>,
    #[serde(rename = "Do", alias = "do", default)]
    pub(crate) do_steps: Vec<Step>,
    #[serde(rename = "Prove", alias = "prove")]
    pub(crate) prove: Vec<RawAssertion>,
    #[serde(rename = "Evidence", alias = "evidence")]
    pub(crate) evidence: RawEvidence,
}

/// Raw assumption with span-aware fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAssumption {
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
    #[serde(default)]
    pub(crate) allow_vacuous: Option<Spanned<bool>>,
    #[serde(default)]
    pub(crate) vacuity_because: Option<Spanned<String>>,
}

impl RawTheoremDoc {
    /// Converts this raw document into the public theorem document type.
    #[must_use]
    pub(crate) fn to_theorem_doc(&self) -> TheoremDoc {
        TheoremDoc {
            schema: self.schema,
            theorem: self.theorem.value.clone(),
            about: self.about.value.clone(),
            tags: self.tags.clone(),
            given: self.given.clone(),
            forall: self.forall.clone(),
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
            let_bindings: self.let_bindings.clone(),
            do_steps: self.do_steps.clone(),
            prove: self
                .prove
                .iter()
                .map(|p| super::types::Assertion {
                    assert_expr: p.assert_expr.value.clone(),
                    because: p.because.value.clone(),
                })
                .collect(),
            evidence: self.evidence.to_evidence(),
        }
    }

    /// Returns the canonical theorem-level fallback location.
    #[must_use]
    pub(crate) const fn theorem_location(&self) -> Location {
        self.theorem.referenced
    }

    /// Returns the best-effort field location for a validation error reason.
    #[must_use]
    pub(crate) fn location_for_validation_reason(&self, reason: &str) -> Location {
        self.location_for_reason(reason)
            .unwrap_or_else(|| self.theorem_location())
    }

    fn location_for_reason(&self, reason: &str) -> Option<Location> {
        if reason.starts_with("About must be non-empty") {
            return Some(self.about.referenced);
        }

        if let Some(location) = self.prove_field_location(reason) {
            return Some(location);
        }
        if let Some(location) = self.assume_field_location(reason) {
            return Some(location);
        }
        if let Some(location) = self.witness_field_location(reason) {
            return Some(location);
        }
        if let Some(location) = self.kani_field_location(reason) {
            return Some(location);
        }

        None
    }

    fn prove_field_location(&self, reason: &str) -> Option<Location> {
        let index = indexed_error_position(reason, "Prove assertion ")?;
        let prove = self.prove.get(index)?;
        if reason.contains(": because ") {
            Some(prove.because.referenced)
        } else {
            Some(prove.assert_expr.referenced)
        }
    }

    fn assume_field_location(&self, reason: &str) -> Option<Location> {
        let index = indexed_error_position(reason, "Assume constraint ")?;
        let assume = self.assume.get(index)?;
        if reason.contains(": because ") {
            Some(assume.because.referenced)
        } else {
            Some(assume.expr.referenced)
        }
    }

    fn witness_field_location(&self, reason: &str) -> Option<Location> {
        let index = indexed_error_position(reason, "Witness ")?;
        let witness = self.witness.get(index)?;
        if reason.contains(": because ") {
            Some(witness.because.referenced)
        } else {
            Some(witness.cover.referenced)
        }
    }

    fn kani_field_location(&self, reason: &str) -> Option<Location> {
        let kani = self.evidence.kani.as_ref()?;

        if reason.starts_with("Evidence.kani.unwind") {
            return Some(kani.unwind.referenced);
        }
        if reason.starts_with("vacuity_because is required when allow_vacuous is true") {
            return kani
                .allow_vacuous
                .as_ref()
                .map(|allow_vacuous| allow_vacuous.referenced);
        }
        if reason.starts_with("Evidence.kani.vacuity_because must be non-empty") {
            return kani
                .vacuity_because
                .as_ref()
                .map(|vacuity_because| vacuity_because.referenced);
        }

        None
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

/// Parses indexed validation reason prefixes like `Prove assertion 2: …`.
fn indexed_error_position(reason: &str, prefix: &str) -> Option<usize> {
    let tail = reason.strip_prefix(prefix)?;
    let (raw_index, _) = tail.split_once(':')?;
    let parsed = raw_index.trim().parse::<usize>().ok()?;
    parsed.checked_sub(1)
}
