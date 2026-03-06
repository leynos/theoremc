//! Per-file module-name mangling helpers.
//!
//! These functions implement the `.theorem` file path to Rust-module mapping
//! defined in `docs/name-mangling-rules.md`.

use camino::Utf8Path;

/// The prefix for all generated per-file module names.
pub(crate) const MODULE_PREFIX: &str = "__theoremc__file__";

/// A path stem: a `.theorem` file path with its extension removed.
///
///     use theoremc::mangle::PathStem;
///     let stem = PathStem::from("foo/bar");
///     assert_eq!(stem.as_str(), "foo/bar");
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStem(String);

impl PathStem {
    /// Returns the inner string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PathStem {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PathStem {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for PathStem {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// The result of mangling a `.theorem` file path into a per-file
/// Rust module name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledModule {
    stem: PathStem,
    mangled_stem: String,
    hash: String,
    module_name: String,
}

impl MangledModule {
    /// The original path stem (`P` with `.theorem` removed).
    #[must_use]
    pub fn stem(&self) -> &str {
        self.stem.as_str()
    }

    /// The sanitized stem after `path_mangle`.
    #[must_use]
    pub fn mangled_stem(&self) -> &str {
        &self.mangled_stem
    }

    /// The 12-character blake3 hash of the original path.
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The full generated module name.
    #[must_use]
    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}

/// Removes a trailing `.theorem` extension from `path`, returning
/// a [`PathStem`]. Returns the path unchanged when no `.theorem`
/// suffix exists.
///
///     use theoremc::mangle::path_stem;
///     assert_eq!(path_stem("foo/bar.theorem").as_str(), "foo/bar");
///     assert_eq!(path_stem("no_extension").as_str(), "no_extension");
#[must_use]
pub fn path_stem(path: impl AsRef<Utf8Path>) -> PathStem {
    let s = path.as_ref().as_str();
    PathStem(s.strip_suffix(".theorem").unwrap_or(s).to_owned())
}

/// Maps a single character to its mangled equivalent.
///
/// Path separators (`/`, `\`) and non-ASCII-alphanumeric characters
/// become `_`; ASCII alphanumerics and `_` are lowercased.
#[must_use]
const fn mangle_char(ch: char) -> char {
    match ch {
        '/' | '\\' => '_',
        _ if ch.is_ascii_alphanumeric() || ch == '_' => ch.to_ascii_lowercase(),
        _ => '_',
    }
}

/// Sanitizes a [`PathStem`] into a Rust-identifier-safe fragment.
///
/// Algorithm (per `docs/name-mangling-rules.md` §1):
///
/// 1. Map `/` and `\` to `_`.
/// 2. Map any character not in `[A-Za-z0-9_]` to `_`.
/// 3. Collapse consecutive `_` to a single `_`.
/// 4. Lowercase the result.
/// 5. If the result starts with a digit, prefix `_`.
///
/// # Examples
///
///     use theoremc::mangle::{PathStem, path_mangle};
///     assert_eq!(path_mangle(&PathStem::from("theorems/bidirectional")), "theorems_bidirectional");
///     assert_eq!(path_mangle(&PathStem::from("123foo")), "_123foo");
#[must_use]
pub fn path_mangle(stem: &PathStem) -> String {
    let s = stem.as_str();
    let mut buf = String::with_capacity(s.len() + 4);
    let mut prev_underscore = false;

    for ch in s.chars() {
        let mapped = mangle_char(ch);

        if mapped == '_' && prev_underscore {
            continue;
        }

        buf.push(mapped);
        prev_underscore = mapped == '_';
    }

    if buf.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        buf.insert(0, '_');
    }

    buf
}

/// Mangles a `.theorem` file path into a [`MangledModule`].
///
/// The `hash12` is computed from the **original** path, not the
/// mangled stem, so paths that sanitize identically still produce
/// distinct module names.
///
///     use theoremc::mangle::mangle_module_path;
///     let m = mangle_module_path("theorems/bidirectional.theorem");
///     assert_eq!(m.mangled_stem(), "theorems_bidirectional");
#[must_use]
pub fn mangle_module_path(path: impl AsRef<Utf8Path>) -> MangledModule {
    let path_ref = path.as_ref();
    let s = path_ref.as_str();
    let stem = path_stem(path_ref);
    let mangled_stem = path_mangle(&stem);
    let hash = crate::mangle::hash12(s);
    let module_name = format!("{MODULE_PREFIX}{mangled_stem}__{hash}");
    MangledModule {
        stem,
        mangled_stem,
        hash,
        module_name,
    }
}
