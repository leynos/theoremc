//! Compile-pass fixture for equivalent theorem action declarations.

use theoremc_macros::theorem_file;

mod theorem_actions {
    #[expect(non_snake_case, reason = "theorem action exports use mangled identifiers")]
    pub(crate) fn account__deposit__h05158894bfb4(account: u64) -> bool {
        account > 0
    }
}

theorem_file!("tests/expand/equivalent_action_signatures.theorem");

fn main() {}
