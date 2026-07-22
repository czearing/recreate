use crate::model::{Interaction, PageState, Specification, Viewport};
use std::{fs, path::Path};

#[path = "browser_support.rs"]
mod browser_support;
pub use browser_support::{browser_path, free_port, launch_browser, wait_for_browser};

pub fn viewport(width: u32, height: u32) -> Viewport {
    Viewport {
        width,
        height,
        dpr: 1.0,
    }
}

pub fn selected_fixtures<'a>(fixtures: &'a [&str]) -> Vec<&'a str> {
    let selected = std::env::var("RECREATE_GATE_FIXTURE").ok();
    fixtures
        .iter()
        .copied()
        .filter(|fixture| selected.as_deref().is_none_or(|value| value == *fixture))
        .collect()
}

pub fn selected_viewports(viewports: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if std::env::var_os("RECREATE_GATE_FIXTURE").is_some()
        && std::env::var_os("RECREATE_GATE_ALL_VIEWPORTS").is_none()
        && !viewports.is_empty()
    {
        return vec![viewports[0]];
    }
    viewports.to_vec()
}

pub fn collect_errors(cdp: &mut crate::cdp::Cdp) -> (usize, usize) {
    let mut console = 0;
    let mut network = 0;
    for event in cdp.take_events() {
        match event["method"].as_str() {
            Some("Runtime.exceptionThrown") => console += 1,
            Some("Runtime.consoleAPICalled") if event["params"]["type"] == "error" => console += 1,
            Some("Network.loadingFailed")
                if event["params"]["canceled"].as_bool() != Some(true)
                    && event["params"]["errorText"] != "net::ERR_ABORTED" =>
            {
                let file_probe = event["params"]["type"] == "Other"
                    && event["params"]["blockedReason"] == "origin";
                if !file_probe {
                    eprintln!(
                        "network loading failure: {} {}",
                        event["params"]["errorText"], event["params"]["type"]
                    );
                    network += 1;
                }
            }
            Some("Network.responseReceived")
                if event["params"]["response"]["status"]
                    .as_f64()
                    .is_some_and(|status| status >= 400.0) =>
            {
                eprintln!(
                    "network response failure: {} {}",
                    event["params"]["response"]["status"], event["params"]["response"]["url"]
                );
                network += 1
            }
            _ => {}
        }
    }
    (console, network)
}

pub struct Parity {
    pub mismatches: usize,
    pub details: Vec<String>,
}

pub fn parity(left: &PageState, right: &PageState) -> Parity {
    let actual = right
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut report = Parity {
        mismatches: 0,
        details: Vec::new(),
    };
    let expected = left
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for expected in &left.nodes {
        let Some(candidate) = actual.get(expected.path.as_str()) else {
            mismatch(&mut report, format!("missing {}", expected.path));
            continue;
        };
        if expected.text != candidate.text {
            mismatch(&mut report, format!("text {}", expected.path));
        }
        if !crate::compare_node::same_rect(expected, candidate) {
            mismatch(
                &mut report,
                format!(
                    "rect {} expected={:?} actual={:?}",
                    expected.path, expected.rect, candidate.rect
                ),
            );
        }
        for key in [
            "color",
            "background-color",
            "font-family",
            "font-size",
            "font-weight",
            "border-radius",
            "display",
            "position",
        ] {
            if expected.style.get(key) != candidate.style.get(key) {
                mismatch(&mut report, format!("style {key} {}", expected.path));
            }
        }
    }
    for path in actual.keys().filter(|path| !expected.contains(*path)) {
        mismatch(&mut report, format!("unexpected {path}"));
    }
    report
}

fn mismatch(report: &mut Parity, detail: String) {
    report.mismatches += 1;
    if report.details.len() < 20 {
        report.details.push(detail);
    }
}

pub fn specification(states: Vec<PageState>, interactions: Vec<Interaction>) -> Specification {
    let captured_url = states
        .first()
        .map(|state| state.url.clone())
        .unwrap_or_default();
    Specification {
        schema_version: 1,
        requested_url: captured_url.clone(),
        captured_url,
        states,
        interactions,
    }
}

pub fn directory_size(path: &Path) -> std::io::Result<u64> {
    fs::read_dir(path)?.try_fold(0, |total, entry| {
        let entry = entry?;
        let value = if entry.file_type()?.is_dir() {
            directory_size(&entry.path())?
        } else {
            entry.metadata()?.len()
        };
        Ok(total + value)
    })
}
