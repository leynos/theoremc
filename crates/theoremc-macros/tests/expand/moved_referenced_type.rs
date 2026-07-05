//! Compile-fail fixture for moved referenced action parameter types.

use theoremc_macros::theorem_file;

pub mod new {
    pub struct DepositCommand;
}

pub struct DepositOutcome;

theorem_file!("tests/expand/moved_referenced_type.theorem");

fn main() {}
