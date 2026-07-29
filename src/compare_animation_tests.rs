use super::*;
use crate::model::{PageState, Viewport};
use serde_json::json;

fn state(target: &str, duration: f64) -> PageState {
    PageState {
        url: String::new(),
        title: String::new(),
        viewport: Viewport::default(),
        nodes: Vec::new(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: vec![Animation {
            target: target.into(),
            keyframes: vec![
                json!({"offset":0.0,"opacity":"0","easing":"ease-in-out"}),
                json!({"offset":1.0,"opacity":"1","easing":"ease-in-out"}),
            ],
            timing: json!({"duration":duration,"iterations":"infinite","easing":"linear"}),
        }],
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

#[test]
fn recognizes_equivalent_browser_animation_observations() {
    let expected = state("html>body>div:nth-of-type(1)", 16000.0);
    let mut actual = expected.clone();
    actual.animations[0].keyframes[0]["computedOffset"] = json!(0.0);
    assert!(equivalent_at(
        &expected,
        &actual,
        "html>body>div:nth-of-type(1)"
    ));
    assert!(phase_shifted_descendant(
        &expected,
        &actual,
        "html>body>div:nth-of-type(1)>span:nth-of-type(1)"
    ));
}

#[test]
fn rejects_different_animation_timing() {
    let expected = state("html>body", 16000.0);
    let actual = state("html>body", 9000.0);
    assert!(!equivalent_at(&expected, &actual, "html>body"));
}

#[test]
fn recognizes_equivalent_animation_on_a_rebased_target() {
    let expected = state("html>body>main", 16000.0);
    let actual = state("html>body>section", 16000.0);
    assert!(equivalent_anywhere(&expected, &actual, "html>body>section"));
}

#[test]
fn exposes_browser_webkit_animation_properties_as_css_names() {
    let mut expected = state("html>body>main", 16000.0);
    expected.animations[0].keyframes = vec![
        json!({"webkitMaskPositionX":"0%"}),
        json!({"webkitMaskPositionX":"100%"}),
    ];
    let properties = properties(&expected, "html>body>main");
    assert!(properties.contains("-webkit-mask-position-x"));
    assert!(properties.contains("-webkit-mask-position"));
    assert!(properties.contains("mask-position-x"));
    assert!(properties.contains("mask-position"));
}
