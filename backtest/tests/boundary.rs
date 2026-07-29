use serde_json::Value;
use std::{path::PathBuf, process::Command};

#[test]
fn package_has_no_recreate_path_dependencies() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let package = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["name"] == "recreate-backtest")
        .unwrap();
    for dependency in package["dependencies"].as_array().unwrap() {
        assert!(
            dependency["path"].is_null(),
            "path dependency is forbidden: {dependency}"
        );
        assert!(
            !dependency["name"]
                .as_str()
                .unwrap_or_default()
                .starts_with("recreate"),
            "Recreate dependency is forbidden: {dependency}"
        );
    }
}

#[test]
fn root_manifest_does_not_include_backtest() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Cargo.toml");
    let value = std::fs::read_to_string(root).unwrap();
    assert!(!value.contains("\"backtest\""));
    assert!(!value.contains("backtest/"));
}

