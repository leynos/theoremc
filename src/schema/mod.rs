//! Schema types and loading for `.theorem` documents.
//!
//! This module provides strongly-typed Rust representations of the YAML
//! schema defined in the theorem file specification (`TFS-1`). Documents
//! are deserialized using `serde-saphyr` with strict unknown-key rejection
//! and support for both TitleCase and lowercase key aliases.

mod action_name;
pub mod arg_value;
mod diagnostic;
mod error;
mod expr;
mod identifier;
mod loader;
mod newtypes;
mod raw;
mod raw_action;
mod source_id;
mod step;
mod types;
mod validate;
mod value;

pub use arg_value::{ArgValue, LiteralValue};
pub use diagnostic::{SchemaDiagnostic, SchemaDiagnosticCode, SourceLocation};
pub use error::SchemaError;
pub use identifier::validate_identifier;
pub use loader::{load_theorem_docs, load_theorem_docs_with_source};
pub use newtypes::{ForallVar, TheoremName};
pub use source_id::SourceId;
pub use types::{
    ActionCall, Assertion, Assumption, Evidence, KaniEvidence, KaniExpectation, LetBinding,
    LetCall, LetMust, MaybeBlock, Step, StepCall, StepMaybe, StepMust, TheoremDoc, WitnessCheck,
};
pub use value::TheoremValue;
