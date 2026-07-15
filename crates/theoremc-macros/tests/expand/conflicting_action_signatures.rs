//! Compile-fail fixture for conflicting theorem action declarations.

use theoremc_macros::theorem_file;

theorem_file!("tests/expand/conflicting_action_signatures.theorem");

fn main() {}
