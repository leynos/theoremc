//! Contract tests for the root `dylint.toml`.
//!
//! The Whitaker configuration must not exempt a whole crate, because that
//! abandons the policy for every module in it. Where an exclusion is genuinely
//! warranted, the narrowest key is `excluded_paths`, and every entry must
//! carry a comment immediately above it saying why. This test reads the file
//! as text, so an exemption cannot arrive unexplained or wider than declared.
//!
//! `tests/lint_ci_contract.rs` holds the workflow side of the same gate, and
//! `tests/lint_makefile_contract.rs` holds the recipes.

const DYLINT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dylint.toml"));

/// Returns the active lines of a Dylint configuration.
///
/// Comment lines are removed so the prose that explains the exclusion policy
/// cannot read as the policy itself.
fn active_lines(document: &str) -> Vec<&str> {
    document
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Reports whether the line before `index` is a justification comment.
fn has_justification(lines: &[&str], index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|previous| lines.get(previous))
        .is_some_and(|line| line.trim().starts_with('#'))
}

/// Reports whether a value begins with a TOML string, basic or literal.
///
/// TOML accepts both `"crate::module"` and `'crate::module'`, so a contract
/// that recognized only the double-quoted form would miss a real exclusion.
fn starts_with_string(value: &str) -> bool {
    value.starts_with(['"', '\''])
}

/// Returns the value of an `excluded_paths = …` assignment.
///
/// The key is compared through [`assignment_key`], so a quoted key is read as
/// the same declaration as an unquoted one.
fn excluded_paths_value(trimmed: &str) -> Option<&str> {
    let (key, value) = trimmed.split_once('=')?;
    (key.trim().trim_matches(['"', '\'']) == "excluded_paths").then(|| value.trim())
}

/// Returns the active `excluded_paths` entries that carry no adjacent reason.
///
/// A bracket inside a comment or a string is data, so the array's extent is
/// tracked on comment-free text rather than on the raw line.
fn unjustified_exclusions(document: &str) -> Vec<String> {
    let lines: Vec<&str> = document.lines().collect();
    let mut offences = Vec::new();
    let mut inside_list = false;
    for (index, line) in lines.iter().enumerate() {
        let Some(code) = comment_free(line.trim()) else {
            continue;
        };
        if inside_list {
            // A closing bracket may share the line with the final entry, so
            // the entry is checked before the list closes.
            let closing = code.find(']');
            let entry = closing.map_or(code, |end| code.get(..end).unwrap_or_default());
            record_entry(entry.trim(), &lines, index, &mut offences);
            if closing.is_some() {
                inside_list = false;
            }
            continue;
        }
        let Some(value) = excluded_paths_value(code) else {
            continue;
        };
        if value.starts_with('[') && !value.contains(']') {
            inside_list = true;
            record_opening_entry(value, &lines, index, &mut offences);
            continue;
        }
        record_entry(value, &lines, index, &mut offences);
    }
    offences
}

/// Returns a TOML line without its trailing comment.
///
/// A `#` inside a string is data, so the scan tracks the open quote. A line
/// that is only a comment yields `None`, which the caller treats as no code.
fn comment_free(trimmed: &str) -> Option<&str> {
    let mut quote = None;
    for (index, character) in trimmed.char_indices() {
        match character {
            '"' | '\'' if quote == Some(character) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(character),
            '#' if quote.is_none() => {
                return trimmed
                    .get(..index)
                    .map(str::trim)
                    .filter(|code| !code.is_empty());
            }
            _ => {}
        }
    }
    Some(trimmed).filter(|code| !code.is_empty())
}

/// Records an entry that shares the line opening a multi-line array.
fn record_opening_entry(value: &str, lines: &[&str], index: usize, offences: &mut Vec<String>) {
    if let Some((_, first)) = value.split_once('[') {
        record_entry(first.trim(), lines, index, offences);
    }
}

/// Records `line` when it is an exclusion entry with no adjacent reason.
fn record_entry(line: &str, lines: &[&str], index: usize, offences: &mut Vec<String>) {
    let entry = line.trim_start_matches('[').trim_start();
    if starts_with_string(entry) && !has_justification(lines, index) {
        offences.push(line.to_owned());
    }
}

