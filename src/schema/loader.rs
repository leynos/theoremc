//! Multi-document `.theorem` file loading.
//!
//! Provides [`load_theorem_docs`] which deserializes one or more YAML
//! documents from a single string into a `Vec<TheoremDoc>`, then
//! validates theorem identifiers.

use super::error::SchemaError;
use super::identifier::validate_identifier;
use super::types::TheoremDoc;

/// Loads one or more theorem documents from a YAML string.
///
/// A `.theorem` file may contain a single YAML document or multiple
/// documents separated by `---`. Each document is deserialized into a
/// [`TheoremDoc`] with strict unknown-key rejection. After
/// deserialization, theorem identifiers and `Forall` keys are
/// validated.
///
/// # Errors
///
/// Returns [`SchemaError::Deserialize`] if the YAML is malformed or
/// does not match the theorem schema. Returns
/// [`SchemaError::InvalidIdentifier`] if a theorem name or `Forall`
/// key fails identifier validation.
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
        validate_identifier(&doc.theorem)?;

        for key in doc.forall.keys() {
            validate_identifier(key)?;
        }
    }

    Ok(docs)
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn load_single_minimal_document() {
        let docs = load_theorem_docs(MINIMAL_YAML).expect("should parse");
        assert_eq!(docs.len(), 1);
        assert_eq!(docs.first().map(|d| d.theorem.as_str()), Some("Minimal"));
    }

    #[test]
    fn load_multi_document_file() {
        let yaml = "
Theorem: First
About: First theorem
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
---
Theorem: Second
About: Second theorem
Prove:
  - assert: 'false'
    because: expected to fail
Evidence:
  kani:
    unwind: 5
    expect: FAILURE
Witness:
  - cover: 'true'
    because: always reachable
";
        let docs = load_theorem_docs(yaml).expect("should parse");
        assert_eq!(docs.len(), 2);
        assert_eq!(docs.first().map(|d| d.theorem.as_str()), Some("First"));
        assert_eq!(docs.get(1).map(|d| d.theorem.as_str()), Some("Second"));
    }

    #[test]
    fn reject_unknown_top_level_key() {
        let yaml = "
Theorem: Bad
About: Has an unknown key
UnknownKey: oops
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
";
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("unknown field"));
    }

    #[test]
    fn reject_wrong_scalar_type_for_tags() {
        let yaml = "
Theorem: Bad
About: Tags should be a list
Tags: not_a_list
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
";
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn reject_missing_required_field_theorem() {
        let yaml = "
About: Missing Theorem field
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
";
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn reject_rust_keyword_theorem_name() {
        let yaml = "
Theorem: fn
About: Theorem named after a keyword
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
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("Rust reserved keyword"));
    }

    #[test]
    fn accept_lowercase_aliases() {
        let yaml = "
theorem: LowercaseKeys
about: All keys use lowercase aliases
tags: [test]
given:
  - some context
prove:
  - assert: 'true'
    because: trivially true
evidence:
  kani:
    unwind: 1
    expect: SUCCESS
witness:
  - cover: 'true'
    because: always reachable
";
        let docs = load_theorem_docs(yaml).expect("should parse");
        assert_eq!(docs.len(), 1);
        assert_eq!(
            docs.first().map(|d| d.theorem.as_str()),
            Some("LowercaseKeys")
        );
    }

    #[test]
    fn reject_invalid_identifier_in_forall() {
        let yaml = "
Theorem: Bad
About: Forall key is invalid
Forall:
  123bad: u64
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
        let result = load_theorem_docs(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn load_document_has_correct_theorem_name() {
        let docs = load_theorem_docs(full_example_yaml()).expect("should parse");
        assert_eq!(docs.len(), 1);
        let doc = docs.first().expect("should have one doc");
        assert_eq!(doc.theorem, "FullExample");
    }

    #[test]
    fn load_document_has_correct_metadata_counts() {
        let docs = load_theorem_docs(full_example_yaml()).expect("should parse");
        let doc = docs.first().expect("should have one doc");
        assert_eq!(doc.tags.len(), 2);
        assert_eq!(doc.given.len(), 1);
        assert_eq!(doc.forall.len(), 1);
    }

    #[test]
    fn load_document_has_correct_section_counts() {
        let docs = load_theorem_docs(full_example_yaml()).expect("should parse");
        let doc = docs.first().expect("should have one doc");
        assert_eq!(doc.assume.len(), 1);
        assert_eq!(doc.witness.len(), 1);
        assert_eq!(doc.let_bindings.len(), 1);
        assert_eq!(doc.do_steps.len(), 1);
        assert_eq!(doc.prove.len(), 1);
    }

    fn full_example_yaml() -> &'static str {
        "
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
"
    }
}
