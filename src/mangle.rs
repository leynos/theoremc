//! Name mangling for deterministic, collision-resistant resolution.
//!
//! Provides two mangling pipelines, both following
//! `docs/name-mangling-rules.md`:
//!
//! # Action name mangling
//!
//! Transforms canonical dot-separated action names into mangled Rust
//! identifiers suitable for binding in `crate::theorem_actions`.
//!
//! 1. `segment_escape` replaces `_` with `_u` in each segment.
//! 2. `action_slug` splits on `.`, escapes each segment, and joins
//!    with `__`.
//! 3. `hash12` computes the first 12 lowercase hex characters of
//!    the blake3 digest.
//! 4. `mangle_action_name` assembles the full mangled identifier and
//!    resolution path.
//!
//! # Per-file module naming
//!
//! Transforms `.theorem` file paths into deterministic, collision-
//! resistant Rust module names of the form
//! `__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}`.
//!
//! 1. `path_stem` removes a trailing `.theorem` extension.
//! 2. `path_mangle` sanitises the stem into an identifier-safe
//!    fragment (replace separators, collapse underscores, lowercase).
//! 3. `mangle_module_path` assembles the full module name using
//!    `hash12` of the **original** path for disambiguation.

/// The module path into which mangled action identifiers resolve.
///
/// All mangled action names resolve to identifiers within this module.
/// Use this constant (or [`MangledAction::path`]) instead of
/// hard-coding the resolution target string.
pub const RESOLUTION_TARGET: &str = "crate::theorem_actions";

/// The result of mangling a canonical action name.
///
/// Contains the individual components (slug, hash) and the fully
/// assembled mangled identifier and resolution path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledAction {
    slug: String,
    hash: String,
    identifier: String,
    path: String,
}

impl MangledAction {
    /// The escaped slug portion (segments joined by `__`).
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert_eq!(m.slug(), "account__deposit");
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The 12-character lowercase hex hash suffix.
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert_eq!(m.hash().len(), 12);
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The full mangled Rust identifier: `{slug}__h{hash12}`.
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert_eq!(m.identifier(), "account__deposit__h05158894bfb4");
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// The fully qualified resolution path:
    /// `crate::theorem_actions::{identifier}`.
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert!(m.path().starts_with("crate::theorem_actions::"));
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Escapes a single action-name segment by replacing `_` with `_u`.
///
/// ASCII letters and digits are left unchanged. This function assumes
/// the segment has already passed canonical action-name validation
/// (i.e. matches `^[A-Za-z_][A-Za-z0-9_]*$`).
///
/// # Examples
///
///     use theoremc::mangle::segment_escape;
///
///     assert_eq!(segment_escape("deposit"), "deposit");
///     assert_eq!(segment_escape("attach_node"), "attach_unode");
///     assert_eq!(segment_escape("_private"), "_uprivate");
///     assert_eq!(segment_escape("__double"), "_u_udouble");
#[must_use]
pub fn segment_escape(segment: &str) -> String {
    let mut result = String::with_capacity(segment.len() + segment.matches('_').count());
    for ch in segment.chars() {
        if ch == '_' {
            result.push_str("_u");
        } else {
            result.push(ch);
        }
    }
    result
}

/// Builds the escaped slug from a canonical dot-separated action name.
///
/// Splits `canonical_name` on `.`, applies [`segment_escape`] to each
/// segment, and joins the escaped segments with `__`.
///
/// This function assumes the input has already passed canonical
/// action-name validation.
///
/// # Examples
///
///     use theoremc::mangle::action_slug;
///
///     assert_eq!(action_slug("account.deposit"), "account__deposit");
///     assert_eq!(
///         action_slug("hnsw.attach_node"),
///         "hnsw__attach_unode",
///     );
///     assert_eq!(
///         action_slug("hnsw.graph.with_capacity"),
///         "hnsw__graph__with_ucapacity",
///     );
#[must_use]
pub fn action_slug(canonical_name: &str) -> String {
    let mut slug = String::with_capacity(canonical_name.len() * 2);
    let mut segments = canonical_name.split('.');
    if let Some(first) = segments.next() {
        slug.push_str(&segment_escape(first));
    }
    for segment in segments {
        slug.push_str("__");
        slug.push_str(&segment_escape(segment));
    }
    slug
}

/// Computes the first 12 lowercase hex characters of the blake3 hash
/// of the given value.
///
/// # Examples
///
///     use theoremc::mangle::hash12;
///
///     let h = hash12("account.deposit");
///     assert_eq!(h, "05158894bfb4");
#[must_use]
pub fn hash12(value: &str) -> String {
    let digest = blake3::hash(value.as_bytes());
    let hex = digest.to_hex();
    hex.as_str().get(..12).unwrap_or_default().to_owned()
}

/// Mangles a canonical action name into a [`MangledAction`].
///
/// Produces a deterministic, injective Rust identifier and a fully
/// qualified resolution path into `crate::theorem_actions`.
///
/// This function assumes the input has already passed canonical
/// action-name validation (see
/// `validate_canonical_action_name` from the `schema` module).
///
/// # Examples
///
///     use theoremc::mangle::mangle_action_name;
///
///     let m = mangle_action_name("account.deposit");
///     assert_eq!(m.slug(), "account__deposit");
///     assert_eq!(m.hash(), "05158894bfb4");
///     assert_eq!(m.identifier(), "account__deposit__h05158894bfb4");
///     assert_eq!(
///         m.path(),
///         "crate::theorem_actions::account__deposit__h05158894bfb4",
///     );
#[must_use]
pub fn mangle_action_name(canonical_name: &str) -> MangledAction {
    let slug = action_slug(canonical_name);
    let hash = hash12(canonical_name);
    let identifier = format!("{slug}__h{hash}");
    let path = format!("{RESOLUTION_TARGET}::{identifier}");
    MangledAction {
        slug,
        hash,
        identifier,
        path,
    }
}

// ── Per-file module naming ─────────────────────────────────────────

/// The prefix for all generated per-file module names.
const MODULE_PREFIX: &str = "__theoremc__file__";

/// The result of mangling a `.theorem` file path into a per-file
/// Rust module name.
///
/// The module name has the form
/// `__theoremc__file__{mangled_stem}__{hash}` and is deterministic,
/// human-recognizable, and collision-resistant thanks to the 12-
/// character blake3 hash suffix.
///
/// # Examples
///
///     use theoremc::mangle::mangle_module_path;
///
///     let m = mangle_module_path("theorems/bidirectional.theorem");
///     assert!(m.module_name().starts_with("__theoremc__file__"));
///     assert_eq!(m.hash().len(), 12);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledModule {
    stem: String,
    mangled_stem: String,
    hash: String,
    module_name: String,
}

