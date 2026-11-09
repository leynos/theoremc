//! Single-key mapping dispatch shared by the raw step and binding enums.
//!
//! `serde`'s untagged enums buffer variants through a `ContentDeserializer`,
//! which discards every `serde_saphyr` span nested inside the variant. The
//! raw step and binding shapes therefore inspect the mapping's single key
//! themselves instead of deriving an untagged enum. This module holds that
//! dispatch once, so each raw enum supplies only its variant names and a
//! decoder for the value under the selected key.
//!
//! The machinery is crate-private and scoped to the raw adapters: it exists
//! for any raw shape that is a mapping with exactly one variant key. It is
//! not a general-purpose deserialization helper and must not be reused
//! outside the raw adapters.

use std::fmt;

use serde::{
    Deserializer,
    de::{self, IgnoredAny, MapAccess, Visitor},
};

/// Builds one raw enum variant from the value under its variant key.
///
/// Implementors hold no state; they exist only to give each raw enum's match
/// arms a distinct type so the visitor scaffolding is written once.
pub(super) trait SingleKeyDecoder<'de> {
    /// The enum this decoder produces.
    type Output;

    /// Consumes the value stored under `key`, returning `Ok(None)` when the
    /// key is not one of this family's variant names.
    fn decode<A>(self, key: &str, map: &mut A) -> Result<Option<Self::Output>, A::Error>
    where
        A: MapAccess<'de>;
}

/// Deserializes a raw enum from a mapping holding exactly one variant key.
pub(super) fn deserialize_single_key<'de, D, C>(
    deserializer: D,
    expectation: &'static str,
    variants: &'static [&'static str],
    decoder: C,
) -> Result<C::Output, D::Error>
where
    D: Deserializer<'de>,
    C: SingleKeyDecoder<'de>,
{
    deserializer.deserialize_map(SingleKeyVisitor {
        expectation,
        variants,
        decoder,
    })
}

/// Implements `Deserialize` for a raw enum that is a mapping holding exactly
/// one of the listed variant keys.
///
/// Every raw single-key enum has this same shape. Generating the impl from one
/// template keeps the dispatch rules from drifting between them.
macro_rules! single_key_deserialize {
    ($ty:ty, $expectation:literal, [$($variant:literal),+ $(,)?], $decoder:expr) => {
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserialize_single_key(
                    deserializer,
                    $expectation,
                    &[$($variant),+],
                    $decoder,
                )
            }
        }
    };
}

pub(super) use single_key_deserialize;

/// Visitor for a mapping that must hold exactly one of a fixed key set.
struct SingleKeyVisitor<D> {
    /// Wording serde uses when the value is not a mapping.
    expectation: &'static str,
    /// Accepted variant keys, in diagnostic order.
    variants: &'static [&'static str],
    /// Builds the enum variant selected by the key.
    decoder: D,
}

impl<'de, D> Visitor<'de> for SingleKeyVisitor<D>
where
    D: SingleKeyDecoder<'de>,
{
    type Value = D::Output;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.expectation)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let variants = self.variants;
        dispatch_single_key(map, variants, |key, access| {
            self.decoder.decode(key, access)
        })
    }
}

/// Deserialize a mapping that holds exactly one supported variant key.
///
/// `dispatch` returns `Ok(None)` for an unrecognized key so the caller can
/// report the accepted variant names with serde's standard wording. Any
/// second key is rejected: the raw step and binding shapes never permit one,
/// and silently ignoring it would hide a malformed document.
fn dispatch_single_key<'de, A, T>(
    mut map: A,
    variants: &'static [&'static str],
    dispatch: impl FnOnce(&str, &mut A) -> Result<Option<T>, A::Error>,
) -> Result<T, A::Error>
where
    A: MapAccess<'de>,
{
    let key = map
        .next_key::<String>()?
        .ok_or_else(|| de::Error::custom("expected a mapping with exactly one key"))?;
    let value =
        dispatch(&key, &mut map)?.ok_or_else(|| de::Error::unknown_variant(&key, variants))?;
    if map.next_key::<IgnoredAny>()?.is_some() {
        return Err(de::Error::custom(
            "mapping must contain exactly one of the supported keys",
        ));
    }
    Ok(value)
}
