use super::*;
use crate::model::{Node, Rect, StateStyle, Viewport};
use std::collections::BTreeMap;

fn state(nodes: usize) -> PageState {
    PageState {
        url: "https://example.test".into(),
        title: "Fixture".into(),
        viewport: Viewport::default(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
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
                disabled: false,
                rtl: false,
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
        attribute_sequences: Vec::new(),
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
fn detects_newly_visible_fixed_surfaces() {
    let mut baseline = state(2);
    baseline.nodes[1]
        .style
        .insert("position".into(), "fixed".into());
    baseline.nodes[1]
        .style
        .insert("display".into(), "none".into());
    let mut opened = baseline.clone();
    opened.nodes[1]
        .style
        .insert("display".into(), "block".into());
    assert!(surface_differs(
        &opened,
        &baseline,
        "html>body>button:nth-of-type(1)",
        "Open actions"
    ));
}

#[test]
fn detects_newly_inserted_portal_roots() {
    let baseline = state(1);
    let mut opened = baseline.clone();
    let mut portal = opened.nodes[0].clone();
    portal.path = "html>body>div:nth-of-type(2)".into();
    portal
        .attributes
        .insert("data-portal-node".into(), "true".into());
    portal.text = "Command panel".into();
    opened.nodes.push(portal);

    assert!(surface_differs(
        &opened,
        &baseline,
        "html>body>button:nth-of-type(1)",
        "Command panel"
    ));
}

#[test]
fn detects_visible_descendants_of_zero_size_portals() {
    let baseline = state(1);
    let mut opened = baseline.clone();
    let mut portal = opened.nodes[0].clone();
    portal.path = "html>body>div:nth-of-type(2)".into();
    portal.rect.width = 0.0;
    portal.rect.height = 0.0;
    portal
        .attributes
        .insert("data-portal-node".into(), "true".into());
    let mut child = opened.nodes[0].clone();
    child.path = format!("{}>iframe:nth-of-type(1)", portal.path);
    child.parent = Some(portal.path.clone());
    child.tag = "iframe".into();
    opened.nodes.extend([portal, child]);

    assert!(surface_differs(
        &opened,
        &baseline,
        "html>body>button:nth-of-type(1)",
        "Command panel"
    ));
}

#[test]
fn ignores_newly_visible_content_inside_the_trigger() {
    let mut baseline = state(2);
    baseline.nodes[1].path = format!("{}>span:nth-of-type(1)", baseline.nodes[0].path);
    baseline.nodes[1]
        .style
        .insert("position".into(), "absolute".into());
    baseline.nodes[1]
        .style
        .insert("display".into(), "none".into());
    let mut changed = baseline.clone();
    changed.nodes[1]
        .style
        .insert("display".into(), "block".into());

    assert!(!surface_differs(
        &changed,
        &baseline,
        &baseline.nodes[0].path,
        "Open profile"
    ));
}

#[test]
fn ignores_replaced_text_inside_absolute_content() {
    let baseline = state(1);
    let mut changed = baseline.clone();
    changed.nodes.push(Node {
        path: "html>body>prompt>#text(2)".into(),
        parent: Some("html>body>prompt".into()),
        tag: "#text".into(),
        text: "Rotating prompt".into(),
        attributes: BTreeMap::new(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        style: BTreeMap::from([("position".into(), "absolute".into())]),
        before: None,
        after: None,
        disabled: false,
        rtl: false,
    });

    assert!(!surface_differs(
        &changed,
        &baseline,
        "html>body>avatar",
        "Open profile"
    ));
}

#[path = "interaction_state_compaction_tests.rs"]
mod compaction;
