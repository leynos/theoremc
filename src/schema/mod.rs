//! Schema types and loading for `.theorem` documents.
//!
//! This module provides strongly-typed Rust representations of the YAML
//! schema defined in the theorem file specification (`TFS-1`). Documents
//! are deserialized using `serde-saphyr` with strict unknown-key rejection
//! and support for both TitleCase and lowercase key aliases.

mod error;
mod expr;
mod identifier;
mod loader;
mod newtypes;
mod types;
mod validate;
mod value;

pub use error::SchemaError;
pub use identifier::validate_identifier;
pub use loader::load_theorem_docs;
pub use newtypes::{ForallVar, TheoremName};
pub use types::{
    ActionCall, Assertion, Assumption, Evidence, KaniEvidence, KaniExpectation, LetBinding,
    LetCall, LetMust, MaybeBlock, Step, StepCall, StepMaybe, StepMust, TheoremDoc, WitnessCheck,
};
pub use value::TheoremValue;
