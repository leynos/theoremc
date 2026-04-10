//! Core theorem parsing, validation, and deterministic naming for `theoremc`.
//!
//! This crate owns the shared logic consumed by the public facade crate and by
//! proc-macro expansion.

/// Mangled-identifier collision detection across loaded theorem documents.
pub mod collision;

/// Action name mangling for deterministic, injective resolution.
pub mod mangle;

/// Schema types for `.theorem` document deserialization and validation.
pub mod schema;
