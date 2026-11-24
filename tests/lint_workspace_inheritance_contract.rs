//! Contract test for workspace-wide lint inheritance.
//!
//! `make lint` only reaches a crate that Cargo knows about, and the shared lint
//! policy only binds a crate that opts into it. Two omissions are therefore
//! invisible to every other gate: a new crate directory that nobody lists in
//! `[workspace] members`, and a member that keeps its own lint table instead of
//! inheriting `[workspace.lints]`. Either way the crate compiles and tests,
//! while escaping the policy the issue #98 baseline defines.
//!
//! The readers and verifiers live in
//! `tests/lint_workspace_inheritance_support/mod.rs`; this file states the
//! contract and its cases.
//!
//! Temporary fixture manifests are excluded: they are written by tests into
//! scratch directories, never committed, and never injected into the
//! `members` list. See `tests/common/fixture_crate.rs` for the manifests the
//! fixture helpers copy.

#[path = "lint_workspace_inheritance_support/mod.rs"]
mod lint_workspace_inheritance_support;

use lint_workspace_inheritance_support::{
    Repository, array_items, is_workspace_opt_in, member_manifest_path, package_directories,
    resolved_members, undeclared_crates, verify_member_manifest, verify_root_manifest,
    workspace_members,
};

/// Verifies the whole inheritance contract over the committed manifests.
#[test]
fn every_workspace_package_inherits_the_workspace_lint_tables() {
    let repository = Repository::open().expect("the repository root must be readable");
    let root_manifest = repository
        .read("Cargo.toml")
        .unwrap_or_else(|error| panic!("{error}"));
    if let Err(report) = verify_root_manifest(&root_manifest) {
        panic!("the root manifest must hold the lint inheritance contract: {report}");
    }
    let members = workspace_members(&repository).expect("the workspace members must be readable");
    let packages = package_directories(&repository).unwrap_or_else(|error| panic!("{error}"));
    let undeclared = undeclared_crates(&packages, &members);
    assert!(
        undeclared.is_empty(),
        "every crate directory must be a workspace member, or no gate reaches it; found {undeclared:?}"
    );
    for member in resolved_members(&members, &packages) {
        let path = member_manifest_path(&member);
        let manifest = repository
            .read(&path)
            .unwrap_or_else(|error| panic!("{error}"));
        if let Err(report) = verify_member_manifest(&path, &manifest) {
            panic!("{member} must inherit the workspace lint policy: {report}");
        }
    }
}

/// A member that defines its own Clippy table does not inherit.
#[test]
fn a_local_lint_table_is_rejected() {
    let manifest = "[package]\nname = \"member\"\n\n[lints]\nworkspace = true\n\n[lints.clippy]\npedantic = \"warn\"\n";
    let report = verify_member_manifest("crates/member/Cargo.toml", manifest)
        .expect_err("a local [lints.clippy] table must be rejected");
    assert!(
        report.contains("lints.clippy"),
        "the report must name the offending table; found {report:?}"
    );
}

/// A member that never opts in does not inherit either.
#[test]
fn a_missing_opt_in_is_rejected() {
    let manifest = "[package]\nname = \"member\"\n";
    let report = verify_member_manifest("crates/member/Cargo.toml", manifest)
        .expect_err("a member without `[lints] workspace = true` must be rejected");
    assert!(
        report.contains("workspace = true"),
        "the report must name the missing opt-in; found {report:?}"
    );
}

/// A member with the opt-in and no local table satisfies the contract.
#[test]
fn an_inheriting_member_is_accepted() {
    let manifest = "[package]\nname = \"member\"\n\n[lints]\nworkspace = true\n";
    if let Err(report) = verify_member_manifest("crates/member/Cargo.toml", manifest) {
        panic!("a member that inherits every table must be accepted: {report}");
    }
}

/// The `members` reader returns every quoted item, across line breaks.
#[test]
fn members_are_read_across_line_breaks() {
    let manifest =
        "[workspace]\nmembers = [\n    \"crates/one\",\n    \"crates/two\",\n]\nresolver = \"3\"\n";
    assert_eq!(
        array_items(manifest, "members"),
        Some(vec!["crates/one".to_owned(), "crates/two".to_owned()]),
        "every quoted member must be read, whatever the line breaks"
    );
    assert_eq!(
        array_items(manifest, "package"),
        None,
        "an absent key must yield None rather than an empty list"
    );
    let commented = "[workspace]\nmembers = [\n    \"crates/one\",\n    # \"crates/two\",\n]\n";
    assert_eq!(
        array_items(commented, "members"),
        Some(vec!["crates/one".to_owned()]),
        "a commented path must not count as a member"
    );
    let tight = "[workspace]\nmembers=[\"crates/one\"]\n";
    assert_eq!(
        array_items(tight, "members"),
        Some(vec!["crates/one".to_owned()]),
        "TOML ignores whitespace around the assignment"
    );
    let quoted_key = "[workspace]\n\"members\" = [\"crates/one\"]\n";
    assert_eq!(
        array_items(quoted_key, "members"),
        Some(vec!["crates/one".to_owned()]),
        "a quoted key is the same declaration, and its name is not an item"
    );
}

