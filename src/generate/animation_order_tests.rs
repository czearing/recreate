use super::css;
use crate::model::{
    Animation, Interaction, Node, PageState, Rect, Specification, Styles, Viewport,
};
use serde_json::json;

const TARGET: &str = "html>body:nth-of-type(1)>div:nth-of-type(1)";

#[test]
fn baseline_animation_rules_follow_interaction_base_styles() {
    let specification = Specification {
        schema_version: 1,
        requested_url: "https://example.com".into(),
        captured_url: "https://example.com".into(),
        states: vec![state(true)],
        interactions: vec![Interaction {
            trigger_path: TARGET.into(),
            trigger_tag: "div".into(),
            trigger_label: "Target".into(),
            focused_path: None,
            states: vec![state(false)],
        }],
    };
    let output = css::build(&specification, &Default::default());
    assert!(
        output.css.rfind("animation:none;").unwrap()
            < output.css.rfind("{animation-name:").unwrap()
    );
}

fn state(animated: bool) -> PageState {
    let mut style = Styles::new();
    style.insert("display".into(), "block".into());
    style.insert("animation".into(), "none".into());
    PageState {
        url: "https://example.com".into(),
        title: "Example".into(),
        viewport: Viewport {
            width: 800,
            height: 600,
            dpr: 1.0,
        },
        nodes: vec![
            node("html", None, style.clone()),
            node(TARGET, Some("html"), style),
        ],
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: animated
            .then(|| Animation {
                target: TARGET.into(),
                keyframes: vec![
                    json!({"offset":0,"opacity":"0"}),
                    json!({"offset":1,"opacity":"1"}),
                ],
                timing: json!({"duration":200}),
            })
            .into_iter()
            .collect(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

fn node(path: &str, parent: Option<&str>, style: Styles) -> Node {
    Node {
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        style,
        before: None,
        after: None,
    }
}