/// Returns the key of a TOML assignment, without quotes.
///
/// TOML permits a quoted key, so the quotes are syntax rather than part of the
/// name and are removed before the key is compared with the policy.
fn assignment_key(line: &str) -> Option<&str> {
    line.split_once('=')
        .map(|(key, _)| key.trim().trim_matches(['"', '\'']))
}

/// Verifies that `dylint.toml` declares no whole-crate exclusion.
#[test]
fn the_dylint_configuration_exempts_no_whole_crate() {
    assert!(
        !active_lines(DYLINT)
            .iter()
            .any(|line| assignment_key(line) == Some("excluded_crates")),
        "dylint.toml must not exempt a whole crate; prefer `excluded_paths` with a recorded reason"
    );
}

/// A whole-crate exclusion is rejected in every key spelling.
#[test]
fn a_whole_crate_exclusion_is_rejected() {
    for entry in [
        "excluded_crates = [\"theoremc\"]\n",
        "\"excluded_crates\" = [\"theoremc\"]\n",
        "'excluded_crates' = ['theoremc']\n",
    ] {
        assert_eq!(
            assignment_key(entry.trim_end()),
            Some("excluded_crates"),
            "a whole-crate exclusion must be recognized whatever the key spelling: {entry:?}"
        );
    }
}

/// Verifies that every committed exclusion carries an adjacent reason.
#[test]
fn every_committed_exclusion_is_justified() {
    let offences = unjustified_exclusions(DYLINT);
    assert!(
        offences.is_empty(),
        "every excluded_paths entry needs a comment immediately above it; found {offences:?}"
    );
}

/// An exclusion without an adjacent comment is rejected, in either string form.
#[test]
fn an_unjustified_exclusion_is_rejected() {
    for entry in [
        "excluded_paths = [\"crate::legacy_io\"]\n",
        "excluded_paths = ['crate::legacy_io']\n",
        "excluded_paths = [ \"crate::legacy_io\" ]\n",
        "excluded_paths = [\n  \"crate::legacy_io\",\n]\n",
        "excluded_paths = [\n  'crate::legacy_io',\n]\n",
        "excluded_paths = [\"crate::legacy_io\",\n]\n",
        "\"excluded_paths\" = [\"crate::legacy_io\"]\n",
        "'excluded_paths' = ['crate::legacy_io']\n",
        "excluded_paths = [ # temporary [legacy]\n  \"crate::legacy_io\",\n]\n",
        "excluded_paths = [\n  \"crate::legacy_io\"]\n",
        "excluded_paths = [\n  # ok\n  \"crate::legacy_io\",\n  \"crate::other\"]\n",
    ] {
        assert!(
            !unjustified_exclusions(entry).is_empty(),
            "an exclusion without a preceding comment must be rejected: {entry:?}"
        );
    }
}

/// A documented exclusion is accepted, whatever the string form or layout.
#[test]
fn a_justified_exclusion_is_accepted() {
    for entry in [
        "# The ambient probe cannot be capability-scoped.\nexcluded_paths = [\"crate::legacy_io\"]\n",
        "# The ambient probe cannot be capability-scoped.\nexcluded_paths = ['crate::legacy_io']\n",
        "excluded_paths = [\n  # The ambient probe cannot be capability-scoped.\n  \"crate::legacy_io\",\n]\n",
        "# The ambient probe cannot be capability-scoped.\nexcluded_paths = [\"crate::legacy_io\",\n]\n",
    ] {
        assert!(
            unjustified_exclusions(entry).is_empty(),
            "a justified exclusion must be accepted: {entry:?}"
        );
    }
}

/// An empty array declares no exclusion and needs no reason.
#[test]
fn an_empty_exclusion_list_is_accepted() {
    assert!(
        unjustified_exclusions("excluded_paths = []\n").is_empty(),
        "an empty excluded_paths array must not need a justification"
    );
}

/// A mention of the key in prose is not an active exclusion.
#[test]
fn a_commented_exclusion_is_ignored() {
    let document = "# excluded_paths = [\"crate::legacy_io\"]\nexcluded_crates = []\n";
    assert!(
        unjustified_exclusions(document).is_empty(),
        "a commented exclusion is prose, not policy"
    );
    assert!(
        active_lines(document).contains(&"excluded_crates = []"),
        "the active-line reader must keep the active declaration"
    );
}
