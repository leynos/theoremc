//! Unit tests for conflicting action signature declarations.
//!
//! Two theorem documents may repeat one action; when their declared signatures
//! disagree, expansion must fail with a diagnostic that names both declaring
//! theorems and both signature summaries.

use super::super::MacroExpansionError;
use super::super::tests_support::{TheoremFixture, expand_fixture};
use camino::Utf8Path;
use googletest::prelude::*;
use rstest::{fixture, rstest};
use theoremc_core::mangle::mangle_action_name;

/// Builds a two-document fixture whose documents disagree on `account.deposit`.
///
/// Each declaration is a `(parameter name, parameter type, return type)`
/// triple. Only the second document may rename its parameter: the first
/// document's `Do` section passes its arguments by name, so its parameter name
/// must stay `account` for the fixture to parse.
fn provenance_conflict_fixture(
    first: (&str, &str, &str),
    conflicting: (&str, &str, &str),
) -> TheoremFixture {
    TheoremFixture(format!(
        concat!(
            "Theorem: FirstProvenanceThesis\n",
            "About: First provenance thesis\n",
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      {first_name}: {first_param}\n",
            "    returns: {first_return}\n",
            "Do:\n",
            "  - call:\n",
            "      action: account.deposit\n",
            "      args:\n",
            "        {first_name}: 1\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: \"reachable\"\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivial\"\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
            "---\n",
            "Theorem: SecondProvenanceThesis\n",
            "About: Second provenance thesis\n",
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      {conflicting_name}: {conflicting_param}\n",
            "    returns: {conflicting_return}\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: \"reachable\"\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivial\"\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
        ),
        first_name = first.0,
        first_param = first.1,
        first_return = first.2,
        conflicting_name = conflicting.0,
        conflicting_param = conflicting.1,
        conflicting_return = conflicting.2,
    ))
}

#[rstest]
#[case::parameter_type_drift(("account", "u64", "bool"), ("account", "u32", "bool"))]
#[case::return_type_drift(("account", "u64", "bool"), ("account", "u64", "u32"))]
#[case::parameter_name_drift(("account", "u64", "bool"), ("wallet", "u64", "bool"))]
fn expansion_names_both_declaring_theorems(
    #[case] first: (&str, &str, &str),
    #[case] conflicting: (&str, &str, &str),
) -> Result<(), Box<dyn std::error::Error>> {
    let theorem = provenance_conflict_fixture(first, conflicting);
    let expected_first = format!("fn({}: {}) -> {}", first.0, first.1, first.2);
    let expected_conflicting = format!(
        "fn({}: {}) -> {}",
        conflicting.0, conflicting.1, conflicting.2
    );

    let expansion = expand_fixture(Utf8Path::new("theorems/provenance.theorem"), &theorem)
        .expect_err("conflicting action signatures should fail expansion");
    let error = expansion
        .downcast_ref::<MacroExpansionError>()
        .ok_or_else(|| std::io::Error::other("expected macro expansion error"))?;

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::ConflictingActionSignature {
            action: eq("account.deposit"),
            first_theorem: eq("FirstProvenanceThesis"),
            conflicting_theorem: eq("SecondProvenanceThesis"),
            first_signature: eq(expected_first.as_str()),
            conflicting_signature: eq(expected_conflicting.as_str()),
        })
    );

    // A user only ever sees the rendered message, so assert the provenance
    // reaches it rather than trusting the typed fields alone.
    let message = error.to_string();
    for fragment in [
        "FirstProvenanceThesis",
        "SecondProvenanceThesis",
        expected_first.as_str(),
        expected_conflicting.as_str(),
    ] {
        assert_that!(message.as_str(), contains_substring(fragment));
    }
    Ok(())
}

fn assert_conflicting_action_signature(
    fixture_path: &str,
    theorem_source: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(theorem_source);

    let expansion = expand_fixture(Utf8Path::new(fixture_path), &theorem)
        .expect_err("conflicting action signature should fail expansion");
    let error = expansion
        .downcast_ref::<MacroExpansionError>()
        .ok_or_else(|| std::io::Error::other("expected macro expansion error"))?;

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::ConflictingActionSignature { .. })
    );
    Ok(())
}

/// The opening of the first document in the stale-conflict fixtures: it
/// declares `account.deposit` with a parameter its `Do` section then passes.
#[fixture]
fn first_document_head() -> &'static str {
    concat!(
        "Theorem: ReferencedActionProbe\n",
        "About: Referenced action probe\n",
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      account: u64\n",
        "    returns: bool\n",
    )
}

/// Everything from the first document's `Do` section through the `---`
/// separator that opens the second document.
#[fixture]
fn first_document_tail() -> &'static str {
    concat!(
        "Do:\n",
        "  - call:\n",
        "      action: account.deposit\n",
        "      args:\n",
        "        account: 1\n",
        "Witness:\n",
        "  - cover: \"true\"\n",
        "    because: \"reachable\"\n",
        "Prove:\n",
        "  - assert: \"true\"\n",
        "    because: \"trivial\"\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "---\n",
    )
}

