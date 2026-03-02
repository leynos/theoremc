//! Semantically decoded action-call argument values.
//!
//! This module defines [`ArgValue`], the domain-level representation of
//! action-call arguments after YAML deserialization and semantic
//! decoding. Plain YAML scalars become [`Literal`](ArgValue::Literal)
//! variants, explicit `{ ref: <Identifier> }` maps become
//! [`Reference`](ArgValue::Reference) variants, and other composite
//! forms are preserved as raw values for future lowering steps
//! (`TFS-5`, `ADR-3`, `DES-5`).

use indexmap::IndexMap;

use super::identifier::{is_rust_reserved_keyword, is_valid_ascii_identifier_pattern};
use super::value::TheoremValue;

/// The sentinel YAML map key that identifies a variable reference.
const REF_KEY: &str = "ref";

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
    /// A YAML map not yet lowered (future: struct-literal synthesis or
    /// `{ literal: ... }` wrapper recognition).
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
    /// `{ literal: "..." }` wrapper in future steps).
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
/// - `TheoremValue::Mapping(m)` (any other map) →
///   `ArgValue::RawMap(m)` (preserved for future lowering)
///
/// The `param_name` argument is used in error messages to identify
/// which argument failed decoding.
///
/// # Errors
///
/// Returns an error string when a `{ ref: ... }` wrapper contains an
/// invalid target: empty string, non-identifier pattern, Rust reserved
/// keyword, or non-string value.
///
/// # Examples
///
///     use theoremc::schema::{ArgValue, LiteralValue, TheoremValue};
///     use theoremc::schema::arg_value::decode_arg_value;
///
///     let result = decode_arg_value("name", TheoremValue::String("hello".into()));
///     assert_eq!(result.unwrap(), ArgValue::Literal(LiteralValue::String("hello".into())));
pub fn decode_arg_value(param_name: &str, value: TheoremValue) -> Result<ArgValue, String> {
    match value {
        TheoremValue::Bool(b) => Ok(ArgValue::Literal(LiteralValue::Bool(b))),
        TheoremValue::Integer(n) => Ok(ArgValue::Literal(LiteralValue::Integer(n))),
        TheoremValue::Float(f) => Ok(ArgValue::Literal(LiteralValue::Float(f))),
        TheoremValue::String(s) => Ok(ArgValue::Literal(LiteralValue::String(s))),
        TheoremValue::Sequence(v) => Ok(ArgValue::RawSequence(v)),
        TheoremValue::Mapping(m) => decode_mapping(param_name, m),
    }
}

/// Decodes a YAML mapping into either a `Reference` (if the map is a
/// single-key `{ ref: <name> }` wrapper) or a `RawMap` (for all other
/// maps).
fn decode_mapping(
    param_name: &str,
    map: IndexMap<String, TheoremValue>,
) -> Result<ArgValue, String> {
    if !is_ref_wrapper(&map) {
        return Ok(ArgValue::RawMap(map));
    }

    // `is_ref_wrapper` confirmed exactly one key == "ref", so the
    // iterator always yields a value.
    let Some(ref_value) = map.into_values().next() else {
        return Ok(ArgValue::RawMap(IndexMap::new()));
    };
    decode_ref_target(param_name, ref_value)
}

/// Returns `true` if the map has exactly one key and that key is `"ref"`.
fn is_ref_wrapper(map: &IndexMap<String, TheoremValue>) -> bool {
    map.len() == 1 && map.contains_key(REF_KEY)
}

/// Validates the `ref` target value and produces an `ArgValue::Reference`.
fn decode_ref_target(param_name: &str, value: TheoremValue) -> Result<ArgValue, String> {
    let TheoremValue::String(name) = value else {
        return Err(format!(
            concat!(
                "argument '{param}': ref value must be a string ",
                "identifier, not {kind}"
            ),
            param = param_name,
            kind = non_string_kind(&value),
        ));
    };

    if name.is_empty() {
        return Err(format!(
            "argument '{param_name}': ref value must not be empty"
        ));
    }

    if !is_valid_ascii_identifier_pattern(&name) {
        return Err(format!(
            concat!(
                "argument '{param}': ref value '{name}' is not a ",
                "valid identifier (must match ",
                "^[A-Za-z_][A-Za-z0-9_]*$)"
            ),
            param = param_name,
            name = name,
        ));
    }

    if is_rust_reserved_keyword(&name) {
        return Err(format!(
            concat!(
                "argument '{param}': ref value '{name}' is a Rust ",
                "reserved keyword"
            ),
            param = param_name,
            name = name,
        ));
    }

    Ok(ArgValue::Reference(name))
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
