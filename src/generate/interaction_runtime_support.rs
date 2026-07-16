use crate::{
    compare::same_rect,
    model::{Node, PageState},
};
use anyhow::Result;
use serde_json::json;
use std::collections::BTreeMap;

pub fn assert_parity(expected: &PageState, actual: &PageState) {
    let actual: BTreeMap<_, _> = actual.nodes.iter().map(|node| (&node.path, node)).collect();
    assert_eq!(expected.nodes.len(), actual.len());
    for node in &expected.nodes {
        let candidate = actual
            .get(&node.path)
            .unwrap_or_else(|| panic!("missing {}", node.path));
        assert_eq!(node.text, candidate.text, "text {}", node.path);
        assert_styles(&node.style, &candidate.style, &node.path);
        assert!(same_rect(node, candidate), "geometry {}", node.path);
    }
    assert_footer_shift(expected, &actual);
}

fn assert_styles(expected: &crate::model::Styles, actual: &crate::model::Styles, path: &str) {
    assert_eq!(expected.len(), actual.len(), "style count {path}");
    for (property, expected) in expected {
        let actual = actual
            .get(property)
            .unwrap_or_else(|| panic!("missing style {property} on {path}"));
        assert!(
            expected == actual
                || matches!(
                    property.as_str(),
                    "background-position" | "mask-position" | "object-position"
                ) && normalize_zero_units(expected) == normalize_zero_units(actual),
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

fn assert_footer_shift(expected: &PageState, actual: &BTreeMap<&String, &Node>) {
    let footer = expected
        .nodes
        .iter()
        .find(|node| node.tag == "footer")
        .unwrap();
    assert!(footer.rect.y > 180.0);
    assert_eq!(actual[&footer.path].rect, footer.rect);
}

pub fn errors(cdp: &mut crate::cdp::Cdp) -> (usize, usize) {
    cdp.take_events()
        .into_iter()
        .fold((0, 0), |mut counts, event| {
            match event["method"].as_str() {
                Some("Runtime.exceptionThrown") => counts.0 += 1,
                Some("Runtime.consoleAPICalled") if event["params"]["type"] == "error" => {
                    counts.0 += 1
                }

                Some("Network.loadingFailed")
                    if event["params"]["canceled"].as_bool() != Some(true)
                        && event["params"]["errorText"] != "net::ERR_ABORTED"
                        && !(event["params"]["type"] == "Other"
                            && event["params"]["blockedReason"] == "origin") =>
                {
                    eprintln!("network failure: {event}");
                    counts.1 += 1
                }
                Some("Network.responseReceived")
                    if event["params"]["response"]["status"]
                        .as_f64()
                        .is_some_and(|status| status >= 400.0) =>
                {
                    eprintln!("network response failure: {event}");
                    counts.1 += 1
                }
                _ => {}
            }
            counts
        })
}

pub async fn press(cdp: &mut crate::cdp::Cdp, key: &str, code: &str, key_code: u32) -> Result<()> {
    let params = json!({"key":key,"code":code,
        "windowsVirtualKeyCode":key_code,"nativeVirtualKeyCode":key_code});
    cdp.send(
        "Input.dispatchKeyEvent",
        merge(params.clone(), json!({"type":"rawKeyDown"})),
    )
    .await?;
    if key == "Enter" || key == " " {
        let text = if key == "Enter" { "\r" } else { " " };
        cdp.send(
            "Input.dispatchKeyEvent",
            merge(params.clone(), json!({"type":"char","text":text})),
        )
        .await?;
    }
    cdp.send(
        "Input.dispatchKeyEvent",
        merge(params, json!({"type":"keyUp"})),
    )
    .await?;
    Ok(())
}

pub async fn active_attribute(cdp: &mut crate::cdp::Cdp, name: &str) -> Result<String> {
    Ok(cdp
        .evaluate(&format!(
            "document.activeElement.getAttribute({})",
            serde_json::to_string(name)?
        ))
        .await?
        .as_str()
        .unwrap_or_default()
        .into())
}

pub async fn active_in_dialog(cdp: &mut crate::cdp::Cdp) -> Result<bool> {
    Ok(cdp
        .evaluate("document.activeElement.closest('[role=dialog]') != null")
        .await?
        .as_bool()
        == Some(true))
}

pub async fn validate_modal_environment(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    cdp.send("Page.reload", json!({"ignoreCache":false}))
        .await?;
    cdp.evaluate(
        "new Promise(resolve=>{const ready=()=>requestAnimationFrame(()=>\
         requestAnimationFrame(resolve));document.readyState==='complete'?ready():\
         addEventListener('load',ready,{once:true})})",
    )
    .await?;
    press(cdp, "Tab", "Tab", 9).await?;
    press(cdp, "Enter", "Enter", 13).await?;
    let value = cdp
        .evaluate(
            "({matches:matchMedia('(prefers-reduced-motion: reduce)').matches,\
             animation:getComputedStyle(document.querySelector('[role=dialog]')).animationName})",
        )
        .await?;
    assert_eq!(value["matches"].as_bool(), Some(true));
    assert_eq!(value["animation"].as_str(), Some("none"));
    assert_eq!(errors(cdp), (0, 0));
    Ok(())
}

fn merge(mut left: serde_json::Value, right: serde_json::Value) -> serde_json::Value {
    left.as_object_mut()
        .unwrap()
        .extend(right.as_object().unwrap().clone());
    left
}
