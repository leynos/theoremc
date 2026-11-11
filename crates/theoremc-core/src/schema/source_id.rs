//! Source identifier newtype for schema diagnostics.

/// Strongly typed source identifier used for parser and validation diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceId(String);

impl SourceId {
    /// Creates a new source identifier from string-like input.
    ///
    /// Construction is infallible: a source identifier is a diagnostic label,
    /// not a validated domain value.
    ///
    /// # Examples
    ///
    /// ```
    /// use theoremc_core::schema::SourceId;
    ///
    /// let source = SourceId::new("tests/fixtures/valid_full.theorem");
    /// assert_eq!(source.as_str(), "tests/fixtures/valid_full.theorem");
    /// ```
    #[must_use]
    pub fn new(source: impl Into<String>) -> Self {
        Self(source.into())
    }

    /// Returns the source identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SourceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SourceId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for SourceId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
