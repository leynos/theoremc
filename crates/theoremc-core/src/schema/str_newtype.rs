//! Shared boilerplate for `&str`-backed newtype wrappers.
//!
//! [`YamlKey`](super::loader_decode_location::YamlKey) is a thin
//! `&'a str` wrapper. Rather than repeat a `new`/`as_str` pair on every
//! such wrapper, each one derives [`the_newtype::Newtype`] plus
//! [`derive_more::From`] and [`derive_more::Into`], and this module
//! supplies the constructor and accessor from that derive surface.
//!
//! The trait exists only to name the constructor and accessor. The
//! `From`/`Into` conversions stay authoritative: they are what make the
//! blanket implementation applicable, and every call site keeps its
//! `new`/`as_str` spelling.

use the_newtype::Newtype;

/// Shared constructor and accessor for a `&'a str`-backed newtype.
///
/// Implemented for every wrapper whose inner type is `&'a str` and which
/// provides the corresponding `From`/`Into` conversions. Wrapper authors
/// do not write this by hand; deriving the three macros is enough.
///
/// # Examples
///
/// A wrapper deriving the three macros gains both methods:
///
/// ```
/// use derive_more::{From, Into};
/// use theoremc_core::schema::StrNewtype;
/// use the_newtype::Newtype;
///
/// #[derive(Newtype, From, Into, Copy, Clone)]
/// struct Name<'a>(&'a str);
///
/// let name = Name::new("Witness");
/// assert_eq!(name.as_str(), "Witness");
/// ```
pub trait StrNewtype<'a>: Sized {
    /// Wraps `value` in the implementing newtype.
    fn new(value: &'a str) -> Self;

    /// Returns the wrapped string slice.
    #[must_use]
    fn as_str(&self) -> &'a str;
}

impl<'a, T> StrNewtype<'a> for T
where
    T: Newtype<Inner = &'a str> + From<&'a str> + Into<&'a str> + Copy,
{
    fn new(value: &'a str) -> Self {
        T::from(value)
    }

    fn as_str(&self) -> &'a str {
        (*self).into()
    }
}
