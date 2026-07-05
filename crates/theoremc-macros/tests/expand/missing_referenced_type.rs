//! Compile-fail fixture for missing referenced type probes.

use theoremc_macros::theorem_file;

pub struct DepositCommand;
pub struct DepositOutcome;

mod theorem_actions {
    pub(crate) fn account__deposit__h05158894bfb4(
        _command: crate::DepositCommand,
    ) -> crate::DepositOutcome {
        crate::DepositOutcome
    }
}

theorem_file!("tests/expand/missing_referenced_type.theorem");

fn main() {}
