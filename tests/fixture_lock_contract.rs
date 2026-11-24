//! Contract tests for the committed fixture-template locks.
//!
//! Each template under `tests/fixtures/` seeds the lock a nested fixture build
//! starts from, and those builds run `--frozen`, so a template that resolves a
//! package the workspace does not is a lookup the fetch step never covers. The
//! nested-cargo rule NC-001 states the same requirement: a fixture lock may
//! resolve only packages the root `Cargo.lock` already resolves, identified by
//! name, version and source. A same-named crate at another version, or one from
//! git rather than crates.io, counts as different.
//!
//! Regenerate both locks with `make fixture-locks` after changing the manifests,
//! the build-support crate or a workspace dependency.
//!
//! These tests read committed text only, so they run in the default nextest
//! group rather than beside the tests that start a compiler.

use std::collections::BTreeSet;

use test_helpers::{LockPackage, lock_packages};

/// The root lock every template must be a subset of.
const ROOT_LOCK: &str = include_str!("../Cargo.lock");

/// A committed fixture template: its lock and the root package it records.
struct Template {
    /// The template's directory, as a reader can find it.
    path: &'static str,
    /// The template's committed lock text.
    lock: &'static str,
    /// The root package entry the lock must hold exactly once.
    root: &'static str,
}

/// Every template a fixture build seeds from.
const TEMPLATES: [Template; 2] = [
    Template {
        path: "tests/fixtures/build_support_fixture",
        lock: include_str!("fixtures/build_support_fixture/Cargo.lock"),
        root: "build_support_fixture",
    },
    Template {
        path: "tests/fixtures/theorem_file_macro_fixture",
        lock: include_str!("fixtures/theorem_file_macro_fixture/Cargo.lock"),
        root: "theorem_file_macro_fixture",
    },
];

/// A package identified the way NC-001 compares it: name, version and source.
type RegistryPackage = (String, String, String);

/// Returns the registry packages of `packages`, excluding path packages.
fn registry_packages(packages: &[LockPackage<'_>]) -> BTreeSet<RegistryPackage> {
    packages
        .iter()
        .filter_map(|package| {
            let source = package.source?;
            Some((
                package.name.to_owned(),
                package.version.to_owned(),
                source.to_owned(),
            ))
        })
        .collect()
}

/// Returns the packages in `fixture` that `root` does not resolve.
fn unaligned<'a>(
    fixture: &'a BTreeSet<RegistryPackage>,
    root: &'a BTreeSet<RegistryPackage>,
) -> Vec<&'a RegistryPackage> {
    fixture.difference(root).collect()
}

/// Every registry package in every template exists at the same name, version
/// and source in the workspace lock.
#[test]
fn every_template_resolves_only_packages_the_workspace_lock_resolves() -> Result<(), String> {
    let root = registry_packages(&lock_packages(ROOT_LOCK)?);
    let mut problems = Vec::new();
    for template in &TEMPLATES {
        let packages = registry_packages(&lock_packages(template.lock)?);
        for (name, version, source) in unaligned(&packages, &root) {
            problems.push(format!(
                "{} resolves {name} {version} from {source}, which the root lock does not; \
                 run `make fixture-locks`",
                template.path
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// Each template records its own root package exactly once, without a source,
/// so the rename a fixture applies has one unambiguous target.
#[test]
fn each_template_names_its_root_package_once() -> Result<(), String> {
    let mut problems = Vec::new();
    for template in &TEMPLATES {
        let packages = lock_packages(template.lock)?;
        let named: Vec<&LockPackage<'_>> = packages
            .iter()
            .filter(|package| package.name == template.root)
            .collect();
        match named.as_slice() {
            [package] if package.source.is_none() => {}
            [package] => problems.push(format!(
                "{} records {} with a source, so it is not the template's root",
                template.path, package.name
            )),
            _ => problems.push(format!(
                "{} names {} {} times; it must name its root exactly once",
                template.path,
                template.root,
                named.len()
            )),
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// A template never names another template's root, so a fixture cannot be
/// seeded with a lock belonging to a different dependency graph.
#[test]
fn no_template_resolves_another_templates_root() -> Result<(), String> {
    let mut problems = Vec::new();
    for template in &TEMPLATES {
        let packages = lock_packages(template.lock)?;
        for other in &TEMPLATES {
            if other.root == template.root {
                continue;
            }
            if packages.iter().any(|package| package.name == other.root) {
                problems.push(format!(
                    "{} resolves {}, which is another template's root",
                    template.path, other.root
                ));
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    //! The comparisons hold for the shapes a template lock can take, including
    //! the drift the contract exists to catch.

    use std::collections::BTreeSet;

    use super::{LockPackage, RegistryPackage, registry_packages, unaligned};

    /// A registry package at one version.
    fn registry<'a>(name: &'a str, version: &'a str) -> LockPackage<'a> {
        LockPackage {
            name,
            version,
            source: Some("registry+https://github.com/rust-lang/crates.io-index"),
        }
    }

    /// Returns one package in the shape the contract compares.
    fn identified(name: &str, version: &str) -> RegistryPackage {
        (
            name.to_owned(),
            version.to_owned(),
            "registry+https://github.com/rust-lang/crates.io-index".to_owned(),
        )
    }

    #[test]
    fn a_version_drift_is_unaligned() {
        let fixture = registry_packages(&[registry("serde", "1.0.220")]);
        let root = registry_packages(&[registry("serde", "1.0.219")]);

        assert_eq!(
            unaligned(&fixture, &root),
            [&identified("serde", "1.0.220")],
            "the same name at another version is a different package",
        );
    }

    #[test]
    fn a_git_source_is_unaligned() {
        let fixture = registry_packages(&[LockPackage {
            name: "serde",
            version: "1.0.219",
            source: Some("git+https://example.invalid/serde"),
        }]);
        let root = registry_packages(&[registry("serde", "1.0.219")]);

        assert_eq!(
            unaligned(&fixture, &root).len(),
            1,
            "the same name and version from another source is a different package",
        );
    }

    #[test]
    fn a_path_package_is_not_compared() {
        let fixture = [LockPackage {
            name: "theoremc-build-support",
            version: "0.1.0",
            source: None,
        }];

        assert_eq!(
            registry_packages(&fixture),
            BTreeSet::new(),
            "a path package has no source to compare",
        );
    }

    #[test]
    fn an_aligned_package_is_not_reported() {
        let fixture = registry_packages(&[registry("serde", "1.0.219")]);
        let root = registry_packages(&[registry("serde", "1.0.219")]);

        assert_eq!(
            unaligned(&fixture, &root),
            Vec::<&RegistryPackage>::new(),
            "a package the workspace resolves is not drift",
        );
    }
}
