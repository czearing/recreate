use crate::digest;
use serde::Serialize;
use std::{path::Path, process::Command, time::Instant};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEvidence {
    pub executable: String,
    pub arguments: Vec<String>,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub elapsed_ms: u128,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(executable: &Path, arguments: &[String]) -> anyhow::Result<ProcessEvidence> {
    anyhow::ensure!(
        executable.is_file(),
        "selected Recreate binary does not exist"
    );
    let started = Instant::now();
    let output = Command::new(executable).args(arguments).output()?;
    Ok(ProcessEvidence {
        executable: executable.display().to_string(),
        arguments: arguments.to_vec(),
        exit_code: output.status.code(),
        success: output.status.success(),
        elapsed_ms: started.elapsed().as_millis(),
        stdout_sha256: digest::bytes(&output.stdout),
        stderr_sha256: digest::bytes(&output.stderr),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
