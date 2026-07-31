use crate::model::{Report, Status};
use std::{fs, path::Path};

/// Withholds findings the caller has declared expected, so a comparison whose
/// only remaining differences are known fixtures can still reach a clean gate.
/// An allowance that matches nothing is reported rather than ignored.
pub fn apply_allowances(report: &mut Report, allowances: &[String]) {
    if allowances.is_empty() || !matches!(report.status, Status::Pass | Status::Fail) {
        return;
    }
    let matches =
        |line: &str, allowance: &str| line.to_lowercase().contains(&allowance.to_lowercase());
    report.unused_allowances = allowances
        .iter()
        .filter(|allowance| {
            !report
                .findings
                .iter()
                .any(|finding| matches(&finding.line, allowance))
        })
        .cloned()
        .collect();
    let (allowed, remaining) = report
        .findings
        .drain(..)
        .partition(|finding| allowances.iter().any(|a| matches(&finding.line, a)));
    report.findings = remaining;
    report.allowed = allowed;
    if report.status == Status::Fail && report.findings.is_empty() {
        report.status = Status::Pass;
    }
}

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
    let allowed = if report.allowed.is_empty() {
        String::new()
    } else {
        format!(" ALLOWED {}", report.allowed.len())
    };
    let mut lines = vec![format!(
        "{status} {}{allowed}{scope}",
        report.findings.len()
    )];
    lines.extend(report.findings.iter().map(|finding| finding.line.clone()));
    for allowance in &report.unused_allowances {
        lines.push(format!(
            "STALE ALLOWANCE {} matched nothing",
            allowance.replace('\n', " ")
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Finding;

    fn finding(line: &str) -> Finding {
        Finding {
            key: line.into(),
            line: line.into(),
            viewport: 1440,
            scenario: "base".into(),
            target: String::new(),
            property: String::new(),
            source: String::new(),
            candidate: String::new(),
            severity: "high".into(),
            confidence: "high".into(),
        }
    }

    fn failing(lines: &[&str]) -> Report {
        Report {
            schema_version: 1,
            status: Status::Fail,
            findings: lines.iter().map(|line| finding(line)).collect(),
            suppressed_duplicates: 0,
            elapsed_ms: 1,
            source_digest: String::new(),
            candidate_digest: String::new(),
            diagnostic: None,
            scope: None,
            allowed: Vec::new(),
            unused_allowances: Vec::new(),
        }
    }

    #[test]
    fn an_allowed_finding_no_longer_fails_the_comparison() {
        let mut report = failing(&["image \"Caleb Zearing\" content present->missing"]);
        apply_allowances(&mut report, &["caleb zearing".into()]);
        assert_eq!(report.status, Status::Pass);
        assert_eq!(report.allowed.len(), 1);
        assert!(report.findings.is_empty());
        assert!(text(&report).starts_with("PASS 0 ALLOWED 1"));
    }

    #[test]
    fn an_unrelated_finding_still_fails_the_comparison() {
        let mut report = failing(&["toolbar x 12->36 +24px", "image \"Caleb Zearing\" missing"]);
        apply_allowances(&mut report, &["Caleb Zearing".into()]);
        assert_eq!(report.status, Status::Fail);
        assert_eq!(report.findings.len(), 1);
        assert!(text(&report).starts_with("FAIL 1 ALLOWED 1"));
    }

    #[test]
    fn an_allowance_that_matches_nothing_is_reported() {
        let mut report = failing(&["toolbar x 12->36 +24px"]);
        apply_allowances(&mut report, &["sign in".into()]);
        assert_eq!(report.unused_allowances, vec!["sign in".to_string()]);
        assert!(text(&report).contains("STALE ALLOWANCE sign in matched nothing"));
    }
}
