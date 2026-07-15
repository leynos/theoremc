//! Schema-fixture loading and diagnostic assertion helpers.

use cap_std::{ambient_authority, fs_utf8::Dir};
use theoremc_core::schema::{
    SchemaDiagnosticCode, SourceId, TheoremDoc, load_theorem_docs, load_theorem_docs_with_source,
};

use super::{ExpectedFragment, FixtureName};

/// Loads a fixture file from the `tests/fixtures/` directory.
///
/// # Errors
///
/// Returns an I/O error when the fixture cannot be read.
pub fn load_fixture(name: FixtureName<'_>) -> std::io::Result<String> {
    Dir::open_ambient_dir("tests/fixtures", ambient_authority())?.read_to_string(name.as_str())
}

/// Loads a fixture file and formats I/O failures as test-friendly strings.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read.
pub fn load_fixture_text(name: FixtureName<'_>) -> Result<String, String> {
    load_fixture(name).map_err(|error| format!("failed to load fixture {}: {error}", name.as_str()))
}

/// Loads and parses a theorem fixture.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or parsed as theorem YAML.
pub fn load_fixture_docs(name: FixtureName<'_>) -> Result<Vec<TheoremDoc>, String> {
    let yaml = load_fixture_text(name)?;
    load_theorem_docs(&yaml).map_err(|error| error.to_string())
}

/// Asserts that a theorem fixture loads successfully.
///
/// # Errors
///
/// Returns the parser or validation error when loading fails.
pub fn assert_fixture_loads(name: FixtureName<'_>) -> Result<(), String> {
    load_fixture_docs(name).map(|_| ())
}

/// Returns the parser or validation error message for an invalid fixture.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or unexpectedly succeeds.
pub fn fixture_error_message(name: FixtureName<'_>) -> Result<String, String> {
    let yaml = load_fixture_text(name)?;
    load_theorem_docs(&yaml)
        .err()
        .map(|error| error.to_string())
        .ok_or_else(|| format!("fixture should fail: {}", name.as_str()))
}

/// Asserts that a theorem fixture fails to load.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or unexpectedly succeeds.
pub fn assert_fixture_fails(name: FixtureName<'_>) -> Result<(), String> {
    fixture_error_message(name).map(|_| ())
}

/// Asserts that an invalid fixture error contains `expected_fragment`.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read, unexpectedly succeeds, or
/// fails with a different message.
pub fn assert_fixture_error_contains(
    name: FixtureName<'_>,
    expected_fragment: ExpectedFragment<'_>,
) -> Result<(), String> {
    let message = fixture_error_message(name)?;
    if message.contains(expected_fragment.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "expected '{}' in error for {}, got: {message}",
            expected_fragment.as_str(),
            name.as_str()
        ))
    }
}

/// Asserts that an invalid fixture reports the expected typed diagnostic code.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read, unexpectedly succeeds, or
/// reports a diagnostic without the expected code and source location.
pub fn assert_diagnostic_failure(
    fixture_name: FixtureName<'_>,
    expected_code: SchemaDiagnosticCode,
) -> Result<(), String> {
    let source = format!("tests/fixtures/{}", fixture_name.as_str());
    let yaml = load_fixture_text(fixture_name)?;
    let error = load_theorem_docs_with_source(&SourceId::new(&source), &yaml)
        .err()
        .ok_or_else(|| format!("fixture should fail: {}", fixture_name.as_str()))?;
    let diagnostic = error
        .diagnostic()
        .ok_or_else(|| String::from("diagnostic should be present"))?;

    if diagnostic.code != expected_code {
        return Err(format!(
            "unexpected diagnostic code: expected {}, got {}",
            expected_code.as_str(),
            diagnostic.code.as_str()
        ));
    }
    if diagnostic.location.source != source {
        return Err(format!(
            "unexpected diagnostic source: expected {source}, got {}",
            diagnostic.location.source
        ));
    }
    if diagnostic.location.line == 0 {
        return Err(String::from("diagnostic line should be greater than 0"));
    }
    if diagnostic.location.column == 0 {
        return Err(String::from("diagnostic column should be greater than 0"));
    }

    Ok(())
}
