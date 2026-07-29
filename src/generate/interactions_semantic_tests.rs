use super::*;

#[test]
fn semantic_trigger_ignores_same_path_wrong_control() {
    let mut wrong = node("button");
    wrong
        .attributes
        .insert("aria-label".into(), "Primary item".into());
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
        trigger_label: "Open profile".into(),
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
        transitions: Vec::new(),
    };
    assert!(base_handlers(&specification, &specification.states[0]).is_empty());
}

#[test]
fn repeated_controls_all_receive_the_shared_surface_handler() {
    let mut first = node("button");
    first
        .attributes
        .insert("aria-label".into(), "Open actions".into());
    let mut second = first.clone();
    second.path = "html>body:nth-of-type(1)>div:nth-of-type(2)".into();
    let interaction = Interaction {
        trigger_path: first.path.clone(),
        trigger_tag: "button".into(),
        trigger_label: "Open actions".into(),
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
        transitions: Vec::new(),
    };
    let handlers = base_handlers(&specification, &specification.states[0]);
    assert_eq!(handlers.len(), 2);
    assert!(
        handlers
            .values()
            .all(|value| value.contains("activate(event,1)"))
    );
}
