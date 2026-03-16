//! Semantically decoded action-call argument values.
//!
//! This module defines [`ArgValue`], the domain-level representation of
//! action-call arguments after YAML deserialization and semantic
//! decoding. Plain YAML scalars become [`Literal`](ArgValue::Literal)
//! variants, explicit `{ ref: <Identifier> }` maps become
//! [`Reference`](ArgValue::Reference) variants, explicit
//! `{ literal: <String> }` maps also become `Literal` variants, and
//! other composite forms are preserved as raw values for future
//! lowering steps (`TFS-5`, `ADR-3`, `DES-5`).

use indexmap::IndexMap;

use super::identifier::{is_rust_reserved_keyword, is_valid_ascii_identifier_pattern};
use super::value::TheoremValue;

/// The sentinel YAML map key that identifies a variable reference.
const REF_KEY: &str = "ref";

/// The sentinel YAML map key that identifies an explicit string literal.
const LITERAL_KEY: &str = "literal";

/// Discriminates recognized sentinel map keys for dispatch.
enum SentinelKind {
    /// The `{ ref: <Identifier> }` sentinel.
    Ref,
    /// The `{ literal: <String> }` sentinel.
    Literal,
}

/// Errors produced when decoding a raw [`TheoremValue`] into an
/// [`ArgValue`].
///
/// Each variant carries the parameter name (`param`) for diagnostic
/// context. Variants derive `PartialEq` and `Eq` so callers and tests
/// can match on specific error conditions.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArgDecodeError {
    /// The `{ ref: "" }` target was an empty string.
    #[error("argument '{param}': ref value must not be empty")]
    EmptyRefTarget {
        /// Argument parameter name.
        param: String,
    },

    /// The `{ ref: <name> }` target is not a valid ASCII identifier.
    #[error(
        "argument '{param}': ref value '{name}' is not a valid \
         identifier (must match ^[A-Za-z_][A-Za-z0-9_]*$)"
    )]
    InvalidIdentifier {
        /// Argument parameter name.
        param: String,
        /// The invalid identifier value.
        name: String,
    },

    /// The `{ ref: <name> }` target is a Rust reserved keyword.
    #[error("argument '{param}': ref value '{name}' is a Rust reserved keyword")]
    ReservedKeyword {
        /// Argument parameter name.
        param: String,
        /// The keyword value.
        name: String,
    },

    /// The `ref` value is not a string (e.g. an integer or boolean).
    #[error(
        "argument '{param}': ref value must be a string identifier, \
         not {kind}"
    )]
    NonStringRefTarget {
        /// Argument parameter name.
        param: String,
        /// Human-readable kind label (e.g. "an integer").
        kind: &'static str,
    },

    /// The `literal` value is not a string (e.g. an integer or boolean).
    #[error(
        "argument '{param}': literal value must be a string, \
         not {kind}"
    )]
    NonStringLiteralValue {
        /// Argument parameter name.
        param: String,
        /// Human-readable kind label (e.g. "an integer").
        kind: &'static str,
    },
}

/// A semantically decoded action-call argument value.
///
/// After YAML deserialization, each [`TheoremValue`] in an action
/// call's `args` map is decoded into an `ArgValue` that distinguishes
/// literals from variable references. This encoding ensures that plain
/// YAML strings are unconditionally treated as string literals and
/// variable references require the explicit `{ ref: <name> }` wrapper
/// (`TFS-5` section 5.2, `ADR-3` decision 3).
///
/// # Examples
///
///     use theoremc::schema::ArgValue;
///     use theoremc::schema::LiteralValue;
///
///     let lit = ArgValue::Literal(LiteralValue::String("hello".into()));
///     let reference = ArgValue::Reference("graph".into());
#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    /// A scalar literal value (bool, integer, float, or string).
    Literal(LiteralValue),
    /// An explicit variable reference via `{ ref: <Identifier> }`.
    Reference(String),
    /// A YAML sequence not yet lowered (future: `vec![...]` synthesis).
    RawSequence(Vec<TheoremValue>),
    /// A YAML map not yet lowered (future: struct-literal synthesis).
    RawMap(IndexMap<String, TheoremValue>),
}

