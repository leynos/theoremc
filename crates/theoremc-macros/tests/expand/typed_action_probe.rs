//! Compile-pass fixture for typed action probe generation.

use theoremc_macros::theorem_file;

mod theorem_actions {
    #[expect(non_snake_case, reason = "theorem action exports use mangled identifiers")]
    pub(crate) fn account__deposit__h05158894bfb4(account: u64, amount: u32) -> bool {
        account >= amount as u64
    }
}

theorem_file!("tests/expand/typed_action_probe.theorem");

fn main() {}
