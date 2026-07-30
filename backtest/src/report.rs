use crate::model::{Report, Status};
use std::{fs, path::Path};

pub fn text(report: &Report) -> String {
    let status = match report.status {
        Status::Pass => "PASS",
        Status::Fail => "FAIL",
        Status::Inconclusive => "INCONCLUSIVE",
        Status::PreparationRequired => "PREPARATION_REQUIRED",
    };
    let scope = report
        .scope
        .as_ref()
        .map(|scope| format!(" FOCUS {}", scope.replace('\n', " ")))
        .unwrap_or_default();
    let mut lines = vec![format!("{status} {}{scope}", report.findings.len())];
    lines.extend(report.findings.iter().map(|finding| finding.line.clone()));
    if let Some(diagnostic) = &report.diagnostic {
        lines.push(format!("DIAG {}", diagnostic.replace('\n', " ")));
    }
    format!("{}\n", lines.join("\n"))
}

pub fn write(output: &Path, report: &Report) -> anyhow::Result<()> {
    fs::create_dir_all(output)?;
    fs::write(output.join("comparison.txt"), text(report))?;
    fs::write(
        output.join("comparison.json"),
        serde_json::to_vec_pretty(report)?,
    )?;
    Ok(())
}