/// A scalar literal value decoded from a YAML argument.
///
/// Each variant corresponds to one of the four YAML scalar types that
/// `TheoremValue` supports (null is rejected earlier).
///
/// # Examples
///
///     use theoremc::schema::LiteralValue;
///
///     let s = LiteralValue::String("hello".into());
///     let n = LiteralValue::Integer(42);
///     let b = LiteralValue::Bool(true);
///     let f = LiteralValue::Float(3.14);
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    /// A boolean literal (`true` / `false`).
    Bool(bool),
    /// A signed 64-bit integer literal.
    Integer(i64),
    /// A floating-point literal.
    Float(f64),
    /// A string literal (plain YAML string or explicit
    /// `{ literal: "..." }` wrapper).
    String(String),
}

/// Decodes a raw [`TheoremValue`] into a semantically typed [`ArgValue`].
///
/// Decoding rules (`TFS-5` section 5.2):
///
/// - `TheoremValue::Bool(b)` → `ArgValue::Literal(LiteralValue::Bool(b))`
/// - `TheoremValue::Integer(n)` →
///   `ArgValue::Literal(LiteralValue::Integer(n))`
/// - `TheoremValue::Float(f)` →
///   `ArgValue::Literal(LiteralValue::Float(f))`
/// - `TheoremValue::String(s)` →
///   `ArgValue::Literal(LiteralValue::String(s))`
/// - `TheoremValue::Sequence(v)` → `ArgValue::RawSequence(v)`
/// - `TheoremValue::Mapping(m)` with exactly one key `"ref"` whose
///   value is `TheoremValue::String(name)` where `name` is a valid
///   ASCII identifier and not a Rust keyword →
///   `ArgValue::Reference(name)`
/// - `TheoremValue::Mapping(m)` with exactly one key `"ref"` whose
///   value is invalid → `Err(...)` with an actionable message
/// - `TheoremValue::Mapping(m)` with exactly one key `"literal"` whose
///   value is `TheoremValue::String(s)` →
///   `ArgValue::Literal(LiteralValue::String(s))`
/// - `TheoremValue::Mapping(m)` with exactly one key `"literal"` whose
///   value is not a string → `Err(...)` with an actionable message
/// - `TheoremValue::Mapping(m)` (any other map) →
///   `ArgValue::RawMap(m)` (preserved for future lowering)
///
/// The `param_name` argument is used in error messages to identify
/// which argument failed decoding.
///
/// # Errors
///
/// Returns [`ArgDecodeError`] when a `{ ref: ... }` wrapper contains
/// an invalid target: empty string, non-identifier pattern, Rust
/// reserved keyword, or non-string value. Also returns an error when
/// a `{ literal: ... }` wrapper contains a non-string value.
///
/// # Examples
///
///     use theoremc::schema::{ArgValue, LiteralValue, TheoremValue};
///     use theoremc::schema::arg_value::decode_arg_value;
///
///     let result = decode_arg_value("name", TheoremValue::String("hello".into()));
///     assert_eq!(result.unwrap(), ArgValue::Literal(LiteralValue::String("hello".into())));
pub fn decode_arg_value(param_name: &str, value: TheoremValue) -> Result<ArgValue, ArgDecodeError> {
    match value {
        TheoremValue::Bool(b) => Ok(ArgValue::Literal(LiteralValue::Bool(b))),
        TheoremValue::Integer(n) => Ok(ArgValue::Literal(LiteralValue::Integer(n))),
        TheoremValue::Float(f) => Ok(ArgValue::Literal(LiteralValue::Float(f))),
        TheoremValue::String(s) => Ok(ArgValue::Literal(LiteralValue::String(s))),
        TheoremValue::Sequence(v) => Ok(ArgValue::RawSequence(v)),
        TheoremValue::Mapping(m) => decode_mapping(param_name, m),
    }
}

