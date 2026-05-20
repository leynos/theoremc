//! Compile-fail fixture: theorem files without Kani evidence trigger a
//! compile_error! diagnostic emitted by theorem_file!.
//!
//! The referenced .theorem file declares only placeholder non-Kani evidence.
//! The paired .stderr snapshot asserts the macro-specific diagnostic text and
//! call-site span.

theoremc_macros::theorem_file!("tests/expand/missing_kani_evidence.theorem");

fn main() {}
