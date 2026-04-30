//! Compile-fail fixture: referencing a non-existent theorem file triggers a
//! compile_error! diagnostic emitted by theorem_file!.
//!
//! The missing path is intentional; the paired .stderr snapshot asserts the
//! IO diagnostic text and the macro call-site span.

theoremc_macros::theorem_file!("tests/expand/this_file_does_not_exist.theorem");

fn main() {}
