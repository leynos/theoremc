//! Compile-fail fixture: zero Kani unwind triggers a compile_error! diagnostic.
//!
//! The referenced .theorem file contains an invalid Evidence.kani.unwind value.
//! The paired .stderr snapshot asserts the exact diagnostic text and call-site
//! span.

theoremc_macros::theorem_file!("tests/expand/zero_unwind.theorem");

fn main() {}
