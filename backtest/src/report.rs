use crate::model::{Finding, Report, Status};
use anyhow::Context;
use std::{fs, path::Path};

pub fn line(finding: &Finding) -> String {
    let source = display_value(&finding.property, &finding.source);
    let candidate = display_value(&finding.property, &finding.candidate);
    let mut value = format!(
        "V{} {} {} {} {}->{}",
        finding.viewport,
        finding.action,
        finding.target,
        finding.property,
        source,
        candidate
    );
    if let Some(delta) = &finding.delta
        && delta != "0"
    {
        value.push(' ');
        value.push_str(delta);
    }

    fn display_value(property: &str, value: &str) -> String {
        let value = compact(value);
        if property == "text" {
            format!("\"{value}\"")
        } else {
            value
        }
    }
    value.chars().take(120).collect()
}

pub fn text(report: &Report) -> String {
    let status = match report.status {
        Status::Pass => "PASS",
        Status::Fail => "FAIL",
        Status::Inconclusive => "INCONCLUSIVE",
        Status::PreparationRequired => "PREPARATION_REQUIRED",
    };
    let mut lines = vec![format!("{status} {}", report.findings.len())];
    lines.extend(report.findings.iter().map(line));
    if report.status == Status::Inconclusive {
        lines.extend(
            report
                .coverage
                .iter()
                .map(|value| format!("COVERAGE {}", compact(value))),
        );
    }
    lines.join("\n") + "\n"
}

pub fn write(report: &Report, directory: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(directory)
        .with_context(|| format!("create report directory {}", directory.display()))?;
    fs::write(directory.join("comparison.txt"), text(report))?;
    fs::write(
        directory.join("comparison.json"),
        serde_json::to_vec_pretty(report)?,
    )?;
    Ok(())
}

fn compact(value: &str) -> String {
    value.replace(['\r', '\n', '\t'], " ").replace("  ", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_compact_stable_line() {
        let finding = Finding {
            id: "a".into(),
            key: "a".into(),
            category: "geometry".into(),
            viewport: 1440,
            checkpoint: "base".into(),
            action: "click:sign-in".into(),
            target: "dialog".into(),
            property: "width".into(),
            source: "480".into(),
            candidate: "456".into(),
            delta: Some("-24px".into()),
            effects: Vec::new(),
        };
        assert_eq!(
            line(&finding),
            "V1440 click:sign-in dialog width 480->456 -24px"
        );
    }
}
