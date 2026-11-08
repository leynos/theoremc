//! Unit tests for the macro expansion query phase.

use camino::Utf8Path;
use rstest::rstest;
use theoremc_core::mangle::mangle_module_path;

use super::*;
use crate::tests_support::{
    TheoremFixture, TheoremSpec, make_single_theorem_fixture, temp_fixture_dir, write_fixture,
};

#[rstest]
fn plan_expansion_derives_module_and_harness_from_the_path()
-> Result<(), Box<dyn std::error::Error>> {
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    let path = Utf8Path::new("theorems/planned.theorem");
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "Planned",
        about: "query phase plan",
    });
    write_fixture(&fixture_dir, path, &fixture)?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());

    let plan = plan_expansion(&fixture_dir, &path_literal)?;

    assert_eq!(
        plan.module_ident.to_string(),
        mangle_module_path(path.as_str()).module_name()
    );
    assert_eq!(plan.path_literal.value(), path.as_str());
    assert_eq!(plan.harnesses.len(), 1);
    // No `Actions` or `Forall` entries means no probes of either kind.
    assert!(plan.action_probes.is_empty());
    assert!(plan.referenced_types.is_empty());
    Ok(())
}

#[test]
fn parse_referenced_type_reports_invalid_types() {
    let error = parse_referenced_type("Vec<")
        .err()
        .expect("unclosed generic must fail");

    assert!(matches!(
        error,
        MacroExpansionError::InvalidReferencedType { .. }
    ));
}

#[test]
fn parse_action_type_reports_the_offending_action() {
    let error = parse_action_type("account.deposit", "Vec<")
        .err()
        .expect("unclosed generic must fail");

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

    let error = generated_harnesses("theorems/no-kani.theorem", &docs)
        .err()
        .expect("missing Kani evidence must fail expansion");

    assert!(matches!(
        error,
        MacroExpansionError::MissingKaniEvidence { ref theorem } if theorem == "NoKani"
    ));
}
