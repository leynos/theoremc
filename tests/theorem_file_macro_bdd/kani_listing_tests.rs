//! Guards exact Kani summary-row parsing against prefix matches.

use rstest::rstest;

use super::is_expected_single_harness_listing;

#[rstest]
#[case("| Total | 1 |", true)]
#[case("| Total | 10 |", false)]
#[case("| Total | 1 | extra |", false)]
fn accepts_only_an_exact_single_harness_total_row(#[case] total_row: &str, #[case] expected: bool) {
    let output = format!(
        concat!(
            "Standard Harnesses (#[kani::proof]):\n",
            "theorem__smoke\n",
            "No contracts or contract harnesses found.\n",
            "{total_row}\n",
        ),
        total_row = total_row,
    );

    assert_eq!(
        is_expected_single_harness_listing(&output, "theorem__smoke"),
        expected
    );
}
