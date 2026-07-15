//! Path string formatting helpers for compile-time theorem tooling.
//!
//! These helpers keep path separator normalization and manifest-string
//! escaping consistent between proc-macro expansion and integration-test
//! fixture crate generation.

/// Normalizes path separators to forward slashes.
///
/// This preserves every non-separator character exactly and replaces each
/// backslash (`\`) with a forward slash (`/`). The helper works on strings
/// rather than filesystem paths because macro literals and Cargo manifest
/// fragments are string formats, not host filesystem operations.
///
/// # Examples
///
/// ```
/// use theoremc_core::path_format::normalize_path_separators;
///
/// assert_eq!(
///     normalize_path_separators(r"theorems\nested\proof.theorem"),
///     "theorems/nested/proof.theorem",
/// );
/// ```
#[must_use]
pub fn normalize_path_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Escapes a value for inclusion in a TOML basic string.
///
/// This helper escapes backslashes, double quotes, and prohibited control
/// characters. It does not add the surrounding quote characters.
///
/// # Examples
///
/// ```
/// use theoremc_core::path_format::toml_basic_string_value;
///
/// assert_eq!(
///     toml_basic_string_value(r#"C:\work\"theoremc""#),
///     r#"C:\\work\\\"theoremc\""#,
/// );
/// ```
#[must_use]
pub fn toml_basic_string_value(value: &str) -> String {
    value
        .chars()
        .map(|input_character| match input_character {
            '\\' => "\\\\".to_owned(),
            '"' => "\\\"".to_owned(),
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            control_character if control_character.is_control() => {
                format!("\\u{:04X}", control_character as u32)
            }
            character => character.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    //! Tests for path string formatting helpers.

    use super::{normalize_path_separators, toml_basic_string_value};
    use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};
    use rstest::rstest;

    #[rstest]
    #[case::unchanged("theorems/proof.theorem", "theorems/proof.theorem")]
    #[case::windows(r"theorems\nested\proof.theorem", "theorems/nested/proof.theorem")]
    fn normalize_path_separators_examples(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(normalize_path_separators(input), expected);
    }

    #[rstest]
    #[case::plain("theoremc", "theoremc")]
    #[case::backslash(r"C:\work\theoremc", r"C:\\work\\theoremc")]
    #[case::quote(r#"/work/"theoremc""#, r#"/work/\"theoremc\""#)]
    #[case::line_breaks("one\ntwo\rthree\tfour", r"one\ntwo\rthree\tfour")]
    fn toml_basic_string_value_examples(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(toml_basic_string_value(input), expected);
    }

    #[test]
    fn toml_basic_string_value_escapes_all_prohibited_control_characters() {
        let input = (0_u32..=0x1f)
            .chain(0x7f..=0x9f)
            .map(|code_point| char::from_u32(code_point).expect("control code point is valid"))
            .collect::<String>();
        let expected = (0_u32..=0x1f)
            .chain(0x7f..=0x9f)
            .map(|code_point| match code_point {
                0x09 => r"\t".to_owned(),
                0x0a => r"\n".to_owned(),
                0x0d => r"\r".to_owned(),
                _ => format!(r"\u{code_point:04X}"),
            })
            .collect::<String>();

        assert_eq!(toml_basic_string_value(&input), expected);
    }

    proptest! {
        /// Normalization replaces every backslash and preserves all other
        /// characters.
        #[test]
        fn normalize_path_separators_replaces_only_backslashes(input in any::<String>()) {
            let normalized = normalize_path_separators(&input);
            let expected = input
                .chars()
                .map(|character| if character == '\\' { '/' } else { character })
                .collect::<String>();

            prop_assert!(!normalized.contains('\\'));
            prop_assert_eq!(normalized, expected);
        }
    }
}
