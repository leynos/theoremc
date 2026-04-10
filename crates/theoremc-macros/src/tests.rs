//! Unit tests for deterministic theorem-file macro expansion.

use camino::Utf8Path;
use tempfile::TempDir;
use theoremc_core::mangle::{mangle_module_path, mangle_theorem_harness};

use super::expand_theorem_file_at;

fn write_fixture(
    fixture_dir: &Utf8Path,
    path: &str,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let full_path = fixture_dir.join(path);
    let parent = full_path
        .parent()
        .ok_or_else(|| std::io::Error::other("fixture path must have a parent"))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(full_path, contents)?;
    Ok(())
}

fn temp_fixture_dir() -> Result<(TempDir, camino::Utf8PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let fixture_dir = Utf8Path::from_path(temp_dir.path())
        .ok_or_else(|| std::io::Error::other("temp dir path must be UTF-8"))?
        .to_path_buf();
    Ok((temp_dir, fixture_dir))
}

fn expand_fixture(path: &str, contents: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    write_fixture(&fixture_dir, path, contents)?;
    let path_literal = syn::LitStr::new(path, proc_macro2::Span::call_site());
    let tokens = expand_theorem_file_at(&fixture_dir, &path_literal)?;
    Ok(normalize(&tokens.to_string()))
}

fn normalize(tokens: &str) -> String {
    tokens.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn expected_expansion(path: &str, theorems: &[&str]) -> String {
    let module_name = mangle_module_path(path).module_name().to_owned();
    let harnesses: Vec<String> = theorems
        .iter()
        .map(|theorem| {
            mangle_theorem_harness(path, theorem)
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

#[test]
fn single_document_expansion_matches_expected_shape() -> Result<(), Box<dyn std::error::Error>> {
    let path = "theorems/single.theorem";
    let theorem = concat!(
        "Theorem: Smoke\n",
        "About: Macro smoke test\n",
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

    let actual = expand_fixture(path, theorem)?;
    let expected = expected_expansion(path, &["Smoke"]);
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn multi_document_expansion_preserves_document_order() -> Result<(), Box<dyn std::error::Error>> {
    let path = "theorems/multi.theorem";
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

    let actual = expand_fixture(path, theorem)?;
    let expected = expected_expansion(path, &["FirstMacro", "SecondMacro"]);
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn nested_path_expansion_uses_stable_module_mangling() -> Result<(), Box<dyn std::error::Error>> {
    let path = "theorems/Nested Path/HTTP-2.theorem";
    let theorem = concat!(
        "Theorem: HTTP2StreamID\n",
        "About: Path mangling coverage\n",
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

    let actual = expand_fixture(path, theorem)?;
    let expected = expected_expansion(path, &["HTTP2StreamID"]);
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn expansion_is_stable_for_repeat_calls() -> Result<(), Box<dyn std::error::Error>> {
    let path = "theorems/repeat.theorem";
    let theorem = concat!(
        "Theorem: RepeatableMacro\n",
        "About: Repeatability test\n",
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

    let first = expand_fixture(path, theorem)?;
    let second = expand_fixture(path, theorem)?;
    assert_eq!(first, second);
    Ok(())
}
