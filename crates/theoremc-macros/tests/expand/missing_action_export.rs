//! Compile-fail fixture for missing theorem action exports.

use theoremc_macros::theorem_file;

mod theorem_actions {}

theorem_file!("tests/expand/missing_action_export.theorem");

fn main() {}
