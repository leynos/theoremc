//! Small parser-message wrappers used by schema loader diagnostics.

/// Newtype representing a YAML field name extracted from error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldName<'a>(&'a str);

impl<'a> FieldName<'a> {
    /// Wraps a field name borrowed from a parser message.
    pub(crate) const fn new(name: &'a str) -> Self {
        Self(name)
    }

    /// Returns the wrapped field name.
    pub(crate) const fn as_str(self) -> &'a str {
        self.0
    }
}

/// Newtype representing an error message from deserialization failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErrorMessage<'a>(&'a str);

impl<'a> ErrorMessage<'a> {
    /// Wraps a deserialization error message.
    pub(crate) const fn new(message: &'a str) -> Self {
        Self(message)
    }

    /// Returns the wrapped message text.
    pub(crate) const fn as_str(self) -> &'a str {
        self.0
    }
}
