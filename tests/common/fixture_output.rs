//! Selects a fixture's generated output from Cargo's structured messages.
//!
//! A shared target may contain many historical build-script outputs. Correlate
//! the exact root manifest's compiler artefact with its execution message;
//! directory enumeration could accidentally accept another fixture's suite.

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

pub(super) fn root_build_out_dir(
    stdout: &str,
    manifest_path: &Utf8Path,
) -> Result<Option<Utf8PathBuf>, String> {
    let messages: Vec<Value> = stdout
        .lines()
        // Cargo -vv interleaves build-script stdout with JSON messages.
        .filter(|line| line.starts_with('{'))
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .map_err(|error| format!("invalid Cargo JSON output: {error}"))?;
    let package_id = messages.iter().find_map(|message| {
        (message.get("reason")?.as_str()? == "compiler-artifact"
            && message.get("manifest_path")?.as_str()? == manifest_path.as_str())
        .then(|| message.get("package_id").and_then(Value::as_str))
        .flatten()
    });
    Ok(package_id.and_then(|root| {
        messages.iter().find_map(|message| {
            (message.get("reason")?.as_str()? == "build-script-executed"
                && message.get("package_id")?.as_str()? == root)
                .then(|| {
                    message
                        .get("out_dir")
                        .and_then(Value::as_str)
                        .map(Utf8PathBuf::from)
                })
                .flatten()
        })
    }))
}

#[cfg(test)]
mod tests {
    //! Regression coverage for selecting root output in a shared target.

    use super::root_build_out_dir;
    use camino::{Utf8Path, Utf8PathBuf};
    use serde_json::json;

    #[test]
    fn selects_exact_manifest_despite_similar_names_and_message_order() -> Result<(), String> {
        let messages = [
            json!({"reason":"build-script-executed", "package_id":"other#fixture@0.1.0", "out_dir":"/target/stale"}),
            json!({"reason":"build-script-executed", "package_id":"file:///current#0.1.0", "out_dir":"/target/current"}),
            json!({"reason":"compiler-artifact", "manifest_path":"/other/Cargo.toml", "package_id":"other#fixture@0.1.0"}),
            json!({"reason":"compiler-artifact", "manifest_path":"/current/Cargo.toml", "package_id":"file:///current#0.1.0"}),
        ];
        let stdout = messages
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let actual = root_build_out_dir(&stdout, Utf8Path::new("/current/Cargo.toml"))?;
        let expected = Some(Utf8PathBuf::from("/target/current"));
        if actual != expected {
            return Err(format!(
                "expected exact root output {expected:?}, got {actual:?}"
            ));
        }
        let missing = root_build_out_dir(&stdout, Utf8Path::new("/missing/Cargo.toml"))?;
        if missing.is_some() {
            return Err(format!(
                "missing manifest must not select another fixture's output: {missing:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn selects_root_output_amid_verbose_build_script_lines() -> Result<(), String> {
        let artifact = json!({
            "reason": "compiler-artifact",
            "manifest_path": "/fixture/Cargo.toml",
            "package_id": "file:///fixture#0.1.0",
        })
        .to_string();
        let execution = json!({
            "reason": "build-script-executed",
            "package_id": "file:///fixture#0.1.0",
            "out_dir": "/target/fixture",
        })
        .to_string();
        let stdout = [
            "plain build output\n",
            &artifact,
            "[fixture 0.1.0] cargo::rerun-if-changed=theorems\n",
            &execution,
            "\nfinished build output",
        ]
        .join("\n");
        let actual = root_build_out_dir(&stdout, Utf8Path::new("/fixture/Cargo.toml"))?;
        let expected = Some(Utf8PathBuf::from("/target/fixture"));
        if actual != expected {
            return Err(format!("expected root output {expected:?}, got {actual:?}"));
        }
        Ok(())
    }

    #[test]
    fn ignores_plain_output_without_a_root_artifact() {
        assert_eq!(
            root_build_out_dir(
                "plain build output\n\n[fixture 0.1.0] cargo::rerun-if-changed=theorems\n",
                Utf8Path::new("/fixture/Cargo.toml"),
            )
            .expect("non-JSON output should be ignored"),
            None,
        );
    }

    #[test]
    fn rejects_malformed_json_object_messages() {
        assert!(root_build_out_dir("{not json", Utf8Path::new("/fixture/Cargo.toml")).is_err());
    }
}
