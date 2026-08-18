use super::*;
use crate::model::{Interaction, PageState, Rect, Viewport};

fn node(path: &str, parent: Option<&str>, text: &str, visible: bool) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: "div".into(),
        text: text.into(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: if visible { 20.0 } else { 0.0 },
            height: if visible { 20.0 } else { 0.0 },
        },
        style: Default::default(),
        ..Default::default()
    }
}

#[test]
fn removes_stale_hidden_overflow_surfaces() {
    let baseline = node("html>body", Some("html"), "", true);
    let mut first_trigger = node(
        "html>body>button:nth-of-type(1)",
        Some("html>body"),
        "",
        true,
    );
    first_trigger.tag = "button".into();
    first_trigger
        .attributes
        .insert("aria-label".into(), "Open actions".into());
    let mut second_trigger = first_trigger.clone();
    second_trigger.path = "html>body>button:nth-of-type(2)".into();
    let state = PageState {
        url: String::new(),
        title: String::new(),
        viewport: Viewport::default(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes: vec![
            baseline.clone(),
            first_trigger.clone(),
            second_trigger.clone(),
            node(
                "html>body>div:nth-of-type(1)",
                Some("html>body"),
                "Pin",
                false,
            ),
            node(
                "html>body>div:nth-of-type(2)",
                Some("html>body"),
                "Pin",
                true,
            ),
        ],
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        css_shorthands: Default::default(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    };
    let mut specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![PageState {
            nodes: vec![baseline, first_trigger, second_trigger],
            ..state.clone()
        }],
        interactions: vec![Interaction {
            trigger_path: String::new(),
            trigger_tag: "button".into(),
            trigger_label: "Open actions".into(),
            trigger_occurrence: None,
            focused_path: None,
            states: vec![state],
        }],
        transitions: Vec::new(),
    };
    normalize(&mut specification);
    assert_eq!(specification.interactions[0].states[0].nodes.len(), 4);
    assert!(
        specification.interactions[0].states[0].nodes[3]
            .path
            .ends_with("div:nth-of-type(2)")
    );
}
