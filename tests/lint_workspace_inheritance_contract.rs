//! Contract test for workspace-wide lint inheritance.
//!
//! `make lint` only reaches a crate that Cargo knows about, and the shared lint
//! policy only binds a crate that opts into it. Two omissions are therefore
//! invisible to every other gate: a new crate directory that nobody lists in
//! `[workspace] members`, and a member that keeps its own lint table instead of
//! inheriting `[workspace.lints]`. Either way the crate compiles and tests,
//! while escaping the policy the issue #98 baseline defines.
//!
//! This test reads the manifests as text, following `test_helpers::toml_section`
//! and a line scan rather than adding a TOML parser dependency, and it fails on
//! the omission rather than on whatever the crate later contains.
//!
//! Temporary fixture manifests are excluded: they are written by tests into
//! scratch directories, never committed, and never injected into the
//! `members` list. See `tests/common/fixture_crate.rs` for the manifests the
//! fixture helpers copy.

use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;

/// The repository root, as the manifest directory.
const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The lint tables a member must inherit rather than define for itself.
const LINT_TABLES: [&str; 3] = ["lints.clippy", "lints.rust", "lints.rustdoc"];

/// The lint tables the root manifest must populate.
const WORKSPACE_LINT_TABLES: [&str; 3] = [
    "workspace.lints.clippy",
    "workspace.lints.rust",
    "workspace.lints.rustdoc",
];

/// Extracts the string items of a top-level TOML array.
///
/// The array is written across several lines in the root manifest, so the
/// body is joined before the quoted items are read.
fn array_items(document: &str, key: &str) -> Option<Vec<String>> {
    let header = format!("{key} =");
    let start = document
        .lines()
        .position(|line| line.starts_with(&header))?;
    let mut body = String::new();
    let mut depth = 0usize;
    for line in document.lines().skip(start) {
        depth += line.matches('[').count();
        depth = depth.saturating_sub(line.matches(']').count());
        body.push_str(line);
        body.push(' ');
        if depth == 0 {
            break;
        }
    }
    let items = body
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    Some(items)
}

/// Reports whether a manifest names a table header.
fn declares(document: &str, table: &str) -> bool {
    let header = format!("[{table}]");
    document
        .lines()
        .any(|line| line.trim_end() == header.as_str())
}

/// Provides read access to the repository tree.
struct Repository {
    /// A capability-scoped handle to the repository root.
    dir: Dir,
}

impl Repository {
    /// Opens the repository root.
    ///
    /// # Errors
    ///
    /// Returns the reason when the root cannot be opened.
    fn open() -> Result<Self, String> {
        let dir = Dir::open_ambient_dir(ROOT, ambient_authority())
            .map_err(|error| format!("opening {ROOT}: {error}"))?;
        Ok(Self { dir })
    }

    /// Reads a repository-relative file.
    ///
    /// # Errors
    ///
    /// Returns the reason when the file cannot be read.
    fn read(&self, path: &str) -> Result<String, String> {
        self.dir
            .read_to_string(path)
            .map_err(|error| format!("reading {path}: {error}"))
    }

    /// Returns the names of the directories under `crates`.
    ///
    /// # Errors
    ///
    /// Returns the reason when `crates` cannot be listed.
    fn crate_directories(&self) -> Result<Vec<String>, String> {
        let entries = self
            .dir
            .read_dir("crates")
            .map_err(|error| format!("listing crates: {error}"))?;
        let mut names = Vec::new();
        for entry in entries {
            let item = entry.map_err(|error| format!("reading a crates entry: {error}"))?;
            let name = item
                .file_name()
                .map_err(|error| format!("reading a crates entry name: {error}"))?;
            let is_directory = item
                .file_type()
                .map_err(|error| format!("reading the type of crates/{name}: {error}"))?
                .is_dir();
            if is_directory {
                names.push(name);
            }
        }
        Ok(names)
    }
}

