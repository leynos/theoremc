//! Workspace Cargo locations for isolated nested fixture packages.

use camino::{Utf8Path, Utf8PathBuf};

/// Returns the workspace Cargo target used by fixture packages.
///
/// Relative `CARGO_TARGET_DIR` values are resolved against the workspace root,
/// rather than against each temporary fixture.
///
/// # Examples
///
/// ```
/// let target = test_helpers::fixture_target_dir();
/// assert!(target.is_absolute());
/// ```
#[must_use]
pub fn fixture_target_dir() -> Utf8PathBuf {
    fixture_target_root()
}

pub(super) fn fixture_target_root() -> Utf8PathBuf {
    let workspace = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map_or_else(Utf8PathBuf::new, Utf8Path::to_path_buf);
    target_root_for(
        &workspace,
        std::env::var("CARGO_TARGET_DIR").ok().as_deref(),
    )
}

fn target_root_for(workspace: &Utf8Path, target: Option<&str>) -> Utf8PathBuf {
    workspace.join(target.unwrap_or("target"))
}

#[cfg(test)]
mod tests {
    //! Target location contracts without process environment mutation.

    use super::target_root_for;
    use camino::Utf8Path;

    #[test]
    fn target_overrides_resolve_from_workspace_instead_of_fixture_source() {
        let workspace = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let absolute_target = workspace.join("external-builds");
        assert_eq!(target_root_for(workspace, None), workspace.join("target"));
        assert_eq!(
            target_root_for(workspace, Some("custom")),
            workspace.join("custom")
        );
        assert_eq!(
            target_root_for(workspace, Some(absolute_target.as_str())),
            absolute_target
        );
    }
}
