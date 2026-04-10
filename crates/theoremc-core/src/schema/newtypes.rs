//! Validated newtypes for domain identifiers.
//!
//! `TheoremName` and `ForallVar` wrap `String` values that have passed
//! identifier validation at construction time, eliminating stringly-typed
//! validation from downstream code.

use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::Deserialize;
use serde::de;

use super::identifier::validate_identifier;

// ── TheoremName ────────────────────────────────────────────────────

/// A validated theorem name.
///
/// Construction (via deserialization or [`TheoremName::new`]) ensures
/// the contained string matches `^[A-Za-z_][A-Za-z0-9_]*$` and is
/// not a Rust reserved keyword.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheoremName(String);

impl Hash for TheoremName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl TheoremName {
    /// Creates a new `TheoremName` after validating the input.
    ///
    /// # Errors
    ///
    /// Returns [`super::error::SchemaError::InvalidIdentifier`] if
    /// the string fails identifier validation.
    pub fn new(s: String) -> Result<Self, super::error::SchemaError> {
        validate_identifier(&s)?;
        Ok(Self(s))
    }

    /// Returns the inner string as a slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq<&str> for TheoremName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl AsRef<str> for TheoremName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TheoremName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for TheoremName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        validate_identifier(&s).map_err(de::Error::custom)?;
        Ok(Self(s))
    }
}

// ── ForallVar ──────────────────────────────────────────────────────

/// A validated quantified variable name for use in `Forall` mappings.
///
/// Construction (via deserialization or [`ForallVar::new`]) ensures
/// the contained string matches `^[A-Za-z_][A-Za-z0-9_]*$` and is
/// not a Rust reserved keyword.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForallVar(String);

impl Hash for ForallVar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl ForallVar {
    /// Creates a new `ForallVar` after validating the input.
    ///
    /// # Errors
    ///
    /// Returns [`super::error::SchemaError::InvalidIdentifier`] if
    /// the string fails identifier validation.
    pub fn new(s: String) -> Result<Self, super::error::SchemaError> {
        validate_identifier(&s)?;
        Ok(Self(s))
    }

    /// Returns the inner string as a slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq<&str> for ForallVar {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl Borrow<str> for ForallVar {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ForallVar {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ForallVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ForallVar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        validate_identifier(&s).map_err(de::Error::custom)?;
        Ok(Self(s))
    }
}
