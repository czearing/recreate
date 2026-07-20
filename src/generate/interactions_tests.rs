use super::*;
use crate::model::{Attributes, Rect, Styles, Viewport};

#[test]
fn reduced_motion_keeps_authored_transitions() {
    assert!(REDUCED_MOTION_CSS.contains("animation:none!important"));
    assert!(!REDUCED_MOTION_CSS.contains("transition:none"));
}

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
        attribute_sequences: Vec::new(),
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
        trigger_occurrence: None,
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
fn missing_semantic_trigger_does_not_bind_the_stale_path() {
    let mut wrong = node("button");
    wrong
        .attributes
        .insert("aria-label".into(), "Search".into());
    let interaction = Interaction {
        trigger_path: wrong.path.clone(),
        trigger_tag: "button".into(),
        trigger_label: "Open account menu".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: Vec::new(),
    };
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![state(vec![wrong.clone()])],
        interactions: vec![interaction],
    };
    assert!(base_handlers(&specification, &specification.states[0]).is_empty());
}

#[test]
fn legacy_repeated_controls_all_receive_the_shared_surface_handler() {
    let mut first = node("button");
    first
        .attributes
        .insert("aria-label".into(), "More options".into());
    let mut second = first.clone();
    second.path = "html>body:nth-of-type(1)>div:nth-of-type(2)".into();
    let interaction = Interaction {
        trigger_path: first.path.clone(),
        trigger_tag: "button".into(),
        trigger_label: "More options".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: Vec::new(),
    };
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![state(vec![first, second])],
        interactions: vec![interaction],
    };
    let handlers = base_handlers(&specification, &specification.states[0]);
    assert_eq!(handlers.len(), 2);
    assert!(
        handlers
            .values()
            .all(|value| value.contains("activate(event,1)"))
    );
}

#[test]
fn overflow_menu_root_is_marked_as_a_surface() {
    let anchor = node("div");
    let mut menu = node("div");
    menu.path.push_str(">div:nth-of-type(1)");
    menu.parent = Some(anchor.path.clone());
    menu.style.insert("position".into(), "absolute".into());
    let mut pin = node("button");
    pin.path = format!("{}>button:nth-of-type(1)", menu.path);
    pin.parent = Some(menu.path.clone());
    pin.text = "Pin".into();
    pin.rect.width = 40.0;
    pin.rect.height = 20.0;
    let baseline = state(vec![anchor.clone()]);
    let captured = state(vec![anchor, menu.clone(), pin]);
    let interaction = Interaction {
        trigger_path: String::new(),
        trigger_tag: "button".into(),
        trigger_label: "More options".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: vec![captured.clone()],
    };
    let handlers = state_handlers(&interaction, &captured, &baseline);
    assert!(
        handlers
            .get(&menu.path)
            .is_some_and(|binding| binding.contains("data-recreate-surface"))
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
        trigger_occurrence: None,
        focused_path: None,
        states: Vec::new(),
    };
    let state = state(vec![button, text]);
    assert_eq!(
        semantic_trigger(&interaction, &state).map(|node| &node.path),
        Some(&state.nodes[0].path)
    );
}

#[test]
fn semantic_trigger_requires_occurrence_for_repeated_controls() {
    let first = node("button");
    let mut second = node("button");
    second.path = "html>body:nth-of-type(1)>div:nth-of-type(2)".into();
    let mut first_text = node("#text");
    first_text.path = format!("{}>#text(1)", first.path);
    first_text.parent = Some(first.path.clone());
    first_text.text = "More options".into();
    let mut second_text = first_text.clone();
    second_text.path = format!("{}>#text(1)", second.path);
    second_text.parent = Some(second.path.clone());
    let mut interaction = Interaction {
        trigger_path: "missing".into(),
        trigger_tag: "button".into(),
        trigger_label: "More options".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: Vec::new(),
    };
    let state = state(vec![first, first_text, second, second_text]);
    assert!(semantic_trigger(&interaction, &state).is_none());
    interaction.trigger_occurrence = Some(1);
    assert_eq!(
        semantic_trigger(&interaction, &state).map(|node| node.path.as_str()),
        Some(state.nodes[2].path.as_str())
    );
}

#[test]
fn one_mismatched_baseline_does_not_turn_scroll_into_overlay() {
    let mut baselines = Vec::new();
    let mut states = Vec::new();
    for width in [1920, 1440, 768] {
        let mut baseline = state(vec![node("div")]);
        baseline.viewport.width = width;
        let mut captured = baseline.clone();
        if width == 1920 {
            for index in 0..8 {
                let mut extra = node("span");
                extra
                    .path
                    .push_str(&format!(">span:nth-of-type({})", index + 1));
                captured.nodes.push(extra);
            }
        }
        baselines.push(baseline);
        states.push(captured);
    }
    let interaction = Interaction {
        trigger_path: String::new(),
        trigger_tag: "button".into(),
        trigger_label: "More tasks".into(),
        trigger_occurrence: None,
        focused_path: None,
        states,
    };
    assert!(!closable(&interaction, &baselines));
}
