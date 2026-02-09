//! Multi-document `.theorem` file loading.
//!
//! Provides [`load_theorem_docs`] which deserializes one or more YAML
//! documents from a single string into a `Vec<TheoremDoc>`, validating
//! identifiers at deserialization time (via `TheoremName` / `ForallVar`
//! newtypes) and enforcing structural constraints post-deserialization.

use super::error::SchemaError;
use super::types::TheoremDoc;

/// Loads one or more theorem documents from a YAML string.
///
/// A `.theorem` file may contain a single YAML document or multiple
/// documents separated by `---`. Each document is deserialized into a
/// [`TheoremDoc`] with strict unknown-key rejection. Theorem names
/// and `Forall` keys are validated at deserialization time via the
/// [`TheoremName`](super::newtypes::TheoremName) and
/// [`ForallVar`](super::newtypes::ForallVar) newtypes. Additional
/// structural constraints (non-empty `Prove`, at-least-one Evidence
/// backend) are checked post-deserialization.
///
/// # Errors
///
/// Returns [`SchemaError::Deserialize`] if the YAML is malformed,
/// does not match the theorem schema, or contains invalid identifiers.
/// Returns [`SchemaError::ValidationFailed`] if a structural
/// constraint is violated.
///
/// # Examples
///
///     use theoremc::schema::load_theorem_docs;
///
///     let yaml = r#"
///     Theorem: MyTheorem
///     About: A simple example
///     Prove:
///       - assert: "x > 0"
///         because: "x is positive"
///     Evidence:
///       kani:
///         unwind: 10
///         expect: SUCCESS
///     Witness:
///       - cover: "x == 1"
///         because: "at least one positive value"
///     "#;
///     let docs = load_theorem_docs(yaml).unwrap();
///     assert_eq!(docs.len(), 1);
pub fn load_theorem_docs(input: &str) -> Result<Vec<TheoremDoc>, SchemaError> {
    let docs: Vec<TheoremDoc> =
        serde_saphyr::from_multiple(input).map_err(|e| SchemaError::Deserialize(e.to_string()))?;

    for doc in &docs {
        if doc.prove.is_empty() {
            return Err(SchemaError::ValidationFailed {
                theorem: doc.theorem.to_string(),
                reason: "Prove section must contain at least one assertion".to_owned(),
            });
        }

        if doc.evidence.kani.is_none()
            && doc.evidence.verus.is_none()
            && doc.evidence.stateright.is_none()
        {
            return Err(SchemaError::ValidationFailed {
                theorem: doc.theorem.to_string(),
                reason: concat!(
                    "Evidence section must specify at least one backend ",
                    "(kani, verus, or stateright)",
                )
                .to_owned(),
            });
        }
    }

    Ok(docs)
}

#[cfg(test)]
mod tests {
    use rstest::*;

    use super::*;

    /// Minimal valid YAML for a theorem document.
    const MINIMAL_YAML: &str = r"
Theorem: Minimal
About: The simplest valid theorem
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: always reachable
";

    /// Full example YAML covering every section.
    const FULL_EXAMPLE_YAML: &str = r"
Schema: 1
Theorem: FullExample
About: A theorem using every section
Tags: [integration, example]
Given:
  - an account with balance 100
Forall:
  amount: u64
Assume:
  - expr: 'amount <= 100'
    because: prevent overflow
Witness:
  - cover: 'amount == 50'
    because: mid-range deposit
Let:
  result:
    call:
      action: account.deposit
      args:
        amount: { ref: amount }
Do:
  - call:
      action: account.check_balance
      args:
        expected: 150
Prove:
  - assert: 'balance == 150'
    because: deposit adds to balance
Evidence:
  kani:
    unwind: 10
    expect: SUCCESS
";

    /// Parsed single document from `FULL_EXAMPLE_YAML`.
    #[fixture]
    fn full_doc() -> TheoremDoc {
        let docs = load_theorem_docs(FULL_EXAMPLE_YAML).expect("should parse");
        docs.into_iter().next().expect("should have one doc")
    }

    #[rstest]
    fn load_single_minimal_document() {
        let docs = load_theorem_docs(MINIMAL_YAML).expect("should parse");
        assert_eq!(docs.len(), 1);
        assert_eq!(docs.first().map(|d| d.theorem.as_str()), Some("Minimal"));
    }

