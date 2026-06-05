//! Internal raw schema types with source-location capture.
//!
//! These types mirror the public schema shape but use `serde_saphyr::Spanned`
//! for selected fields so validation failures can be mapped back to line and
//! column coordinates deterministically.

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, de::Error};
use serde_saphyr::{Location, Spanned};

use crate::canonical_action_name::{CanonicalActionName, InvalidCanonicalActionName};

use super::newtypes::{ForallVar, TheoremName};
use super::raw_action::{self, RawActionCallDecodeError, RawLetBinding, RawStep};
use super::types::{ActionSignature, Evidence, KaniEvidence, KaniExpectation, TheoremDoc};
use super::validate::{ValidationField, ValidationPath};
use super::value::TheoremValue;

/// Errors raised during the raw-to-public conversion in
/// [`RawTheoremDoc::to_theorem_doc`].
///
/// Each variant identifies the location (binding name or step index)
/// and wraps the underlying [`ArgDecodeError`] as a `#[source]` so
/// the full error chain is preserved. Stringification is deferred to
/// the loader boundary when building
/// [`SchemaError::ValidationFailed`](super::error::SchemaError::ValidationFailed).
#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum RawDocDecodeError {
    /// An `Actions` map key failed canonical action-name validation.
    #[error("Actions entry '{action}': {source}")]
    ActionSignature {
        /// Rejected action-name string.
        action: String,
        /// Underlying typed validation failure.
        #[source]
        source: InvalidCanonicalActionName,
    },

    /// An argument in a `Let` binding failed decoding.
    #[error("Let binding '{name}': {source}")]
    LetBinding {
        /// The binding name from the `Let` map.
        name: String,
        /// The underlying action-call decoding failure.
        #[source]
        source: Box<RawActionCallDecodeError>,
    },

    /// An argument in a `Do` step failed decoding.
    #[error("Do step {index}: {source}")]
    DoStep {
        /// One-based step index in the `Do` list.
        index: usize,
        /// The underlying action-call decoding failure.
        #[source]
        source: Box<RawActionCallDecodeError>,
    },
}

impl RawDocDecodeError {
    /// Returns the source location most closely associated with this error.
    #[must_use]
    pub(crate) const fn location(&self) -> Option<Location> {
        match self {
            Self::ActionSignature { .. } => None,
            Self::LetBinding { source, .. } | Self::DoStep { source, .. } => source.location(),
        }
    }
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
        let actions = convert_actions(&self.actions)?;
        let let_bindings = convert_let_bindings(&self.let_bindings)?;
        let do_steps = convert_steps(&self.do_steps)?;

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

    /// Returns the best-effort field location for a typed validation path.
    #[must_use]
    pub(crate) fn location_for_validation_path(&self, path: ValidationPath) -> Location {
        self.location_for_path(path)
            .unwrap_or_else(|| self.theorem_location())
    }

