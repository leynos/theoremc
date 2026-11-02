//! The theorem-owned action signature shape and semantic type comparison.
//!
//! Model-local equivalence belongs with the signature data; schema validation
//! policy remains in the private validation modules.

use indexmap::IndexMap;
use serde::Deserialize;

// ── Action signatures ──────────────────────────────────────────────

/// A theorem-owned expected Rust signature for an action.
///
/// `params` preserves YAML insertion order because generated probes use this
/// order for the bare function pointer parameter list.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSignature {
    /// Ordered parameter names and Rust type strings.
    #[serde(default)]
    pub params: IndexMap<String, String>,
    /// Rust return type. Omitted declarations default to unit.
    #[serde(default = "unit_return_type")]
    pub returns: String,
}

impl ActionSignature {
    /// Returns `true` when `self` and `other` describe the same Rust signature
    /// once parameter and return types are canonicalized.
    ///
    /// Raw string equality treats `Vec<u8>` and `Vec <u8>` as distinct, which
    /// is wrong: schema validation already accepts both as the same Rust type.
    /// This helper round-trips each type through `syn::Type` so equivalent
    /// signatures across multiple theorem documents compare equal.
    #[must_use]
    pub fn is_semantically_equivalent(&self, other: &Self) -> bool {
        if self.params.len() != other.params.len() {
            return false;
        }
        let params_match = self.params.iter().zip(other.params.iter()).all(
            |((self_name, self_ty), (other_name, other_ty))| {
                self_name == other_name && rust_types_equivalent(self_ty, other_ty)
            },
        );
        params_match && rust_types_equivalent(&self.returns, &other.returns)
    }
}

/// Compare two Rust type strings ignoring insignificant whitespace.
///
/// Malformed types are never semantically equivalent. Schema validation rejects
/// them before this point in normal flows.
fn rust_types_equivalent(left: &str, right: &str) -> bool {
    match (
        crate::schema::rust_type::canonical_token_stream(left),
        crate::schema::rust_type::canonical_token_stream(right),
    ) {
        (Some(left_ty), Some(right_ty)) => left_ty == right_ty,
        _ => false,
    }
}

fn unit_return_type() -> String {
    "()".to_owned()
}
