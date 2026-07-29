use super::*;

fn row_parent(width: f64) -> Node {
    let mut parent = node("parent", 0.0, width);
    parent.style.insert("display".into(), "flex".into());
    parent.style.insert("flex-direction".into(), "row".into());
    parent
}

#[test]
fn detects_responsive_flex_shrink() {
    let base = node("group", 0.0, 180.0);
    let mut current = node("group", 0.0, 162.0);
    current.style.insert("flex-shrink".into(), "1".into());
    assert!(shrunk_flex_item(&base, &current, Some(&row_parent(206.0))));
}

#[test]
fn does_not_constrain_intrinsic_content_overflowing_a_flex_parent() {
    let base = node("label", 0.0, 154.0);
    let mut current = node("label", 0.0, 59.0);
    current.style.insert("flex-shrink".into(), "1".into());
    assert!(!shrunk_flex_item(&base, &current, Some(&row_parent(5.0))));
}

#[test]
fn constrains_only_descendants_in_the_same_flex_chain() {
    let mut root = node("root", 0.0, 162.0);
    root.path = "root".into();
    let mut button = node("button", 0.0, 121.0);
    button.path = "root>button".into();
    button.parent = Some(root.path.clone());
    button.style.insert("display".into(), "flex".into());
    button.style.insert("flex-direction".into(), "row".into());
    let mut icon = node("svg", 0.0, 16.0);
    icon.path = "root>button>svg".into();
    icon.parent = Some(button.path.clone());
    let nodes = HashMap::from([
        (root.path.as_str(), &root),
        (button.path.as_str(), &button),
        (icon.path.as_str(), &icon),
    ]);
    let roots = HashSet::from([root.path.as_str()]);
    assert!(constrained_by_flex_chain(&button, &roots, &nodes));
    assert!(constrained_by_flex_chain(&icon, &roots, &nodes));
}

#[test]
fn does_not_constrain_descendants_below_a_non_flex_container() {
    let mut root = node("root", 0.0, 162.0);
    root.path = "root".into();
    let mut card = node("card", 0.0, 140.0);
    card.path = "root>card".into();
    card.parent = Some(root.path.clone());
    card.style.insert("display".into(), "block".into());
    let mut icon = node("svg", 0.0, 16.0);
    icon.path = "root>card>svg".into();
    icon.parent = Some(card.path.clone());
    let nodes = HashMap::from([
        (root.path.as_str(), &root),
        (card.path.as_str(), &card),
        (icon.path.as_str(), &icon),
    ]);
    let roots = HashSet::from([root.path.as_str()]);
    assert!(!constrained_by_flex_chain(&icon, &roots, &nodes));
}

#[test]
fn removes_sampled_width_from_fluid_flex_items() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let child = node("div", 0.0, 600.0);
    let mut parent = node("div", 0.0, 1200.0);
    parent.style.insert("display".into(), "flex".into());
    let css = base_declarations(
        &child,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(!css.contains("width:600px"));
}

#[test]
fn preserves_fixed_width_inside_centered_column_flex() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut child = node("div", 0.0, 48.0);
    child.style.insert("align-self".into(), "auto".into());
    let mut parent = node("button", 0.0, 104.0);
    parent.style.extend([
        ("display".into(), "flex".into()),
        ("flex-direction".into(), "column".into()),
        ("align-items".into(), "center".into()),
    ]);
    let css = base_declarations(
        &child,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(css.contains("width:48px"));
}

#[test]
fn removes_measured_width_from_intrinsic_column_flex_text() {
    let viewport = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("section", 0.0, 768.0);
    parent.style.extend([
        ("display".into(), "flex".into()),
        ("flex-direction".into(), "column".into()),
    ]);
    let mut subtitle = node("div", 0.0, 475.765625);
    subtitle.text = "Bring everyone together".into();
    subtitle.style.extend([
        ("display".into(), "block".into()),
        ("position".into(), "static".into()),
        ("width".into(), "475.765625px".into()),
    ]);
    let css = base_declarations(
        &subtitle,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        true,
    );
    assert!(!css.contains("width:475.765625px"));
}
