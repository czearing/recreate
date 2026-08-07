use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const DEPENDENCIES: &[&str] = &["vite@8.1.2", "react@19.2.0", "react-dom@19.2.0"];

/// A private manifest is what makes a directory a Node package root.
///
/// npm resolves its install prefix by walking up to the nearest `package.json`.
/// Without one of its own the runtime is not a root, so an install there is
/// redirected into the enclosing repository and reports success while leaving
/// the runtime empty.
const MANIFEST: &str = r#"{"name":"recreate-build-runtime","private":true,"version":"0.0.0"}
"#;

/// Builds a generated project with the shared build runtime.
pub fn build(root: &Path) -> Result<()> {
    let runtime = runtime_root();
    let vite = provision(&runtime)?;
    link_dependencies(root, &runtime)?;
    let status = Command::new(&vite)
        .arg("build")
        .arg(".")
        .current_dir(root)
        .env("CI", "1")
        .status()
        .with_context(|| format!("run {}", vite.display()))?;
    if !status.success() {
        bail!("shared Vite build failed with {status}");
    }
    if !root.join("dist/index.html").exists() {
        bail!("generated Vite build did not create dist/index.html");
    }
    Ok(())
}

/// Installs the shared dependencies once and returns the Vite executable.
fn provision(runtime: &Path) -> Result<PathBuf> {
    if let Ok(vite) = installed_executable(runtime) {
        return Ok(vite);
    }
    prepare(runtime)?;
    let mut arguments = vec![
        "install",
        "--save-exact",
        "--ignore-scripts",
        "--no-audit",
        "--no-fund",
    ];
    arguments.extend_from_slice(DEPENDENCIES);
    run_npm(runtime, &arguments)?;
    installed_executable(runtime)
}

/// Makes `runtime` a package root before anything installs into it.
fn prepare(runtime: &Path) -> Result<()> {
    fs::create_dir_all(runtime)
        .with_context(|| format!("create build runtime {}", runtime.display()))?;
    fs::write(runtime.join("package.json"), MANIFEST)
        .with_context(|| format!("mark {} as a package root", runtime.display()))?;
    Ok(())
}

/// Resolves the installed Vite executable, reporting its absence rather than
/// letting the caller execute a path that does not exist.
fn installed_executable(runtime: &Path) -> Result<PathBuf> {
    let vite = runtime
        .join("node_modules")
        .join(".bin")
        .join(if cfg!(windows) { "vite.cmd" } else { "vite" });
    if !vite.exists() {
        bail!("no vite executable at {}", vite.display());
    }
    Ok(vite)
}

fn runtime_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release-gate")
        .join("runtime")
}

fn run_npm(root: &Path, args: &[&str]) -> Result<()> {
    let executable = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(executable)
        .args(args)
        .current_dir(root)
        .env("CI", "1")
        .status()
        .with_context(|| format!("run npm {}", args.join(" ")))?;
    if !status.success() {
        bail!("npm {} failed with {status}", args.join(" "));
    }
    Ok(())
}

#[cfg(unix)]
fn link_dependencies(root: &Path, runtime: &Path) -> Result<()> {
    std::os::unix::fs::symlink(runtime.join("node_modules"), root.join("node_modules"))?;
    Ok(())
}

#[cfg(windows)]
fn link_dependencies(root: &Path, runtime: &Path) -> Result<()> {
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(root.join("node_modules"))
        .arg(runtime.join("node_modules"))
        .status()
        .context("link shared Node dependencies")?;
    if !status.success() {
        bail!("link shared Node dependencies failed with {status}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
