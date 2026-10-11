//! Helpers for validating the optional Kani harness-listing probe.

/// Identifies errors that mean the installed Kani toolchain cannot run here.
pub(crate) fn is_unusable_environment(error: &str) -> bool {
    error.contains("error while loading shared libraries")
        || error.contains("cannot open shared object file")
        || error.contains("Broken pipe")
        || error.contains("unexpected output from `rustc --version`: \"\"")
}

/// Checks that Kani listed only the generated theorem harness.
pub(crate) fn is_expected_single_harness_listing(
    output: &str,
    expected_harness_identifier: &str,
) -> bool {
    let has_one_total_harness = output.lines().any(|line| {
        let mut cells = line
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty());
        cells.next() == Some("Total") && cells.next() == Some("1") && cells.next().is_none()
    });
    output.contains("Standard Harnesses (#[kani::proof]):")
        && output.contains("No contracts or contract harnesses found.")
        && output.contains(expected_harness_identifier)
        && has_one_total_harness
}

#[cfg(test)]
mod tests {
    //! Tests for distinguishing unusable Kani installations from harness failures.

    use rstest::rstest;

    use super::{is_expected_single_harness_listing, is_unusable_environment};

    #[rstest]
    #[case("error while loading shared libraries: libfoo.so", true)]
    #[case("cannot open shared object file", true)]
    #[case("Broken pipe", true)]
    #[case("unexpected output from `rustc --version`: \"\"", true)]
    #[case("expected generated harness was not listed", false)]
    fn classifies_unusable_kani_environment_errors(#[case] error: &str, #[case] expected: bool) {
        assert_eq!(is_unusable_environment(error), expected);
    }

    #[test]
    fn rejects_a_listing_without_the_expected_harness() {
        assert!(!is_expected_single_harness_listing(
            "Standard Harnesses (#[kani::proof]):",
            "theorem__smoke"
        ));
    }

    #[test]
    fn accepts_a_listing_with_only_the_expected_harness() {
        let output = concat!(
            "Standard Harnesses (#[kani::proof]):\n",
            "theorem__smoke\n",
            "No contracts or contract harnesses found.\n",
            "| Total | 1 |\n",
        );

        assert!(is_expected_single_harness_listing(output, "theorem__smoke"));
    }

    #[test]
    fn rejects_a_listing_with_multiple_harnesses() {
        let output = concat!(
            "Standard Harnesses (#[kani::proof]):\n",
            "theorem__smoke\n",
            "theorem__other\n",
            "No contracts or contract harnesses found.\n",
            "| Total | 2 |\n",
        );

        assert!(!is_expected_single_harness_listing(
            output,
            "theorem__smoke"
        ));
    }
}
