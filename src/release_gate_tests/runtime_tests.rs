use super::*;
use std::fs;

/// npm's prefix rule: an install lands in the nearest ancestor holding a
/// manifest, and only in the starting directory when no ancestor has one.
fn npm_install_prefix(start: &Path) -> PathBuf {
    start
        .ancestors()
        .find(|directory| directory.join("package.json").is_file())
        .unwrap_or(start)
        .to_path_buf()
}

#[test]
fn a_prepared_runtime_is_its_own_package_root() {
    let workspace = tempfile::tempdir().unwrap();
    fs::write(workspace.path().join("package.json"), r#"{"name":"host"}"#).unwrap();
    let runtime = workspace
        .path()
        .join("target")
        .join("release-gate")
        .join("runtime");

    prepare(&runtime).unwrap();

    assert_eq!(
        npm_install_prefix(&runtime),
        runtime,
        "npm would install into the enclosing package and leave the runtime empty"
    );
}

#[test]
fn a_missing_executable_is_reported_instead_of_executed() {
    let runtime = tempfile::tempdir().unwrap();

    let error = installed_executable(runtime.path())
        .unwrap_err()
        .to_string();

    assert!(error.contains("vite"), "{error}");
    assert!(
        error.contains(&runtime.path().display().to_string()),
        "{error}"
    );
}
