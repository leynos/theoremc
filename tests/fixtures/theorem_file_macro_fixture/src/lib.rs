//! Empty library root for a fixture-template package.
//!
//! Cargo refuses a package that has no target, so the template needs a
//! target it never compiles. Nothing builds this crate: only the `Cargo.lock`
//! beside it is used, as the seed for a generated fixture crate.