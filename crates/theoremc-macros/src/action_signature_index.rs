//! First-seen index of declared action signatures, with conflict provenance.
//!
//! A `.theorem` file may hold several theorem documents, and each document
//! declares the Rust signatures it expects for the actions it calls. Repeating
//! an action across documents is expected; disagreeing about its signature is
//! not. Building the index in document order keeps the first declaration as the
//! reference, and remembering which theorem declared it lets the conflict
//! diagnostic name both parties instead of only the action.

use std::collections::{BTreeMap, BTreeSet};

use theoremc_core::schema::{ActionSignature, TheoremDoc, TheoremName};

use super::MacroExpansionError;

/// One declared action signature together with the theorem that declared it.
///
/// The declaring theorem is what makes a conflict diagnosable: without it the
/// error can name the action but not the two documents that disagree.
#[derive(Debug, Clone, Copy)]
struct DeclaredSignature<'a> {
    theorem: &'a TheoremName,
    signature: &'a ActionSignature,
}

/// First-seen signatures for the actions one expansion referenced.
#[derive(Debug)]
pub(super) struct ActionSignatureIndex<'a> {
    signatures: BTreeMap<&'a str, DeclaredSignature<'a>>,
}

impl<'a> ActionSignatureIndex<'a> {
    /// Indexes the declarations that `canonical_actions` selects.
    ///
    /// # Errors
    ///
    /// Returns [`MacroExpansionError::ConflictingActionSignature`] when two
    /// documents declare the same action with signatures that are not
    /// semantically equivalent.
    pub(super) fn for_actions(
        theorem_docs: &'a [TheoremDoc],
        canonical_actions: &[&str],
    ) -> Result<Self, MacroExpansionError> {
        let selected = canonical_actions.iter().copied().collect::<BTreeSet<_>>();
        let mut declared_signatures: BTreeMap<&'a str, DeclaredSignature<'a>> = BTreeMap::new();

        for doc in theorem_docs {
            for (action, signature) in &doc.actions {
                let declared = DeclaredSignature {
                    theorem: &doc.theorem,
                    signature,
                };
                Self::insert_signature(&mut declared_signatures, action.as_str(), declared)?;
            }
        }

        let signatures = declared_signatures
            .into_iter()
            .filter(|(action, _)| selected.contains(action))
            .collect();

        Ok(Self { signatures })
    }

    fn insert_signature(
        signatures: &mut BTreeMap<&'a str, DeclaredSignature<'a>>,
        canonical: &'a str,
        declared: DeclaredSignature<'a>,
    ) -> Result<(), MacroExpansionError> {
        let Some(first) = signatures.get(canonical) else {
            signatures.insert(canonical, declared);
            return Ok(());
        };

        if declared
            .signature
            .is_semantically_equivalent(first.signature)
        {
            return Ok(());
        }

        Err(conflicting_signature_error(canonical, *first, declared))
    }

    /// Returns the first-seen signature for a selected action.
    ///
    /// # Errors
    ///
    /// Returns [`MacroExpansionError::MissingActionSignature`] when the action
    /// was referenced but never declared in any document.
    pub(super) fn signature_for(
        &self,
        canonical: &str,
    ) -> Result<&'a ActionSignature, MacroExpansionError> {
        self.signatures
            .get(canonical)
            .map(|declared| declared.signature)
            .ok_or_else(|| MacroExpansionError::MissingActionSignature {
                action: canonical.to_owned(),
            })
    }
}

/// Renders a declared signature the way a theorem author writes it in YAML.
///
/// Parameter names are kept, because they are exactly the keys the author
/// writes under `Actions.params`, and their order is the declaration order
/// that already governs the generated probe.
fn render_signature_summary(signature: &ActionSignature) -> String {
    let params = signature
        .params
        .iter()
        .map(|(name, ty)| format!("{name}: {ty}"))
        .collect::<Vec<String>>()
        .join(", ");
    format!("fn({params}) -> {}", signature.returns)
}

/// Builds the diagnostic for two theorems that disagree about one action.
fn conflicting_signature_error(
    action: &str,
    first: DeclaredSignature<'_>,
    conflicting: DeclaredSignature<'_>,
) -> MacroExpansionError {
    MacroExpansionError::ConflictingActionSignature {
        action: action.to_owned(),
        first_theorem: first.theorem.as_str().to_owned(),
        conflicting_theorem: conflicting.theorem.as_str().to_owned(),
        first_signature: render_signature_summary(first.signature),
        conflicting_signature: render_signature_summary(conflicting.signature),
    }
}
