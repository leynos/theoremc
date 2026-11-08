//! Raw evidence and action-signature mirrors.
//!
//! These types own every serde attribute that the public domain types must
//! not carry: the domain forms in [`super::types`] are plain data, and only
//! this raw adapter knows the YAML spelling, aliases, and defaults. Each
//! mirror converts explicitly into its domain counterpart.

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, de::Error};
use serde_saphyr::Spanned;

use super::types::{ActionSignature, Evidence, KaniEvidence, KaniExpectation};
use super::value::TheoremValue;

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
    pub(crate) expect: RawKaniExpectation,
    #[serde(default, deserialize_with = "deserialize_optional_allow_vacuous")]
    pub(crate) allow_vacuous: Option<Spanned<bool>>,
    #[serde(default)]
    pub(crate) vacuity_because: Option<Spanned<String>>,
}

/// Raw expected outcome of a Kani verification run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum RawKaniExpectation {
    /// The proof harness is expected to succeed.
    #[serde(rename = "SUCCESS")]
    Success,
    /// The proof harness is expected to find a counterexample.
    #[serde(rename = "FAILURE")]
    Failure,
    /// The proof harness is expected to be unreachable.
    #[serde(rename = "UNREACHABLE")]
    Unreachable,
    /// The verification outcome is undetermined.
    #[serde(rename = "UNDETERMINED")]
    Undetermined,
}

/// Raw expected Rust signature for an action.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawActionSignature {
    /// Ordered parameter names and Rust type strings.
    #[serde(default)]
    pub(crate) params: IndexMap<String, String>,
    /// Rust return type. Omitted declarations default to unit.
    #[serde(default = "unit_return_type")]
    pub(crate) returns: String,
}

impl RawEvidence {
    /// Converts this raw evidence container into its domain counterpart.
    pub(super) fn to_evidence(&self) -> Evidence {
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
            expect: self.expect.to_expectation(),
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

impl RawKaniExpectation {
    const fn to_expectation(self) -> KaniExpectation {
        match self {
            Self::Success => KaniExpectation::Success,
            Self::Failure => KaniExpectation::Failure,
            Self::Unreachable => KaniExpectation::Unreachable,
            Self::Undetermined => KaniExpectation::Undetermined,
        }
    }
}

impl RawActionSignature {
    /// Converts this raw signature into its domain counterpart.
    pub(super) fn to_action_signature(&self) -> ActionSignature {
        ActionSignature {
            params: self.params.clone(),
            returns: self.returns.clone(),
        }
    }
}

fn unit_return_type() -> String {
    "()".to_owned()
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