    #[rstest]
    fn load_multi_document_file() {
        let yaml = concat!(
            "\nTheorem: First\n",
            "About: First theorem\n",
            "Prove:\n",
            "  - assert: 'true'\n",
            "    because: trivially true\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
            "Witness:\n",
            "  - cover: 'true'\n",
            "    because: always reachable\n",
            "---\n",
            "Theorem: Second\n",
            "About: Second theorem\n",
            "Prove:\n",
            "  - assert: 'false'\n",
            "    because: expected to fail\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 5\n",
            "    expect: FAILURE\n",
            "Witness:\n",
            "  - cover: 'true'\n",
            "    because: always reachable\n",
        );
        let docs = load_theorem_docs(yaml).expect("should parse");
        assert_eq!(docs.len(), 2);
        assert_eq!(docs.first().map(|d| d.theorem.as_str()), Some("First"));
        assert_eq!(docs.get(1).map(|d| d.theorem.as_str()), Some("Second"));
    }

    #[rstest]
    fn reject_unknown_top_level_key() {
        let yaml = concat!(
            "\nTheorem: Bad\n",
            "About: Has an unknown key\n",
            "UnknownKey: oops\n",
            "Prove:\n",
            "  - assert: 'true'\n",
            "    because: trivially true\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
        );
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("unknown field"));
    }

    #[rstest]
    fn reject_wrong_scalar_type_for_tags() {
        let yaml = concat!(
            "\nTheorem: Bad\n",
            "About: Tags should be a list\n",
            "Tags: not_a_list\n",
            "Prove:\n",
            "  - assert: 'true'\n",
            "    because: trivially true\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
        );
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
    }

    #[rstest]
    fn reject_missing_required_field_theorem() {
        let yaml = concat!(
            "\nAbout: Missing Theorem field\n",
            "Prove:\n",
            "  - assert: 'true'\n",
            "    because: trivially true\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
        );
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
    }

    #[rstest]
    fn reject_rust_keyword_theorem_name() {
        let yaml = concat!(
            "\nTheorem: fn\n",
            "About: Theorem named after a keyword\n",
            "Prove:\n",
            "  - assert: 'true'\n",
            "    because: trivially true\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
            "Witness:\n",
            "  - cover: 'true'\n",
            "    because: always reachable\n",
        );
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("Rust reserved keyword"));
    }

    #[rstest]
    fn accept_lowercase_aliases() {
        let yaml = concat!(
            "\ntheorem: LowercaseKeys\n",
            "about: All keys use lowercase aliases\n",
            "tags: [test]\n",
            "given:\n",
            "  - some context\n",
            "prove:\n",
            "  - assert: 'true'\n",
            "    because: trivially true\n",
            "evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
            "witness:\n",
            "  - cover: 'true'\n",
            "    because: always reachable\n",
        );
        let docs = load_theorem_docs(yaml).expect("should parse");
        assert_eq!(docs.len(), 1);
        assert_eq!(
            docs.first().map(|d| d.theorem.as_str()),
            Some("LowercaseKeys")
        );
    }

    #[rstest]
    fn reject_invalid_identifier_in_forall() {
        let yaml = concat!(
            "\nTheorem: Bad\n",
            "About: Forall key is invalid\n",
            "Forall:\n",
            "  123bad: u64\n",
            "Prove:\n",
            "  - assert: 'true'\n",
            "    because: trivially true\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
            "Witness:\n",
            "  - cover: 'true'\n",
            "    because: always reachable\n",
        );
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
    }

    #[rstest]
    fn load_document_has_correct_theorem_name(full_doc: TheoremDoc) {
        assert_eq!(full_doc.theorem.as_str(), "FullExample");
    }

    #[rstest]
    fn load_document_has_correct_metadata_counts(full_doc: TheoremDoc) {
        assert_eq!(full_doc.tags.len(), 2);
        assert_eq!(full_doc.given.len(), 1);
        assert_eq!(full_doc.forall.len(), 1);
    }

    #[rstest]
    fn load_document_has_correct_section_counts(full_doc: TheoremDoc) {
        assert_eq!(full_doc.assume.len(), 1);
        assert_eq!(full_doc.witness.len(), 1);
        assert_eq!(full_doc.let_bindings.len(), 1);
        assert_eq!(full_doc.do_steps.len(), 1);
        assert_eq!(full_doc.prove.len(), 1);
    }
}
