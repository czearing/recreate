use super::*;
use crate::model::{Attributes, Rect, Styles, Viewport};

fn node(tag: &str) -> Node {
    Node {
        path: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
        parent: Some("html>body:nth-of-type(1)".into()),
        tag: tag.into(),
        text: String::new(),
        attributes: Attributes::new(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    }
}

fn state(nodes: Vec<Node>) -> PageState {
    PageState {
        url: String::new(),
        title: String::new(),
        viewport: Viewport::default(),
        nodes,
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

#[test]
fn custom_controls_get_keyboard_semantics() {
    let binding = trigger_binding(Some(&node("div")), "event=>onReset(event)", None);
    assert!(binding.contains("role=\"button\""));
    assert!(binding.contains("tabIndex={0}"));
    assert!(binding.contains("onKeyDown"));
}

#[test]
fn native_controls_keep_browser_keyboard_behavior() {
    let binding = trigger_binding(Some(&node("button")), "event=>onReset(event)", None);
    assert!(!binding.contains("tabIndex"));
    assert!(!binding.contains("onKeyDown"));
}

#[test]
fn listbox_gets_deliberate_programmatic_focus() {
    let mut listbox = node("div");
    listbox.attributes.insert("role".into(), "listbox".into());
    let binding = focus_binding(&listbox);
    assert!(is_popup(&listbox));
    assert!(binding.contains("focus({preventScroll:true})"));
    assert!(binding.contains("tabIndex={-1}"));
}

#[test]
fn semantic_trigger_ignores_same_path_wrong_control() {
    let mut wrong = node("button");
    wrong
        .attributes
        .insert("aria-label".into(), "My Notebook".into());
    let mut search = node("button");
    search.path.push_str(">button:nth-of-type(1)");
    search
        .attributes
        .insert("aria-label".into(), "Search".into());
    let interaction = Interaction {
        trigger_path: wrong.path.clone(),
        trigger_tag: "button".into(),
        trigger_label: "Search".into(),
        focused_path: None,
        states: Vec::new(),
    };
    let search_path = search.path.clone();
    let state = state(vec![wrong, search]);
    assert!(!matches_trigger(&interaction, &state.nodes[0], &state));
    assert_eq!(
        semantic_trigger(&interaction, &state).map(|node| &node.path),
        Some(&search_path)
    );
}

#[test]
fn semantic_trigger_matches_descendant_text() {
    let button = node("button");
    let mut text = node("#text");
    text.path = format!("{}>#text(1)", button.path);
    text.parent = Some(button.path.clone());
    text.text = "  More   tasks ".into();
    let interaction = Interaction {
        trigger_path: "missing".into(),
        trigger_tag: "button".into(),
        trigger_label: "More tasks".into(),
        focused_path: None,
        states: Vec::new(),
    };
    let state = state(vec![button, text]);
    assert_eq!(
        semantic_trigger(&interaction, &state).map(|node| &node.path),
        Some(&state.nodes[0].path)
    );
}
