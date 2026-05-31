//! Compile-fail fixture for typed action probe signature drift.

use theoremc_macros::theorem_file;

mod theorem_actions {
    #[expect(non_snake_case, reason = "theorem action exports use mangled identifiers")]
    pub(crate) fn account__deposit__h05158894bfb4(account: u64, amount: u32) -> u32 {
        account.saturating_add(amount as u64) as u32
    }
}

theorem_file!("tests/expand/signature_drift.theorem");

fn main() {}
