//! Tests for the shared path-context formatter.

use std::io;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::path_error;

#[test]
fn names_the_operation_the_path_and_the_source_error() {
    let error = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");

    assert_eq!(
        path_error("writing", Utf8Path::new("/tmp/fixture/Cargo.toml"), &error),
        "writing /tmp/fixture/Cargo.toml: access denied",
        "context must name the operation, the path and the underlying error",
    );
}

#[test]
fn keeps_a_real_filesystem_errors_own_text() {
    let scratch = tempfile::tempdir().expect("create a scratch directory");
    let absent = Utf8Path::from_path(scratch.path())
        .expect("a temporary directory path should be UTF-8")
        .join("absent");
    let dir = Dir::open_ambient_dir(
        Utf8Path::from_path(scratch.path()).expect("a scratch path should be UTF-8"),
        ambient_authority(),
    )
    .expect("open the scratch directory");
    let error = dir
        .read_to_string("absent")
        .err()
        .expect("reading an absent file must fail");

    let message = path_error("reading", &absent, &error);

    assert!(
        message.contains(&error.to_string()),
        "the original error text must survive so operators can recognise it: {message}",
    );
    assert!(
        message.starts_with(&format!("reading {absent}: ")),
        "the message must open with the operation and path: {message}",
    );
}
