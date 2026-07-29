use super::*;

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
