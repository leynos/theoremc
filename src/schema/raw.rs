//! Internal raw schema types with source-location capture.
//!
//! These types mirror the public schema shape but use `serde_saphyr::Spanned`
//! for selected fields so validation failures can be mapped back to line and
//! column coordinates deterministically.

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, de::Error};
use serde_saphyr::{Location, Spanned};

use super::newtypes::{ForallVar, TheoremName};
use super::raw_action::{self, RawLetBinding, RawStep};
use super::types::{Evidence, KaniEvidence, KaniExpectation, TheoremDoc};
use super::validate::reason_markers;
use super::value::TheoremValue;

/// Semantic wrapper for validation failure reason strings.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ValidationReason<'a>(&'a str);

impl<'a> ValidationReason<'a> {
    pub(crate) const fn new(reason: &'a str) -> Self {
        Self(reason)
    }

    pub(crate) const fn as_str(&self) -> &'a str {
        self.0
    }

    fn kind(self) -> Option<ValidationKind> {
        let reason = self.as_str();
        let is_because = reason.contains(reason_markers::BECAUSE_FIELD_FRAGMENT);

        if reason.starts_with(reason_markers::ABOUT_NON_EMPTY) {
            return Some(ValidationKind::AboutEmpty);
        }

        if let Some(index) = indexed_error_position(self, reason_markers::PROVE_ASSERTION) {
            return Some(ValidationKind::Prove { index, is_because });
        }

        if let Some(index) = indexed_error_position(self, reason_markers::ASSUME_CONSTRAINT) {
            return Some(ValidationKind::Assume { index, is_because });
        }

        if let Some(index) = indexed_error_position(self, reason_markers::WITNESS) {
            return Some(ValidationKind::Witness { index, is_because });
        }

        if reason.starts_with(reason_markers::KANI_UNWIND_NON_ZERO) {
            return Some(ValidationKind::KaniUnwind);
        }

        if reason.starts_with(reason_markers::KANI_VACUITY_REASON_REQUIRED) {
            return Some(ValidationKind::KaniAllowVacuousRequired);
        }

        if reason.starts_with(reason_markers::KANI_VACUITY_REASON_NON_EMPTY) {
            return Some(ValidationKind::KaniVacuityBecauseNonEmpty);
        }

        None
    }
}

/// Classification of validation reason shapes used for location mapping.
#[derive(Debug, Clone, Copy)]
enum ValidationKind {
    AboutEmpty,
    Prove { index: usize, is_because: bool },
    Assume { index: usize, is_because: bool },
    Witness { index: usize, is_because: bool },
    KaniUnwind,
    KaniAllowVacuousRequired,
    KaniVacuityBecauseNonEmpty,
}

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
    /// Returns an error string when an argument value fails decoding
    /// (e.g., an invalid `{ ref: ... }` target).
    pub(crate) fn to_theorem_doc(&self) -> Result<TheoremDoc, String> {
        let let_bindings = convert_let_bindings(&self.let_bindings)?;
        let do_steps = convert_steps(&self.do_steps)?;

        Ok(TheoremDoc {
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
    pub(crate) fn location_for_validation_reason(&self, reason: ValidationReason<'_>) -> Location {
        self.location_for_reason(reason)
            .unwrap_or_else(|| self.theorem_location())
    }

    fn location_for_reason(&self, reason: ValidationReason<'_>) -> Option<Location> {
        match reason.kind()? {
            ValidationKind::AboutEmpty => Some(self.about.referenced),
            ValidationKind::Prove { index, is_because } => {
                let prove = self.prove.get(index)?;
                Some(if is_because {
                    prove.because.referenced
                } else {
                    prove.assert_expr.referenced
                })
            }
            ValidationKind::Assume { index, is_because } => {
                let assume = self.assume.get(index)?;
                Some(if is_because {
                    assume.because.referenced
                } else {
                    assume.expr.referenced
                })
            }
            ValidationKind::Witness { index, is_because } => {
                let witness = self.witness.get(index)?;
                Some(if is_because {
                    witness.because.referenced
                } else {
                    witness.cover.referenced
                })
            }
            ValidationKind::KaniUnwind => self
                .evidence
                .kani
                .as_ref()
                .map(|kani| kani.unwind.referenced),
            ValidationKind::KaniAllowVacuousRequired => {
                self.evidence.kani.as_ref().and_then(|kani| {
                    kani.allow_vacuous
                        .as_ref()
                        .map(|allow_vacuous| allow_vacuous.referenced)
                })
            }
            ValidationKind::KaniVacuityBecauseNonEmpty => {
                self.evidence.kani.as_ref().and_then(|kani| {
                    kani.vacuity_because
                        .as_ref()
                        .map(|vacuity_because| vacuity_because.referenced)
                })
            }
        }
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

// ── Argument decoding helpers ────────────────────────────────────────

/// Converts a map of raw `Let` bindings, decoding argument values.
fn convert_let_bindings(
    raw: &IndexMap<String, RawLetBinding>,
) -> Result<IndexMap<String, super::types::LetBinding>, String> {
    let mut out = IndexMap::with_capacity(raw.len());
    for (name, binding) in raw {
        let converted = raw_action::convert_let_binding(binding)
            .map_err(|e| format!("Let binding '{name}': {e}"))?;
        out.insert(name.clone(), converted);
    }
    Ok(out)
}

/// Converts a list of raw `Do` steps, decoding argument values.
fn convert_steps(raw: &[RawStep]) -> Result<Vec<super::types::Step>, String> {
    let mut out = Vec::with_capacity(raw.len());
    for (i, step) in raw.iter().enumerate() {
        let converted =
            raw_action::convert_step(step).map_err(|e| format!("Do step {}: {e}", i + 1))?;
        out.push(converted);
    }
    Ok(out)
}

/// Parses indexed validation reason prefixes like `Prove assertion 2: …`.
fn indexed_error_position(reason: ValidationReason<'_>, prefix: &str) -> Option<usize> {
    let prefixed_tail = reason.as_str().strip_prefix(prefix)?;
    let indexed_tail = prefixed_tail.strip_prefix(' ')?;
    let (raw_index, _) = indexed_tail.split_once(':')?;
    let parsed = raw_index.trim().parse::<usize>().ok()?;
    parsed.checked_sub(1)
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
