//! Compile-fail fixture: invalid theorem schema triggers a compile_error! diagnostic.
//!
//! The referenced .theorem file contains an empty About field. The paired
//! .stderr snapshot asserts the exact diagnostic text and call-site span.

theoremc_macros::theorem_file!("tests/expand/invalid_theorem.theorem");

fn main() {}