    fn location_for_path(&self, path: ValidationPath) -> Option<Location> {
        match path {
            ValidationPath::Theorem => Some(self.theorem_location()),
            ValidationPath::About => Some(self.about.referenced),
            ValidationPath::Prove { index, field } => {
                let prove = self.prove.get(index.checked_sub(1)?)?;
                Some(match field {
                    ValidationField::Because => prove.because.referenced,
                    ValidationField::Assert => prove.assert_expr.referenced,
                    ValidationField::Expr | ValidationField::Cover => return None,
                })
            }
            ValidationPath::Assume { index, field } => {
                let assume = self.assume.get(index.checked_sub(1)?)?;
                Some(match field {
                    ValidationField::Because => assume.because.referenced,
                    ValidationField::Expr => assume.expr.referenced,
                    ValidationField::Assert | ValidationField::Cover => return None,
                })
            }
            ValidationPath::Witness { index, field } => {
                let witness = self.witness.get(index.checked_sub(1)?)?;
                Some(match field {
                    ValidationField::Because => witness.because.referenced,
                    ValidationField::Cover => witness.cover.referenced,
                    ValidationField::Assert | ValidationField::Expr => return None,
                })
            }
            ValidationPath::KaniUnwind => self
                .evidence
                .kani
                .as_ref()
                .map(|kani| kani.unwind.referenced),
            ValidationPath::KaniAllowVacuous => self.evidence.kani.as_ref().and_then(|kani| {
                kani.allow_vacuous
                    .as_ref()
                    .map(|allow_vacuous| allow_vacuous.referenced)
            }),
            ValidationPath::KaniVacuityBecause => self.evidence.kani.as_ref().and_then(|kani| {
                kani.vacuity_because
                    .as_ref()
                    .map(|vacuity_because| vacuity_because.referenced)
            }),
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

// ── Domain conversion helpers ────────────────────────────────────────

/// Converts raw action signature keys into canonical action-name keys.
fn convert_actions(
    raw: &IndexMap<String, ActionSignature>,
) -> Result<IndexMap<CanonicalActionName, ActionSignature>, RawDocDecodeError> {
    let mut out = IndexMap::with_capacity(raw.len());
    for (action, signature) in raw {
        let canonical = CanonicalActionName::new(action).map_err(|source| {
            RawDocDecodeError::ActionSignature {
                action: action.clone(),
                source,
            }
        })?;
        out.insert(canonical, signature.clone());
    }
    Ok(out)
}

/// Converts a map of raw `Let` bindings, decoding argument values.
fn convert_let_bindings(
    raw: &IndexMap<String, RawLetBinding>,
) -> Result<IndexMap<String, super::types::LetBinding>, RawDocDecodeError> {
    let mut out = IndexMap::with_capacity(raw.len());
    for (name, binding) in raw {
        let converted = raw_action::convert_let_binding(binding).map_err(|source| {
            RawDocDecodeError::LetBinding {
                name: name.clone(),
                source: Box::new(source),
            }
        })?;
        out.insert(name.clone(), converted);
    }
    Ok(out)
}

/// Converts a list of raw `Do` steps, decoding argument values.
fn convert_steps(raw: &[RawStep]) -> Result<Vec<super::types::Step>, RawDocDecodeError> {
    let mut out = Vec::with_capacity(raw.len());
    for (i, step) in raw.iter().enumerate() {
        let converted =
            raw_action::convert_step(step).map_err(|source| RawDocDecodeError::DoStep {
                index: i + 1,
                source: Box::new(source),
            })?;
        out.push(converted);
    }
    Ok(out)
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
mod tests {
    //! Unit tests for raw-to-domain schema conversion.

    use googletest::prelude::*;
    use rstest::rstest;

    use super::{RawDocDecodeError, RawTheoremDoc};
    use crate::schema::validate::{ValidationField, ValidationPath};

    const VALID_DOCUMENT: &str = r"
Theorem: RawBoundary
About: Exercises raw conversion
Actions:
  account.deposit:
    params: {}
Do:
  - call:
      action: account.deposit
      args: {}
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: always reachable
";

    fn parse_one_raw_doc(yaml: &str) -> RawTheoremDoc {
        serde_saphyr::from_multiple::<RawTheoremDoc>(yaml)
            .expect("raw document should deserialize")
            .into_iter()
            .next()
            .expect("test input should contain one document")
    }

    #[rstest]
    fn raw_conversion_carries_canonical_action_names_into_domain() {
        let raw_doc = parse_one_raw_doc(VALID_DOCUMENT);
        let doc = raw_doc
            .to_theorem_doc()
            .expect("valid canonical action names should convert");

        let action = doc
            .do_steps
            .first()
            .and_then(|step| match step {
                super::super::types::Step::Call(step) => Some(step.call.action.as_str()),
                _ => None,
            })
            .expect("first step should be a call action");

        assert_that!(doc.actions.get("account.deposit"), some(anything()));
        assert_that!(action, eq("account.deposit"));
    }

    #[rstest]
    fn raw_conversion_rejects_invalid_action_signature_key() {
        let raw_doc = parse_one_raw_doc(&VALID_DOCUMENT.replace("account.deposit:", "deposit:"));
        let error = raw_doc
            .to_theorem_doc()
            .expect_err("invalid Actions key should be rejected");

        assert!(
            matches!(
                &error,
                RawDocDecodeError::ActionSignature { action, .. } if action == "deposit"
            ),
            "expected invalid Actions key error, got: {error}",
        );
        assert_that!(
            error.to_string(),
            contains_substring("must contain at least two dot-separated segments")
        );
    }

    #[rstest]
    fn raw_conversion_rejects_invalid_action_call_name() {
        let raw_doc = parse_one_raw_doc(
            &VALID_DOCUMENT.replace("action: account.deposit", "action: deposit"),
        );
        let error = raw_doc
            .to_theorem_doc()
            .expect_err("invalid action call name should be rejected");

        assert!(
            matches!(&error, RawDocDecodeError::DoStep { index: 1, .. }),
            "expected Do step action-name error, got: {error}",
        );
        assert_that!(
            error.to_string(),
            contains_substring("action 'deposit': invalid canonical action name")
        );
    }

    #[rstest]
    #[case::about(ValidationPath::About, |doc: &RawTheoremDoc| doc.about.referenced)]
    #[case::prove_assert(
        ValidationPath::Prove {
            index: 1,
            field: ValidationField::Assert,
        },
        |doc: &RawTheoremDoc| doc.prove[0].assert_expr.referenced
    )]
    #[case::prove_because(
        ValidationPath::Prove {
            index: 1,
            field: ValidationField::Because,
        },
        |doc: &RawTheoremDoc| doc.prove[0].because.referenced
    )]
    #[case::assume_expr(
        ValidationPath::Assume {
            index: 1,
            field: ValidationField::Expr,
        },
        |doc: &RawTheoremDoc| doc.assume[0].expr.referenced
    )]
    #[case::assume_because(
        ValidationPath::Assume {
            index: 1,
            field: ValidationField::Because,
        },
        |doc: &RawTheoremDoc| doc.assume[0].because.referenced
    )]
    #[case::witness_cover(
        ValidationPath::Witness {
            index: 1,
            field: ValidationField::Cover,
        },
        |doc: &RawTheoremDoc| doc.witness[0].cover.referenced
    )]
    #[case::witness_because(
        ValidationPath::Witness {
            index: 1,
            field: ValidationField::Because,
        },
        |doc: &RawTheoremDoc| doc.witness[0].because.referenced
    )]
    #[case::kani_unwind(
        ValidationPath::KaniUnwind,
        |doc: &RawTheoremDoc| doc.evidence.kani.as_ref().expect("kani evidence").unwind.referenced
    )]
    #[case::kani_allow_vacuous(
        ValidationPath::KaniAllowVacuous,
        |doc: &RawTheoremDoc| {
            doc.evidence
                .kani
                .as_ref()
                .expect("kani evidence")
                .allow_vacuous
                .as_ref()
                .expect("allow_vacuous")
                .referenced
        }
    )]
    #[case::kani_vacuity_because(
        ValidationPath::KaniVacuityBecause,
        |doc: &RawTheoremDoc| {
            doc.evidence
                .kani
                .as_ref()
                .expect("kani evidence")
                .vacuity_because
                .as_ref()
                .expect("vacuity_because")
                .referenced
        }
    )]
    fn validation_path_maps_to_raw_field_location(
        #[case] path: ValidationPath,
        #[case] expected_location: fn(&RawTheoremDoc) -> serde_saphyr::Location,
    ) {
        let raw_doc = parse_one_raw_doc(
            r"
Theorem: RawBoundary
About: Exercises raw conversion
Assume:
  - assume: 'balance > 0'
    because: balance is positive
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
    allow_vacuous: true
    vacuity_because: proof intentionally omits witnesses
Witness:
  - cover: 'true'
    because: always reachable
",
        );

        assert_that!(
            raw_doc.location_for_validation_path(path),
            eq(expected_location(&raw_doc))
        );
    }
}
