//! Executable entry point for the `cargo theorem` application.

use std::process::ExitCode;

fn main() -> ExitCode {
    cargo_theorem::run(std::env::args_os())
}
