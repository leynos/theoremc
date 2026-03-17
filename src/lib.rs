//! `theoremc` — a formal verification framework that compiles human-readable
//! `.theorem` files into proof harnesses.
//!
//! This crate provides the core library functionality for parsing, validating,
//! and processing theorem documents written in YAML.

/// Mangled-identifier collision detection across loaded theorem documents.
pub mod collision;

/// Action name mangling for deterministic, injective resolution.
pub mod mangle;

/// Schema types for `.theorem` document deserialization and validation.
pub mod schema;

/// Argument-expression lowering for proof harness code generation.
#[doc(hidden)]
pub mod arg_lowering;
