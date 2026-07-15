//! Integration tests for theorem document deserialization.
//!
//! These tests load `.theorem` fixture files and verify that valid
//! documents deserialize correctly. Unhappy-path tests live in
//! `schema_deser_reject.rs`.

use rstest::rstest;
use test_helpers::{FixtureName, load_fixture};
use theoremc::schema::{LetBinding, Step, load_theorem_docs};

#[rstest::fixture]
fn fixture_loader() -> impl Fn(&str) -> std::io::Result<String> {
    |fixture_name| load_fixture(FixtureName::new(fixture_name))
}

macro_rules! ensure {
    ($condition:expr) => {
        if !($condition) {
            return Err(std::io::Error::other(concat!(
                "assertion failed: ",
                stringify!($condition),
            )));
        }
    };
}

macro_rules! ensure_eq {
    ($actual:expr, $expected:expr) => {{
        let actual = &$actual;
        let expected = &$expected;
        if actual != expected {
            return Err(std::io::Error::other(format!(
                "assertion failed: {} == {}; actual: {actual:?}; expected: {expected:?}",
                stringify!($actual),
                stringify!($expected),
            )));
        }
    }};
}

// ── Happy-path tests ────────────────────────────────────────────────

#[rstest]
fn valid_minimal_document_deserializes(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_minimal.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse valid_minimal");
    ensure_eq!(docs.len(), 1);
    let doc = docs.first().expect("should have one document");
    ensure_eq!(doc.theorem.as_str(), "Minimal");
    ensure_eq!(doc.about, "The simplest valid theorem");
    ensure!(doc.tags.is_empty());
    ensure!(doc.given.is_empty());
    ensure!(doc.forall.is_empty());
    ensure!(doc.assume.is_empty());
    Ok(())
}

#[rstest]
fn valid_minimal_has_required_prove(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_minimal.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    ensure_eq!(doc.prove.len(), 1);
    ensure_eq!(
        doc.prove.first().map(|p| p.assert_expr.as_str()),
        Some("true")
    );
    ensure_eq!(
        doc.prove.first().map(|p| p.because.as_str()),
        Some("trivially true")
    );
    Ok(())
}

#[rstest]
fn valid_minimal_has_kani_evidence(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    use theoremc::schema::KaniExpectation;

    let yaml = fixture_loader("valid_minimal.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    let kani = doc
        .evidence
        .kani
        .as_ref()
        .expect("should have kani evidence");
    ensure_eq!(kani.unwind, 1);
    ensure_eq!(kani.expect, KaniExpectation::Success);
    ensure!(!kani.allow_vacuous);
    ensure!(kani.vacuity_because.is_none());
    Ok(())
}

#[rstest]
#[case::omitted("", None)]
#[case::explicit("Schema: 1\n", Some(1))]
fn schema_field_preserves_omitted_and_explicit_values(
    #[case] schema_line: &str,
    #[case] expected: Option<u32>,
) -> std::io::Result<()> {
    let yaml = format!(
        "{schema_line}{}",
        concat!(
            "Theorem: SchemaExample\n",
            "About: Schema behaviour example\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivially true\"\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: \"reachable path\"\n",
        )
    );

    let docs = load_theorem_docs(&yaml).expect("schema example should parse");

    ensure_eq!(docs.first().map(|doc| doc.schema), Some(expected));
    Ok(())
}

#[rstest]
fn valid_full_populates_all_sections(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_full.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse valid_full");
    let doc = docs.first().expect("should have one document");
    ensure_eq!(doc.theorem.as_str(), "FullExample");
    ensure_eq!(doc.schema, Some(1));
    ensure_eq!(doc.tags, vec!["integration", "example"]);
    ensure_eq!(doc.given.len(), 2);
    ensure!(doc.forall.contains_key("amount"));
    Ok(())
}

#[rstest]
fn valid_full_has_let_bindings(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_full.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    ensure_eq!(doc.let_bindings.len(), 2);
    ensure!(doc.let_bindings.contains_key("params"));
    ensure!(doc.let_bindings.contains_key("result"));

    ensure!(matches!(
        doc.let_bindings.get("params"),
        Some(LetBinding::Must(..))
    ));
    ensure!(matches!(
        doc.let_bindings.get("result"),
        Some(LetBinding::Call(..))
    ));
    Ok(())
}

#[rstest]
fn valid_full_has_maybe_step(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_full.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    ensure_eq!(doc.do_steps.len(), 2);
    ensure!(matches!(doc.do_steps.first(), Some(Step::Must(..))));
    ensure!(matches!(doc.do_steps.get(1), Some(Step::Maybe(..))));
    Ok(())
}

#[rstest]
fn valid_full_has_multiple_prove_assertions(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_full.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    ensure_eq!(doc.prove.len(), 2);
    Ok(())
}

// ── Multi-document tests ────────────────────────────────────────────

#[rstest]
fn multi_document_loads_all_theorems(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_multi.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse valid_multi");
    ensure_eq!(docs.len(), 3);
    Ok(())
}

#[rstest]
fn multi_document_preserves_order(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_multi.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let names: Vec<&str> = docs.iter().map(|d| d.theorem.as_str()).collect();
    ensure_eq!(names, vec!["FirstTheorem", "SecondTheorem", "ThirdTheorem"]);
    Ok(())
}

#[rstest]
fn multi_document_has_independent_sections(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_multi.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse");
    // Only the second document has tags.
    ensure!(docs.first().is_some_and(|d| d.tags.is_empty()));
    let second_tags: Vec<&str> = docs
        .get(1)
        .map(|d| d.tags.iter().map(String::as_str).collect())
        .unwrap_or_default();
    ensure_eq!(second_tags, vec!["smoke"]);
    Ok(())
}

// ── Lowercase alias tests ───────────────────────────────────────────

#[rstest]
fn lowercase_aliases_deserialize_identically(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_lowercase.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse lowercase");
    let doc = docs.first().expect("should have one document");
    ensure_eq!(doc.theorem.as_str(), "LowercaseAliases");
    ensure_eq!(doc.tags, vec!["test", "alias"]);
    ensure_eq!(doc.forall.len(), 1);
    ensure_eq!(doc.assume.len(), 1);
    ensure_eq!(doc.witness.len(), 1);
    Ok(())
}

// ── Vacuous configuration test ──────────────────────────────────────

#[rstest]
fn vacuous_allowed_with_reason(
    fixture_loader: impl Fn(&str) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let yaml = fixture_loader("valid_vacuous.theorem")?;
    let docs = load_theorem_docs(&yaml).expect("should parse vacuous");
    let doc = docs.first().expect("should have one document");
    let kani = doc.evidence.kani.as_ref().expect("should have kani");
    ensure!(kani.allow_vacuous);
    ensure!(kani.vacuity_because.is_some());
    Ok(())
}

// ── Edge case: empty optional fields ────────────────────────────────

#[test]
fn empty_optional_fields_default_correctly() {
    let yaml = "
Theorem: EmptyOptionals
About: Only required fields
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
    let docs = load_theorem_docs(yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert!(doc.tags.is_empty());
    assert!(doc.given.is_empty());
    assert!(doc.forall.is_empty());
    assert!(doc.assume.is_empty());
    assert!(doc.let_bindings.is_empty());
    assert!(doc.do_steps.is_empty());
}