/// Everything that follows the second document's `Actions` mapping. The second
/// document never calls anything, so its declarations are unreachable within
/// its own `Do` section.
#[fixture]
fn second_document_tail() -> &'static str {
    concat!(
        "Witness:\n",
        "  - cover: \"true\"\n",
        "    because: \"reachable\"\n",
        "Prove:\n",
        "  - assert: \"true\"\n",
        "    because: \"trivial\"\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
    )
}

/// One stale-conflict scenario. The two cases agree on everything except which
/// action conflicts, and consequently whether that action is called anywhere:
/// `account.deposit` is called by the first document, while `inventory.reserve`
/// is declared by both documents and called by neither.
struct StaleConflictCase {
    fixture_path: &'static str,
    /// An additional `Actions` entry the first document declares for the
    /// conflicting action, so both documents declare it.
    first_extra_entry: &'static str,
    second_theorem: &'static str,
    second_about: &'static str,
    /// The second document's conflicting `Actions` entry.
    conflicting_entry: &'static str,
}

#[rstest]
#[case::conflict_on_a_called_action(StaleConflictCase {
    fixture_path: "theorems/stale-conflict.theorem",
    first_extra_entry: "",
    second_theorem: "StaleActionDeclaration",
    second_about: "Stale conflicting declaration",
    conflicting_entry: concat!(
        "  account.deposit:\n",
        "    params:\n",
        "      account: u32\n",
        "    returns: bool\n",
    ),
})]
#[case::conflict_on_a_never_called_action(StaleConflictCase {
    fixture_path: "theorems/unreferenced-conflict.theorem",
    first_extra_entry: concat!(
        "  inventory.reserve:\n",
        "    params:\n",
        "      sku: u64\n",
        "    returns: bool\n",
    ),
    second_theorem: "StaleUnreferencedActionDeclaration",
    second_about: "Stale unreferenced conflicting declaration",
    conflicting_entry: concat!(
        "  inventory.reserve:\n",
        "    params:\n",
        "      sku: String\n",
        "    returns: bool\n",
    ),
})]
fn expansion_rejects_conflicting_action_signature_regardless_of_reachability(
    #[case] case: StaleConflictCase,
    first_document_head: &str,
    first_document_tail: &str,
    second_document_tail: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let theorem_source = format!(
        concat!(
            "{head}{extra}{tail}",
            "Theorem: {second_theorem}\n",
            "About: {second_about}\n",
            "Actions:\n",
            "{conflicting_entry}{second_tail}",
        ),
        head = first_document_head,
        extra = case.first_extra_entry,
        tail = first_document_tail,
        second_theorem = case.second_theorem,
        second_about = case.second_about,
        conflicting_entry = case.conflicting_entry,
        second_tail = second_document_tail,
    );

    assert_conflicting_action_signature(case.fixture_path, theorem_source)
}

#[test]
fn whitespace_only_signature_drift_does_not_conflict() -> Result<(), Box<dyn std::error::Error>> {
    // Two theorems declaring the same action with whitespace-only differences
    // (e.g. `Vec<u8>` vs `Vec <u8>`) describe the same Rust signature and must
    // not be reported as conflicting at macro expansion time.
    let theorem = TheoremFixture(
        concat!(
            "Theorem: FirstActionProbe\n",
            "About: First probe with compact type\n",
            "Actions:\n",
            "  payload.write:\n",
            "    params:\n",
            "      buffer: Vec<u8>\n",
            "    returns: u64\n",
            "Do:\n",
            "  - call:\n",
            "      action: payload.write\n",
            "      args:\n",
            "        buffer: [0]\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: \"reachable\"\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivial\"\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
            "---\n",
            "Theorem: SecondActionProbe\n",
            "About: Second probe with spaced type\n",
            "Actions:\n",
            "  payload.write:\n",
            "    params:\n",
            "      buffer: \"Vec <u8>\"\n",
            "    returns: \"u64 \"\n",
            "Do:\n",
            "  - call:\n",
            "      action: payload.write\n",
            "      args:\n",
            "        buffer: [0]\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: \"reachable\"\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivial\"\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
        )
        .to_owned(),
    );

    // The `?` is the negative control: a spurious conflict would return `Err`
    // here. What remains to check is that accepting the pair did not silently
    // drop the action, so assert the probe was actually generated.
    let expanded = expand_fixture(Utf8Path::new("theorems/whitespace-drift.theorem"), &theorem)?;
    let mangled_action = mangle_action_name("payload.write");
    let probe_identifier = mangled_action.identifier();
    assert!(
        expanded.contains(&format!("crate::theorem_actions::{probe_identifier}")),
        "expected the payload.write probe in the expansion, got: {expanded}",
    );
    Ok(())
}
