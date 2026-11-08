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

/// Implements `Deserialize` for a string-backed identifier newtype.
///
/// Every identifier newtype shares the same contract: read a YAML string,
/// reject it through [`validate_identifier`], and only then wrap it. Keeping
/// the rule in one macro prevents the newtypes from drifting apart.
macro_rules! impl_identifier_deserialize {
    ($newtype:ty) => {
        impl<'de> Deserialize<'de> for $newtype {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                validate_identifier(&value).map_err(de::Error::custom)?;
                Ok(Self(value))
            }
        }
    };
}

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
    ///
    /// # Examples
    ///
    /// ```
    /// use theoremc_core::schema::TheoremName;
    ///
    /// let name = TheoremName::new("MyTheorem".to_owned())
    ///     .expect("a Rust identifier is a valid theorem name");
    /// assert_eq!(name.as_str(), "MyTheorem");
    ///
    /// assert!(
    ///     TheoremName::new("1NotAnIdentifier".to_owned()).is_err(),
    ///     "identifiers cannot start with a digit",
    /// );
    /// ```
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

impl_identifier_deserialize!(TheoremName);

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
    ///
    /// # Examples
    ///
    /// ```
    /// use theoremc_core::schema::ForallVar;
    ///
    /// let var = ForallVar::new("account".to_owned())
    ///     .expect("a Rust identifier is a valid quantified variable");
    /// assert_eq!(var.as_str(), "account");
    ///
    /// assert!(
    ///     ForallVar::new("type".to_owned()).is_err(),
    ///     "reserved keywords are rejected",
    /// );
    /// ```
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

impl_identifier_deserialize!(ForallVar);

#[cfg(test)]
#[path = "newtypes_tests.rs"]
mod tests;