/// Returns the member paths the root manifest lists.
///
/// # Errors
///
/// Returns the reason when the manifest cannot be read or names no members.
fn workspace_members(repository: &Repository) -> Result<Vec<String>, String> {
    let manifest = repository.read("Cargo.toml")?;
    let members = array_items(&manifest, "members")
        .ok_or_else(|| "the root Cargo.toml must declare a `members` array".to_owned())?;
    if members.is_empty() {
        return Err("the root Cargo.toml must list at least one workspace member".to_owned());
    }
    Ok(members)
}

/// Fails unless a manifest opts into the workspace lints.
///
/// # Errors
///
/// Returns the missing opt-in.
fn verify_opt_in(path: &str, manifest: &str) -> Result<(), String> {
    if test_helpers::toml_section(manifest, "lints").as_deref() == Some("workspace = true\n") {
        Ok(())
    } else {
        Err(format!(
            "{path} must declare `[lints]\\nworkspace = true` so the workspace policy applies"
        ))
    }
}

/// Fails unless the root manifest opts into and populates its lint tables.
///
/// # Errors
///
/// Returns the first missing opt-in or empty table.
fn verify_root_manifest(manifest: &str) -> Result<(), String> {
    verify_opt_in("the root Cargo.toml", manifest)?;
    for table in WORKSPACE_LINT_TABLES {
        let body = test_helpers::toml_section(manifest, table)
            .ok_or_else(|| format!("the root Cargo.toml must declare [{table}]"))?;
        let has_rule = body
            .lines()
            .any(|line| !line.trim().is_empty() && !line.trim().starts_with('#'));
        if !has_rule {
            return Err(format!("[{table}] must carry at least one rule"));
        }
    }
    Ok(())
}

/// Fails unless a member manifest opts into the workspace lints.
///
/// # Errors
///
/// Returns the first local lint table, or the missing opt-in.
fn verify_member_manifest(path: &str, manifest: &str) -> Result<(), String> {
    for table in LINT_TABLES {
        if declares(manifest, table) {
            return Err(format!(
                "{path} must inherit lint rules from the workspace, not define [{table}]"
            ));
        }
    }
    verify_opt_in(path, manifest)
}

/// Returns the crate directories the workspace does not list.
///
/// A crate directory that is not a member is compiled by no gate and linted by
/// none either, so the caller treats any entry here as a failure.
fn undeclared_crates(crate_directories: &[String], members: &[String]) -> Vec<String> {
    let declared: Vec<&str> = members.iter().map(String::as_str).collect();
    crate_directories
        .iter()
        .map(|name| format!("crates/{name}"))
        .filter(|path| !declared.contains(&path.as_str()))
        .collect()
}

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
    let undeclared = undeclared_crates(
        &repository
            .crate_directories()
            .unwrap_or_else(|error| panic!("{error}")),
        &members,
    );
    assert!(
        undeclared.is_empty(),
        "every crate directory must be a workspace member, or no gate reaches it; found {undeclared:?}"
    );
    for member in &members {
        let path = Utf8Path::new(member).join("Cargo.toml");
        let manifest = repository
            .read(path.as_str())
            .unwrap_or_else(|error| panic!("{error}"));
        if let Err(report) = verify_member_manifest(path.as_str(), &manifest) {
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
}

/// A crate directory the workspace does not list is reported.
#[test]
fn an_undeclared_crate_is_reported() {
    let directories = vec!["declared".to_owned(), "forgotten".to_owned()];
    let members = vec!["crates/declared".to_owned()];
    assert_eq!(
        undeclared_crates(&directories, &members),
        vec!["crates/forgotten".to_owned()],
        "a crate directory missing from `members` must be reported"
    );
}

/// A declared crate directory is not reported.
#[test]
fn a_declared_crate_is_accepted() {
    let directories = vec!["declared".to_owned()];
    let members = vec!["crates/declared".to_owned()];
    assert!(
        undeclared_crates(&directories, &members).is_empty(),
        "a declared crate directory must not be reported"
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
    let report = verify_root_manifest(manifest)
        .expect_err("a root package without the opt-in must be rejected");
    assert!(
        report.contains("workspace = true"),
        "the report must name the missing opt-in; found {report:?}"
    );
}
