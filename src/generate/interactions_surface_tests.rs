use super::*;

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
        trigger_label: "Open actions".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: vec![captured.clone()],
    };
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![baseline.clone()],
        interactions: vec![interaction.clone()],
        transitions: Vec::new(),
    };
    let handlers = state_handlers(&specification, 1, &interaction, &captured, &baseline);
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
    text.text = "  Next   items ".into();
    let interaction = Interaction {
        trigger_path: "missing".into(),
        trigger_tag: "button".into(),
        trigger_label: "Next items".into(),
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
    first_text.text = "Open actions".into();
    let mut second_text = first_text.clone();
    second_text.path = format!("{}>#text(1)", second.path);
    second_text.parent = Some(second.path.clone());
    let mut interaction = Interaction {
        trigger_path: "missing".into(),
        trigger_tag: "button".into(),
        trigger_label: "Open actions".into(),
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
        trigger_label: "Next items".into(),
        trigger_occurrence: None,
        focused_path: None,
        states,
    };
    assert!(!closable(&interaction, &baselines));
}
