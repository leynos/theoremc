//! Unit tests for the macro expansion query phase.

use camino::Utf8Path;
use googletest::{assert_that, matchers::eq};
use rstest::rstest;
use theoremc_core::canonical_action_name::CanonicalActionName;
use theoremc_core::mangle::mangle_module_path;

use super::*;
use crate::tests_support::{
    TheoremFixture, TheoremSpec, make_single_theorem_fixture, temp_fixture_dir, write_fixture,
};

#[rstest]
fn load_theorem_docs_reads_a_manifest_relative_file() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    let path = Utf8Path::new("theorems/planned.theorem");
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "Planned",
        about: "query phase plan",
    });
    write_fixture(&fixture_dir, path, &fixture)?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());

    let theorem_docs = load_theorem_docs(&fixture_dir, &path_literal)?;

    assert_that!(theorem_docs.len(), eq(1_usize));
    Ok(())
}

#[rstest]
fn plan_expansion_derives_from_parsed_documents_without_filesystem_access()
-> Result<(), Box<dyn std::error::Error>> {
    let path = Utf8Path::new("theorems/planned.theorem");
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "Planned",
        about: "query phase plan",
    });
    let theorem_docs = theoremc_core::schema::load_theorem_docs(fixture.0.as_str())?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());

    let plan = plan_expansion(&path_literal, &theorem_docs)?;

    assert_that!(
        plan.module_ident.to_string(),
        eq(mangle_module_path(path.as_str()).module_name())
    );
    assert_that!(plan.path_literal.value(), eq(path.as_str()));
    assert_that!(plan.harnesses.len(), eq(1_usize));
    // No `Actions` or `Forall` entries means no probes of either kind.
    assert_that!(plan.action_probes.len(), eq(0_usize));
    assert_that!(plan.referenced_types.len(), eq(0_usize));
    Ok(())
}

#[test]
fn parse_referenced_type_reports_invalid_types() {
    let Err(error) = parse_referenced_type("Vec<") else {
        panic!("unclosed generic must fail");
    };

    assert!(matches!(
        error,
        MacroExpansionError::InvalidReferencedType { .. }
    ));
}

#[test]
fn parse_action_type_reports_the_offending_action() {
    let canonical = CanonicalActionName::new("account.deposit").expect("valid canonical name");
    let Err(error) = parse_action_type(&canonical, "Vec<") else {
        panic!("unclosed generic must fail");
    };

    let MacroExpansionError::InvalidActionSignature { action, .. } = error else {
        panic!("expected InvalidActionSignature, got a different variant");
    };
    assert_eq!(action, "account.deposit");
}

#[test]
fn generated_harnesses_reject_theorems_without_kani_evidence() {
    let fixture = TheoremFixture(
        concat!(
            "Theorem: NoKani\n",
            "About: missing kani evidence\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivial\"\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: \"reachable\"\n",
            "Evidence:\n",
            "  verus: \"future backend\"\n",
        )
        .to_owned(),
    );
    let docs = theoremc_core::schema::load_theorem_docs(&fixture.0)
        .expect("fixture should parse as a theorem document");

    let Err(error) = generated_harnesses("theorems/no-kani.theorem", &docs) else {
        panic!("missing Kani evidence must fail expansion");
    };

    assert!(matches!(
        error,
        MacroExpansionError::MissingKaniEvidence { ref theorem } if theorem == "NoKani"
    ));
}
