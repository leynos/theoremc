//! Repeatability check for the private theorem-file expansion pipeline.

use super::super::tests_support::{
    TheoremSpec, assert_expansion_is_stable, make_single_theorem_fixture,
};
use camino::Utf8Path;

#[test]
fn expansion_is_stable_for_repeat_calls() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "RepeatableMacro",
        about: "Repeatability test",
    });
    assert_expansion_is_stable(Utf8Path::new("theorems/repeat.theorem"), &fixture)
}
