use super::*;

fn style(target: &str, pseudo: Option<&str>, declarations: &str) -> StateStyle {
    StateStyle {
        target: target.into(),
        pseudo: pseudo.map(str::to_string),
        media: None,
        declarations: declarations.into(),
    }
}

#[test]
fn maps_state_and_reduced_motion_rules_to_generated_classes() {
    let classes = BTreeMap::from([("html>body>button".into(), "control".into())]);
    let mut reduced = style("html>body>button", None, "transition: none;");
    reduced.media = Some("(prefers-reduced-motion: reduce)".into());
    let mut css = String::new();
    append(
        &[
            style("html>body>button", Some(":hover"), "opacity: 0.5;"),
            reduced,
        ],
        &classes,
        &BTreeMap::new(),
        &mut css,
    );
    assert!(css.contains(".control:hover{opacity: 0.5;}"));
    assert!(css.contains("@media (prefers-reduced-motion: reduce)"));
}

#[test]
fn groups_identical_reduced_motion_declarations() {
    let classes = BTreeMap::from([
        ("html>body>button".into(), "control".into()),
        ("html>body>div".into(), "panel".into()),
    ]);
    let mut first = style("html>body>button", None, "transition: none;");
    first.media = Some("(prefers-reduced-motion: reduce)".into());
    let mut second = first.clone();
    second.target = "html>body>div".into();
    let mut css = String::new();
    append(&[first, second], &classes, &BTreeMap::new(), &mut css);
    assert_eq!(css.matches("@media").count(), 1);
    assert!(css.contains(".control,.panel{transition: none;}"));
}

#[test]
fn compounds_animation_classes_in_reduced_motion_selectors() {
    let classes = BTreeMap::from([("html>body>button".into(), "control animation".into())]);
    let mut reduced = style("html>body>button", None, "animation: none !important;");
    reduced.media = Some("(prefers-reduced-motion: reduce)".into());
    let mut css = String::new();
    append(&[reduced], &classes, &BTreeMap::new(), &mut css);
    assert!(css.contains(".control.animation{animation: none !important;}"));
    assert!(!css.contains(".control animation"));
}

#[test]
fn emits_dynamic_pseudo_element_state_selector() {
    let classes = BTreeMap::from([("html>body>button".into(), "control".into())]);
    let mut css = String::new();
    append(
        &[style(
            "html>body>button",
            Some(":hover::before"),
            "content: \"hover\";",
        )],
        &classes,
        &BTreeMap::new(),
        &mut css,
    );
    assert!(css.contains(".control:hover::before{content: \"hover\";}"));
}

#[test]
fn remaps_inherited_state_style_to_changed_interaction_class() {
    let styles = [style("html>body>button", Some(":hover"), "opacity: 0.8;")];
    let base = BTreeMap::from([("html>body>button".into(), "base".into())]);
    let changed = BTreeMap::from([("html>body>button".into(), "changed".into())]);
    let mut css = String::new();
    append_inherited(
        &styles,
        &base,
        &[(&[], &changed)],
        &BTreeMap::new(),
        &mut css,
    );
    assert_eq!(css.matches("opacity: 0.8;").count(), 1);
    assert!(css.contains(".base:hover,.changed:hover{opacity: 0.8;}"));
}

#[test]
fn interaction_override_replaces_inherited_state_style() {
    let base_style = style("html>body>button", Some(":hover"), "opacity: 0.8;");
    let override_style = style("html>body>button", Some(":hover"), "opacity: 1;");
    let base = BTreeMap::from([("html>body>button".into(), "base".into())]);
    let changed = BTreeMap::from([("html>body>button".into(), "changed".into())]);
    let mut css = String::new();
    append_inherited(
        &[base_style],
        &base,
        &[(&[override_style], &changed)],
        &BTreeMap::new(),
        &mut css,
    );
    assert!(!css.contains(".changed:hover"));
}

#[test]
fn preserves_authored_cascade_order_while_grouping() {
    let classes = BTreeMap::from([("html>body>button".into(), "control".into())]);
    let mut css = String::new();
    append(
        &[
            style("html>body>button", Some(":hover"), "opacity: 0.8;"),
            style("html>body>button", Some(":hover"), "opacity: 0.6;"),
        ],
        &classes,
        &BTreeMap::new(),
        &mut css,
    );
    assert!(css.find("opacity: 0.8;").unwrap() < css.find("opacity: 0.6;").unwrap());
}