impl MangledModule {
    /// The original path stem (`P` with `.theorem` removed).
    #[must_use]
    pub fn stem(&self) -> &str {
        &self.stem
    }

    /// The sanitised stem after `path_mangle`.
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
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_module_path;
    ///
    ///     let m = mangle_module_path("theorems/bidirectional.theorem");
    ///     assert_eq!(
    ///         m.module_name(),
    ///         "__theoremc__file__theorems_bidirectional__1fc14bdf614f",
    ///     );
    #[must_use]
    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}

/// Removes a trailing `.theorem` extension from `path`, if present.
///
/// Returns the path unchanged when no `.theorem` suffix exists.
///
/// # Examples
///
///     use theoremc::mangle::path_stem;
///
///     assert_eq!(path_stem("foo/bar.theorem"), "foo/bar");
///     assert_eq!(path_stem("no_extension"), "no_extension");
#[must_use]
pub fn path_stem(path: &str) -> &str {
    path.strip_suffix(".theorem").unwrap_or(path)
}

/// Sanitises a path stem into a Rust-identifier-safe fragment.
///
/// Algorithm (per `docs/name-mangling-rules.md` §1):
///
/// 1. Replace `/` and `\` with `__`.
/// 2. Replace any character not in `[A-Za-z0-9_]` with `_`.
/// 3. Collapse consecutive `_` to a single `_`.
/// 4. Lowercase the result.
/// 5. If the result starts with a digit, prefix `_`.
///
/// # Examples
///
///     use theoremc::mangle::path_mangle;
///
///     assert_eq!(path_mangle("theorems/bidirectional"), "theorems_bidirectional");
///     assert_eq!(path_mangle("123foo"), "_123foo");
#[must_use]
pub fn path_mangle(stem: &str) -> String {
    // Steps 1–2: replace separators and non-identifier characters.
    let mut buf = String::with_capacity(stem.len() + 4);
    for ch in stem.chars() {
        match ch {
            '/' | '\\' => buf.push_str("__"),
            _ if ch.is_ascii_alphanumeric() || ch == '_' => buf.push(ch),
            _ => buf.push('_'),
        }
    }

    // Step 3: collapse consecutive underscores.
    let mut collapsed = String::with_capacity(buf.len());
    let mut prev_underscore = false;
    for ch in buf.chars() {
        if ch == '_' {
            if !prev_underscore {
                collapsed.push('_');
            }
            prev_underscore = true;
        } else {
            collapsed.push(ch);
            prev_underscore = false;
        }
    }

    // Step 4: lowercase.
    let mut result = collapsed.to_ascii_lowercase();

    // Step 5: prefix `_` if the result starts with a digit.
    if result.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        result.insert(0, '_');
    }

    result
}

/// Mangles a `.theorem` file path into a [`MangledModule`].
///
/// Produces a deterministic, collision-resistant Rust module name
/// of the form
/// `__theoremc__file__{path_mangle(path_stem(path))}__{hash12(path)}`.
///
/// The `hash12` is computed from the **original** path, not the
/// mangled stem, so paths that sanitise identically still produce
/// distinct module names.
///
/// # Examples
///
///     use theoremc::mangle::mangle_module_path;
///
///     let m = mangle_module_path("theorems/bidirectional.theorem");
///     assert_eq!(m.stem(), "theorems/bidirectional");
///     assert_eq!(m.mangled_stem(), "theorems_bidirectional");
///     assert_eq!(m.hash(), "1fc14bdf614f");
///     assert_eq!(
///         m.module_name(),
///         "__theoremc__file__theorems_bidirectional__1fc14bdf614f",
///     );
#[must_use]
pub fn mangle_module_path(path: &str) -> MangledModule {
    let stem = path_stem(path).to_owned();
    let mangled_stem = path_mangle(&stem);
    let hash = hash12(path);
    let module_name = format!("{MODULE_PREFIX}{mangled_stem}__{hash}");
    MangledModule {
        stem,
        mangled_stem,
        hash,
        module_name,
    }
}

#[cfg(test)]
#[path = "mangle_tests.rs"]
mod tests;
