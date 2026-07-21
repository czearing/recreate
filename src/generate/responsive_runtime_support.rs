use crate::{
    browser, capture,
    cli::CaptureArgs,
    compare_node::same_rect,
    lifecycle_script,
    model::{PageState, Styles, Viewport},
};
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;

pub const VIEWPORTS: [(u32, u32); 5] = [
    (1920, 1080),
    (1440, 900),
    (768, 1024),
    (390, 844),
    (320, 568),
];

pub async fn connect(url: &str, endpoint: &str) -> Result<crate::cdp::Cdp> {
    let args = CaptureArgs {
        url: Some(url.into()),
        reuse: false,
        reload: false,
        baseline_only: false,
        spec_only: false,
        target: None,
        cdp_url: endpoint.into(),
        out: Default::default(),
        viewports: String::new(),
    };
    let (_, mut cdp) = browser::target(&args).await?;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({"source": lifecycle_script::SOURCE}),
    )
    .await?;
    Ok(cdp)
}

pub async fn capture_matrix(cdp: &mut crate::cdp::Cdp) -> Result<Vec<PageState>> {
    let mut states = Vec::new();
    for (index, (width, height)) in VIEWPORTS.into_iter().enumerate() {
        let viewport = Viewport {
            width,
            height,
            dpr: 1.0,
        };
        let state = capture::capture_state(cdp, viewport, index == 0).await?;
        assert_no_overflow(cdp, width).await?;
        states.push(state);
    }
    Ok(states)
}

pub async fn assert_no_overflow(cdp: &mut crate::cdp::Cdp, width: u32) -> Result<()> {
    let overflow = cdp
        .evaluate(
            "Math.max(document.documentElement.scrollWidth,\
             document.body?.scrollWidth || 0) - document.documentElement.clientWidth",
        )
        .await?
        .as_f64()
        .context("overflow evaluation returned non-number")?;
    let offenders = if overflow > 0.0 {
        cdp.evaluate(
            "[...document.querySelectorAll('*')].map(element => {\
               const rect=element.getBoundingClientRect();\
               return {tag:element.tagName,id:element.id,class:element.className,\
                 left:rect.left,right:rect.right,width:rect.width};\
             }).filter(rect => rect.right > document.documentElement.clientWidth + 0.5)",
        )
        .await?
    } else {
        serde_json::Value::Null
    };
    assert!(
        overflow <= 0.0,
        "{width}px horizontal overflow: {overflow}px; offenders: {offenders}"
    );
    Ok(())
}

pub fn assert_exact_parity(expected: &PageState, actual: &PageState) {
    let actual: BTreeMap<_, _> = actual.nodes.iter().map(|node| (&node.path, node)).collect();
    assert_eq!(expected.nodes.len(), actual.len());
    for node in &expected.nodes {
        let candidate = actual
            .get(&node.path)
            .unwrap_or_else(|| panic!("missing {} at {}px", node.path, expected.viewport.width));
        assert_eq!(node.text, candidate.text, "text {}", node.path);
        assert_style_parity(&node.style, &candidate.style, &node.path);
        assert!(same_rect(node, candidate), "geometry {}", node.path);
    }
}

pub fn assert_stable_dom_text(first: &PageState, second: &PageState) {
    let first: Vec<_> = first
        .nodes
        .iter()
        .map(|node| (&node.path, &node.text))
        .collect();
    let second: Vec<_> = second
        .nodes
        .iter()
        .map(|node| (&node.path, &node.text))
        .collect();
    assert_eq!(first, second);
}

pub fn assert_clean_events(cdp: &mut crate::cdp::Cdp) {
    let errors: Vec<_> = cdp
        .take_events()
        .into_iter()
        .filter(|event| match event["method"].as_str() {
            Some("Runtime.exceptionThrown") => true,
            Some("Runtime.consoleAPICalled") => event["params"]["type"] == "error",
            Some("Network.loadingFailed") => {
                event["params"]["canceled"].as_bool() != Some(true)
                    && event["params"]["errorText"] != "net::ERR_ABORTED"
            }
            Some("Network.responseReceived") => event["params"]["response"]["status"]
                .as_f64()
                .is_some_and(|status| status >= 400.0),
            _ => false,
        })
        .collect();
    assert!(errors.is_empty(), "browser errors: {errors:?}");
}

pub fn equivalent_style(property: &str, expected: &str, actual: &str) -> bool {
    expected == actual
        || matches!(
            property,
            "background-position" | "mask-position" | "object-position"
        ) && normalize_zero_units(expected) == normalize_zero_units(actual)
}

fn assert_style_parity(expected: &Styles, actual: &Styles, path: &str) {
    assert_eq!(expected.len(), actual.len(), "style count {path}");
    for (property, expected) in expected {
        let actual = actual
            .get(property)
            .unwrap_or_else(|| panic!("missing style {property} on {path}"));
        assert!(
            equivalent_style(property, expected, actual),
            "style {path} {property}: expected={expected:?} actual={actual:?}"
        );
    }
}

fn normalize_zero_units(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let number = part.trim_end_matches(|character: char| {
                character.is_ascii_alphabetic() || character == '%'
            });
            if number.parse::<f64>() == Ok(0.0) {
                "0"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