/// Decodes a YAML mapping into a sentinel wrapper (`Reference` or
/// `Literal`) if the map has exactly one recognized sentinel key, or
/// a `RawMap` for all other maps (struct literal candidates).
fn decode_mapping(
    param_name: &str,
    map: IndexMap<String, TheoremValue>,
) -> Result<ArgValue, ArgDecodeError> {
    let Some(kind) = classify_sentinel(&map) else {
        return Ok(ArgValue::RawMap(map));
    };

    // `classify_sentinel` confirmed exactly one key, so the iterator
    // always yields a value. The `else` branch is unreachable but
    // returns a safe fallback to satisfy the no-panic policy.
    let Some(value) = map.into_values().next() else {
        return Ok(ArgValue::RawMap(IndexMap::new()));
    };
    match kind {
        SentinelKind::Ref => decode_ref_target(param_name, value),
        SentinelKind::Literal => decode_literal_target(param_name, value),
    }
}

/// Classifies a single-key map as a recognized sentinel wrapper, or
/// returns `None` for maps that should pass through as `RawMap`
/// struct-literal candidates — including single-key maps whose key is
/// not a recognized sentinel (e.g. `{ frobnicate: "value" }`).
fn classify_sentinel(map: &IndexMap<String, TheoremValue>) -> Option<SentinelKind> {
    if map.len() != 1 {
        return None;
    }
    let key = map.keys().next()?;
    match key.as_str() {
        REF_KEY => Some(SentinelKind::Ref),
        LITERAL_KEY => Some(SentinelKind::Literal),
        _ => None,
    }
}

/// Validates the `ref` target value and produces an `ArgValue::Reference`.
fn decode_ref_target(param_name: &str, value: TheoremValue) -> Result<ArgValue, ArgDecodeError> {
    let TheoremValue::String(name) = value else {
        return Err(ArgDecodeError::NonStringRefTarget {
            param: param_name.to_owned(),
            kind: non_string_kind(&value),
        });
    };

    if name.is_empty() {
        return Err(ArgDecodeError::EmptyRefTarget {
            param: param_name.to_owned(),
        });
    }

    if !is_valid_ascii_identifier_pattern(&name) {
        return Err(ArgDecodeError::InvalidIdentifier {
            param: param_name.to_owned(),
            name,
        });
    }

    if is_rust_reserved_keyword(&name) {
        return Err(ArgDecodeError::ReservedKeyword {
            param: param_name.to_owned(),
            name,
        });
    }

    Ok(ArgValue::Reference(name))
}

/// Validates the `literal` wrapper value and produces an
/// `ArgValue::Literal(LiteralValue::String(...))`.
///
/// Unlike [`decode_ref_target`], empty strings are accepted because
/// an empty string is a valid string literal.
fn decode_literal_target(
    param_name: &str,
    value: TheoremValue,
) -> Result<ArgValue, ArgDecodeError> {
    let TheoremValue::String(s) = value else {
        return Err(ArgDecodeError::NonStringLiteralValue {
            param: param_name.to_owned(),
            kind: non_string_kind(&value),
        });
    };
    Ok(ArgValue::Literal(LiteralValue::String(s)))
}

/// Returns a human-readable kind label for non-string `TheoremValue`
/// variants, used in error messages.
const fn non_string_kind(value: &TheoremValue) -> &'static str {
    match value {
        TheoremValue::Bool(_) => "a boolean",
        TheoremValue::Integer(_) => "an integer",
        TheoremValue::Float(_) => "a float",
        TheoremValue::String(_) => "a string",
        TheoremValue::Sequence(_) => "a sequence",
        TheoremValue::Mapping(_) => "a mapping",
    }
}

#[cfg(test)]
#[path = "arg_value_tests.rs"]
mod tests;
