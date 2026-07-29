use crate::digest;
use anyhow::Context;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

#[derive(Serialize)]
pub struct ProcessEvidence {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub exit_code: i32,
    pub elapsed_ms: u128,
    pub stdout: String,
    pub stderr: String,
    pub executable_hash: String,
}

pub fn run(
    executable: &Path,
    arguments: &[String],
    directory: &Path,
) -> anyhow::Result<ProcessEvidence> {
    fs::create_dir_all(directory)?;
    let started = Instant::now();
    let output = Command::new(executable)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("run {}", executable.display()))?;
    let evidence = ProcessEvidence {
        executable: executable.to_path_buf(),
        arguments: arguments.to_vec(),
        exit_code: output.status.code().unwrap_or(-1),
        elapsed_ms: started.elapsed().as_millis(),
        stdout: String::from_utf8_lossy(&output.stdout).into(),
        stderr: String::from_utf8_lossy(&output.stderr).into(),
        executable_hash: digest::bytes(&fs::read(executable)?),
    };
    fs::write(
        directory.join("recreate-process.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    anyhow::ensure!(
        output.status.success(),
        "Recreate process failed with {}",
        evidence.exit_code
    );
    Ok(evidence)
}

