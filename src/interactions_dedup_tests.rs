use super::*;
use crate::model::Interaction;

#[test]
fn duplicate_path_evidence_keeps_the_richest_state() {
    let mut sparse = Interaction {
        trigger_path: "card>button".into(),
        trigger_tag: "button".into(),
        trigger_label: "Open actions".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: vec![crate::model::PageState {
            url: String::new(),
            title: String::new(),
            viewport: crate::model::Viewport::default(),
            dom: Default::default(),
            capture_blockers: Vec::new(),
            nodes: Vec::new(),
            startup_nodes: Vec::new(),
            startup_delay_ms: 0,
            startup_duration_ms: 0,
            animations: Vec::new(),
            state_styles: Vec::new(),
            attribute_sequences: Vec::new(),
            css_rules: Vec::new(),
            asset_urls: Vec::new(),
            asset_data: Default::default(),
        }],
    };
    let mut rich = sparse.clone();
    rich.trigger_occurrence = Some(7);
    rich.states[0].nodes.push(crate::model::Node {
        path: "menu".into(),
        parent: None,
        tag: "div".into(),
        text: "Pin".into(),
        attributes: Default::default(),
        rect: crate::model::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Default::default(),
        before: None,
        after: None,
        disabled: false,
        rtl: false,
    });
    sparse.focused_path = Some("sparse".into());
    let mut values = vec![sparse, rich];
    deduplicate(&mut values);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].trigger_path, "card>button");
    assert_eq!(values[0].trigger_occurrence, Some(7));
    assert_eq!(values[0].states[0].nodes.len(), 1);
}

pub(super) fn state_with_paths(paths: &[&str]) -> crate::model::PageState {
    let mut state = crate::model::PageState {
        url: String::new(),
        title: String::new(),
        viewport: crate::model::Viewport::default(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes: Vec::new(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    };
    state.nodes = paths
        .iter()
        .map(|path| crate::model::Node {
            path: (*path).into(),
            parent: None,
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: crate::model::Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            style: Default::default(),
            before: None,
            after: None,
            disabled: false,
            rtl: false,
        })
        .collect();
    state
}
