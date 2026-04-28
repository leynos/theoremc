//! Compile-pass fixture for valid `theorem_file!` expansion.

use theoremc_macros::theorem_file;

theorem_file!("tests/expand/valid_theorem.theorem");

/// Assert the generated module contains the expected callable harness.
///
/// This is a compile-time check: if the path, visibility, or generated
/// signature is wrong, this fixture will not compile.
fn _assert_structure() {
    let _: fn() = __theoremc__file__tests_expand_valid_theorem__c972e62265e3::kani::theorem__smoke_expansion__h19a3b63a856a;
}

fn main() {
    _assert_structure();
}
