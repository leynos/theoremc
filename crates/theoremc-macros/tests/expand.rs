//! trybuild compile-error snapshot tests for the `theorem_file!` proc-macro.

fn stage_fixture(name: &str) {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = crate_root.join("tests/expand").join(name);
    let destination = crate_root
        .join("../../target/tests/trybuild/theoremc-macros/tests/expand")
        .join(name);
    let destination_dir = destination
        .parent()
        .expect("staged trybuild fixture should have a parent directory");
    std::fs::create_dir_all(destination_dir).expect("should create trybuild fixture directory");
    std::fs::copy(&source, &destination).expect("should stage trybuild fixture");
}

#[test]
fn compile_errors() {
    stage_fixture("invalid_theorem.theorem");
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/expand/invalid_theorem.rs");
    t.compile_fail("tests/expand/missing_theorem.rs");
}
