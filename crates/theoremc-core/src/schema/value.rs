//! A project-specific YAML value type that rejects null values.
//!
//! `serde-saphyr` does not provide a generic `Value` type. This module
//! defines `TheoremValue` to represent argument values in `ActionCall.args`
//! and placeholder backend configurations, enforcing no-null at the type
//! level and preserving map insertion order via `IndexMap`.

use indexmap::IndexMap;
use serde::Deserialize;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use std::fmt;

/// A YAML value that may appear in theorem action arguments or placeholder
/// backend configurations.
///
/// Null values are rejected at deserialization time because the theorem
/// schema has no semantic use for YAML null.
#[derive(Debug, Clone, PartialEq)]
pub enum TheoremValue {
    /// A boolean scalar (`true` / `false`).
    Bool(bool),
    /// A signed 64-bit integer scalar.
    Integer(i64),
    /// A floating-point scalar.
    Float(f64),
    /// A string scalar.
    String(String),
    /// An ordered sequence of values.
    Sequence(Vec<Self>),
    /// An ordered mapping of string keys to values.
    Mapping(IndexMap<String, Self>),
}

impl<'de> Deserialize<'de> for TheoremValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(TheoremValueVisitor)
    }
}

/// Visitor implementation for deserializing arbitrary YAML values into
/// `TheoremValue`, rejecting null.
struct TheoremValueVisitor;

impl<'de> Visitor<'de> for TheoremValueVisitor {
    type Value = TheoremValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "a non-null YAML value (bool, integer, float, string, \
             sequence, or mapping)",
        )
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(TheoremValue::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(TheoremValue::Integer(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        i64::try_from(v)
            .map(TheoremValue::Integer)
            .map_err(|_| de::Error::custom(format!("integer {v} is out of range for i64")))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(TheoremValue::Float(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(TheoremValue::String(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(TheoremValue::String(v))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Err(de::Error::custom(
            "null values are not permitted in theorem documents",
        ))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Err(de::Error::custom(
            "null values are not permitted in theorem documents",
        ))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(TheoremValue::Sequence(items))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = IndexMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, val)) = map.next_entry()? {
            entries.insert(key, val);
        }
        Ok(TheoremValue::Mapping(entries))
    }
}
