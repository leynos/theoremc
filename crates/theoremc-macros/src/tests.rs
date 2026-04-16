//! Unit tests for deterministic theorem-file macro expansion.

use camino::Utf8Path;
use tempfile::TempDir;
use theoremc_core::mangle::{mangle_module_path, mangle_theorem_harness};

use super::expand_theorem_file_at;

struct TheoremSpec<'a> {
    name: &'a str,
    about: &'a str,
}

struct TheoremFixture(String);

impl TheoremFixture {
    fn as_str(&self) -> &str {
        &self.0
    }
}

fn write_fixture(
    fixture_dir: &Utf8Path,
    path: &Utf8Path,
    contents: &TheoremFixture,
) -> Result<(), Box<dyn std::error::Error>> {
    let full_path = fixture_dir.join(path);
    let parent = full_path
        .parent()
        .ok_or_else(|| std::io::Error::other("fixture path must have a parent"))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(full_path, contents.as_str())?;
    Ok(())
}

fn temp_fixture_dir() -> Result<(TempDir, camino::Utf8PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let fixture_dir = Utf8Path::from_path(temp_dir.path())
        .ok_or_else(|| std::io::Error::other("temp dir path must be UTF-8"))?
        .to_path_buf();
    Ok((temp_dir, fixture_dir))
}

fn expand_fixture(
    path: &Utf8Path,
    contents: &TheoremFixture,
) -> Result<String, Box<dyn std::error::Error>> {
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    write_fixture(&fixture_dir, path, contents)?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());
    let tokens = expand_theorem_file_at(&fixture_dir, &path_literal)?;
    Ok(normalize(&tokens.to_string()))
}

fn normalize(tokens: &str) -> String {
    tokens.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn expected_expansion(path: &Utf8Path, theorems: &[&str]) -> String {
    let module_name = mangle_module_path(path.as_str()).module_name().to_owned();
    let harnesses: Vec<String> = theorems
        .iter()
        .map(|theorem| {
            mangle_theorem_harness(path.as_str(), theorem)
                .identifier()
                .to_owned()
        })
        .collect();
    let harness_defs = harnesses
        .iter()
        .map(|harness| format!("pub(crate) fn {harness} () {{ }}"))
        .collect::<Vec<_>>()
        .join(" ");
    let harness_refs = harnesses
        .iter()
        .map(|harness| format!("kani :: {harness}"))
        .collect::<Vec<_>>()
        .join(" , ");

    normalize(&format!(
        "mod {module_name} {{
            const _: & str = include_str! ( concat! ( env! (\"CARGO_MANIFEST_DIR\") , \"/\" , \"{path}\" ) ) ;
            pub(super) mod kani {{ {harness_defs} }}
            const _: [fn(); {}] = [ {} ] ;
        }}",
        harnesses.len(),
        harness_refs,
    ))
}

fn make_single_theorem_fixture(spec: &TheoremSpec<'_>) -> TheoremFixture {
    TheoremFixture(format!(
        concat!(
            "Theorem: {name}\n",
            "About: {about}\n",
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
        name = spec.name,
        about = spec.about,
    ))
}

fn assert_expansion_matches(
    path: &Utf8Path,
    fixture: &TheoremFixture,
    expected_theorems: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = expand_fixture(path, fixture)?;
    let expected = expected_expansion(path, expected_theorems);
    assert_eq!(actual, expected);
    Ok(())
}

fn assert_expansion_is_stable(
    path: &Utf8Path,
    fixture: &TheoremFixture,
) -> Result<(), Box<dyn std::error::Error>> {
    let first = expand_fixture(path, fixture)?;
    let second = expand_fixture(path, fixture)?;
    assert_eq!(first, second);
    Ok(())
}

fn assert_single_theorem_expansion(
    path: &Utf8Path,
    spec: &TheoremSpec<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = make_single_theorem_fixture(spec);
    assert_expansion_matches(path, &fixture, &[spec.name])
}

#[test]
fn single_document_expansion_matches_expected_shape() -> Result<(), Box<dyn std::error::Error>> {
    assert_single_theorem_expansion(
        Utf8Path::new("theorems/single.theorem"),
        &TheoremSpec {
            name: "Smoke",
            about: "Macro smoke test",
        },
    )
}

#[test]
fn multi_document_expansion_preserves_document_order() -> Result<(), Box<dyn std::error::Error>> {
    let path = Utf8Path::new("theorems/multi.theorem");
    let theorem = concat!(
        "Theorem: FirstMacro\n",
        "About: First theorem\n",
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
        "Theorem: SecondMacro\n",
        "About: Second theorem\n",
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
    );

    let actual = expand_fixture(path, &TheoremFixture(theorem.to_owned()))?;
    let expected = expected_expansion(path, &["FirstMacro", "SecondMacro"]);
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn nested_path_expansion_uses_stable_module_mangling() -> Result<(), Box<dyn std::error::Error>> {
    assert_single_theorem_expansion(
        Utf8Path::new("theorems/Nested Path/HTTP-2.theorem"),
        &TheoremSpec {
            name: "HTTP2StreamID",
            about: "Path mangling coverage",
        },
    )
}

#[test]
fn expansion_is_stable_for_repeat_calls() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "RepeatableMacro",
        about: "Repeatability test",
    });
    assert_expansion_is_stable(Utf8Path::new("theorems/repeat.theorem"), &fixture)
}
