use super::bindings::{focus_binding, is_popup, trigger_binding};
use super::*;
use crate::model::{Attributes, Node, Rect, Styles, Viewport};

#[test]
fn reduced_motion_keeps_authored_transitions() {
    assert!(REDUCED_MOTION_CSS.contains("animation:none!important"));
    assert!(!REDUCED_MOTION_CSS.contains("transition:none"));
}

#[test]
fn interaction_layers_do_not_substitute_extreme_z_indexes_for_dom_hierarchy() {
    assert!(!FOCUS_CSS.contains("214748"));
    assert!(FOCUS_CSS.contains(".recreateAnchoredSurface{display:contents}"));
}

pub(super) fn node(tag: &str) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
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

pub(super) fn state(nodes: Vec<Node>) -> PageState {
    PageState {
        url: String::new(),
        title: String::new(),
        viewport: Viewport::default(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
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
fn text_entry_captures_the_safe_input_state() {
    let binding = trigger_binding(Some(&node("textarea")), "event=>activate(event,1)", Some(1));
    assert!(binding.contains("activate(event,1,event.currentTarget.value.length>0)"));
    assert!(!binding.contains("onClick"));
    assert!(!binding.contains("onKeyDown"));
}

#[test]
fn transition_edges_bind_forward_and_reverse_states() {
    let control = node("button");
    let baseline = state(vec![control.clone()]);
    let interaction = Interaction {
        trigger_path: control.path.clone(),
        trigger_tag: control.tag.clone(),
        trigger_label: String::new(),
        trigger_occurrence: None,
        focused_path: None,
        states: vec![baseline.clone()],
    };
    let edge = |from_state, to_state| crate::model::InteractionTransition {
        from_state,
        to_state,
        action: crate::model::InteractionAction::Activate,
        trigger_path: control.path.clone(),
        trigger_tag: control.tag.clone(),
        trigger_label: String::new(),
        trigger_occurrence: None,
    };
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![baseline.clone()],
        interactions: vec![interaction],
        transitions: vec![edge(0, 1), edge(1, 0)],
    };
    let key = serde_json::to_string(&transition_key(&specification.transitions[0])).unwrap();
    assert!(base_handlers(&specification, &baseline)[&control.path].contains(&key));
    assert!(transition_handlers(&specification, &baseline, 1)[&control.path].contains(&key));
}

#[test]
fn text_entry_states_render_without_becoming_dismissible_popups() {
    let interaction = Interaction {
        trigger_path: String::new(),
        trigger_tag: "textarea".into(),
        trigger_label: "Enter a prompt".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: Vec::new(),
    };
    assert!(rendered(&interaction, &[]));
    assert!(!closable(&interaction, &[]));
}

#[test]
fn semantic_state_controls_render_without_becoming_dismissible() {
    let mut trigger = node("button");
    trigger.attributes.insert("role".into(), "tab".into());
    let interaction = Interaction {
        trigger_path: trigger.path.clone(),
        trigger_tag: trigger.tag.clone(),
        trigger_label: "Activity".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: Vec::new(),
    };
    let baselines = [state(vec![trigger])];
    assert!(rendered(&interaction, &baselines));
    assert!(!closable(&interaction, &baselines));
}

#[test]
fn focus_state_on_popup_trigger_is_not_itself_closable() {
    let mut trigger = node("div");
    trigger
        .attributes
        .insert("aria-haspopup".into(), "dialog".into());
    let baseline = state(vec![trigger.clone()]);
    trigger.style.insert("outline-width".into(), "3px".into());
    let interaction = Interaction {
        trigger_path: trigger.path.clone(),
        trigger_tag: trigger.tag.clone(),
        trigger_label: "Account details".into(),
        trigger_occurrence: None,
        focused_path: Some(trigger.path.clone()),
        states: vec![state(vec![trigger])],
    };
    assert!(!closable(&interaction, &[baseline]));
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

#[path = "interactions_semantic_tests.rs"]
mod semantic;
#[path = "interactions_surface_tests.rs"]
mod surface;
