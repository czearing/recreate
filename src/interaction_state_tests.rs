use super::*;
use crate::model::{Node, Rect, StateStyle, Viewport};
use std::collections::BTreeMap;

fn state(nodes: usize) -> PageState {
    PageState {
        url: "https://example.test".into(),
        title: "Fixture".into(),
        viewport: Viewport::default(),
        nodes: (0..nodes)
            .map(|index| Node {
                path: format!("html>body>div:nth-of-type({index})"),
                parent: Some("html>body".into()),
                tag: "div".into(),
                text: index.to_string(),
                attributes: BTreeMap::new(),
                rect: Rect {
                    x: 0.0,
                    y: index as f64,
                    width: 100.0,
                    height: 20.0,
                },
                style: BTreeMap::new(),
                before: None,
                after: None,
            })
            .collect(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: vec![StateStyle {
            target: "html>body".into(),
            scope: None,
            pseudo: Some(":focus".into()),
            target_pseudo: None,
            media: None,
            declarations: "outline:1px solid".into(),
        }],
        css_rules: vec!["body{margin:0}".into()],
        asset_urls: vec!["https://example.test/logo.svg".into()],
        asset_data: BTreeMap::from([("blob:logo".into(), "data:image/png;base64,AA==".into())]),
    }
}

#[test]
fn scales_to_large_states_without_serializing() {
    let baseline = state(10_000);
    let mut changed = baseline.clone();
    assert!(!differs(&baseline, &changed));
    changed.nodes[9_999]
        .attributes
        .insert("aria-expanded".into(), "true".into());
    assert!(differs(&baseline, &changed));
}

#[test]
fn ignores_focus_only_style_changes_for_interaction_discovery() {
    let baseline = state(1);
    let mut focused = baseline.clone();
    focused.nodes[0]
        .style
        .insert("outline".into(), "2px solid".into());
    focused.nodes[0].rect.y = -4.0;
    assert!(differs(&baseline, &focused));
    assert!(!meaningfully_differs(&baseline, &focused));
}

#[test]
fn detects_persistent_content_actions() {
    let baseline = state(1);
    let mut changed = baseline.clone();
    changed.nodes[0].text = "next card".into();
    assert!(!meaningfully_differs(&baseline, &changed));
    assert!(content_differs(&baseline, &changed));
}

#[test]
fn removes_only_metadata_already_in_baseline() {
    let baseline = state(1);
    let mut changed = baseline.clone();
    changed
        .css_rules
        .push("[role=dialog]{display:block}".into());
    changed
        .asset_urls
        .push("https://example.test/dialog.svg".into());
    compact(&mut changed, &baseline, true);
    assert_eq!(changed.css_rules, ["[role=dialog]{display:block}"]);
    assert_eq!(changed.asset_urls, ["https://example.test/dialog.svg"]);
    assert!(changed.asset_data.is_empty());
    assert!(changed.state_styles.is_empty());
}

#[test]
fn compaction_reduces_repeated_output_size() {
    let mut baseline = state(20);
    baseline.asset_data.insert(
        "blob:large".into(),
        format!("data:image/png;base64,{}", "A".repeat(100_000)),
    );
    let mut changed = baseline.clone();
    let before = serde_json::to_vec(&changed).unwrap().len();
    compact(&mut changed, &baseline, true);
    let after = serde_json::to_vec(&changed).unwrap().len();
    assert!(after * 4 < before, "before={before} after={after}");
}

#[test]
fn preserves_running_animation_metadata_at_safety_cap() {
    let baseline = state(1);
    let mut changed = baseline.clone();
    changed.animations.push(crate::model::Animation {
        target: "html>body>div:nth-of-type(0)".into(),
        keyframes: vec![
            serde_json::json!({"opacity":"0"}),
            serde_json::json!({"opacity":"1"}),
        ],
        timing: serde_json::json!({"duration":2000,"playState":"running"}),
    });
    compact(&mut changed, &baseline, false);
    assert_eq!(changed.animations.len(), 1);
    compact(&mut changed, &baseline, true);
    assert!(changed.animations.is_empty());
}

#[test]
fn removes_synthetic_layout_tracks_at_safety_cap() {
    let baseline = state(1);
    let mut changed = baseline.clone();
    changed.animations.push(crate::model::Animation {
        target: "html>body".into(),
        keyframes: vec![
            serde_json::json!({"x":0,"y":0,"width":100,"height":20,"opacity":"1"}),
            serde_json::json!({"x":0,"y":24,"width":100,"height":20,"opacity":"1"}),
        ],
        timing: serde_json::json!({"duration":2500}),
    });
    compact(&mut changed, &baseline, false);
    assert!(changed.animations.is_empty());
}