/// A literal-quoted member is registered like a basic-quoted one.
#[test]
fn a_literal_quoted_member_is_registered() {
    let manifest = "[workspace]\nmembers = [\n    'crates/one',\n]\n";
    assert_eq!(
        array_items(manifest, "members"),
        Some(vec!["crates/one".to_owned()]),
        "a TOML literal string must be read as a member"
    );
}

/// The opt-in is accepted whatever the whitespace, and nothing else is.
#[test]
fn the_opt_in_is_read_by_its_meaning() {
    for body in [
        "workspace = true\n",
        "workspace=true\n",
        "workspace   =   true\n",
        "workspace = true  # inherit the shared policy\n",
        "# inherit the shared policy\nworkspace = true\n",
    ] {
        assert!(
            is_workspace_opt_in(body),
            "a valid opt-in must be accepted: {body:?}"
        );
    }
    for body in [
        "workspace = false\n",
        "workspace = true\npedantic = \"warn\"\n",
        "",
    ] {
        assert!(
            !is_workspace_opt_in(body),
            "anything but the opt-in alone must be rejected: {body:?}"
        );
    }
}

/// A crate directory the workspace does not list is reported.
#[test]
fn an_undeclared_crate_is_reported() {
    let members = vec!["crates/declared".to_owned()];
    let packages = vec!["crates/declared".to_owned(), "crates/forgotten".to_owned()];
    assert_eq!(
        undeclared_crates(&packages, &members),
        vec!["crates/forgotten".to_owned()],
        "a crate directory missing from `members` must be reported"
    );
    let declared_only = vec!["crates/declared".to_owned()];
    assert!(
        undeclared_crates(&declared_only, &members).is_empty(),
        "a declared crate directory must not be reported"
    );
}

/// A glob member covers the crate directories it matches, and no others.
#[test]
fn a_glob_member_covers_its_matches() {
    let directories = vec![
        "crates/one".to_owned(),
        "crates/two".to_owned(),
        "tools/helper".to_owned(),
    ];
    let glob = vec!["crates/*".to_owned(), "tools/*".to_owned()];
    assert!(
        undeclared_crates(&directories, &glob).is_empty(),
        "the glob members must cover every package directory"
    );
    assert_eq!(
        resolved_members(&glob, &directories),
        directories,
        "a glob member must expand to the directories it matches"
    );
    let narrower = vec!["crates/one".to_owned()];
    assert_eq!(
        undeclared_crates(&directories, &narrower),
        vec!["crates/two".to_owned(), "tools/helper".to_owned()],
        "a literal member must not cover its siblings"
    );
    let prefix = vec!["crates/t*".to_owned(), "tools/*".to_owned()];
    assert_eq!(
        undeclared_crates(&directories, &prefix),
        vec!["crates/one".to_owned()],
        "a segment glob must match a prefix, as Cargo's matching does"
    );
    // `*w*` must require the literal `w`, so `crates/two` matches and
    // `crates/one` does not.
    let middle = vec!["crates/*w*".to_owned(), "tools/*".to_owned()];
    assert_eq!(
        undeclared_crates(&directories, &middle),
        vec!["crates/one".to_owned()],
        "a literal between wildcards must still have to appear"
    );
    // A nested package is only covered by a nested pattern, so `two` and
    // `group/member` must both be reported as undeclared.
    let grouped = vec![
        "crates/one".to_owned(),
        "crates/two".to_owned(),
        "tools/*".to_owned(),
    ];
    let nested = vec![
        "crates/one".to_owned(),
        "crates/group/member".to_owned(),
        "tools/helper".to_owned(),
    ];
    assert_eq!(
        undeclared_crates(&nested, &grouped),
        vec!["crates/group/member".to_owned()],
        "a nested package must not be covered by a shallow member"
    );
    let deep_glob = vec!["crates/group/*".to_owned()];
    assert!(
        undeclared_crates(&["crates/group/member".to_owned()], &deep_glob).is_empty(),
        "a nested pattern must cover a nested package"
    );
}

/// The root manifest must populate all three workspace lint tables.
#[test]
fn an_empty_workspace_lint_table_is_rejected() {
    let manifest = concat!(
        "[workspace]\n",
        "members = [\"crates/one\"]\n",
        "\n",
        "[workspace.lints.clippy]\n",
        "\n",
        "[workspace.lints.rust]\n",
        "missing_docs = \"deny\"\n",
        "\n",
        "[workspace.lints.rustdoc]\n",
        "bare_urls = \"deny\"\n",
        "\n",
        "[lints]\n",
        "workspace = true\n",
    );
    let report = verify_root_manifest(manifest)
        .expect_err("an empty [workspace.lints.clippy] table must be rejected");
    assert!(
        report.contains("workspace.lints.clippy"),
        "the report must name the empty table; found {report:?}"
    );
}

/// The root package must opt into the workspace lints like every member.
#[test]
fn a_root_without_the_opt_in_is_rejected() {
    let manifest = "[workspace]\nmembers = [\"crates/one\"]\n";
    let report =
        verify_root_manifest(manifest).expect_err("a root without the opt-in must be rejected");
    assert!(
        report.contains("workspace = true"),
        "the report must name the missing opt-in; found {report:?}"
    );
}
