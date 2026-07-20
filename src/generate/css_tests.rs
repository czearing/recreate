use super::*;

#[test]
fn preserves_global_font_and_keyframe_rules() {
    assert!(global_rule("@font-face { font-family: Test; }"));
    assert!(global_rule("@keyframes pulse { from { opacity: 0 } }"));
    assert!(global_rule(
        "@-webkit-keyframes pulse { from { opacity: 0 } }"
    ));
    assert!(!global_rule(".card { color: red; }"));
}

#[test]
fn directional_border_contract_is_captured_and_generated() {
    let mut styles = Styles::new();
    for side in ["top", "right", "bottom", "left"] {
        for (property, value) in [
            ("width", "4px"),
            ("style", "solid"),
            ("color", "rgb(216, 168, 78)"),
        ] {
            let name = format!("border-{side}-{property}");
            assert!(
                crate::style_contract::contains(&name),
                "missing capture property {name}"
            );
            styles.insert(name, value.into());
        }
    }
    let css = declarations(&styles, &BTreeMap::new());
    for side in ["top", "right", "bottom", "left"] {
        assert!(css.contains(&format!("border-{side}-width:4px;")));
        assert!(css.contains(&format!("border-{side}-style:solid;")));
        assert!(css.contains(&format!("border-{side}-color:rgb(216, 168, 78);")));
    }
}

#[test]
fn grid_item_contract_is_captured_and_generated() {
    let mut styles = Styles::new();
    for (name, value) in [
        ("grid-column-start", "1"),
        ("grid-column-end", "-1"),
        ("grid-row-start", "auto"),
        ("grid-row-end", "auto"),
        ("justify-self", "start"),
    ] {
        assert!(crate::style_contract::contains(name));
        styles.insert(name.into(), value.into());
    }

    let css = declarations(&styles, &BTreeMap::new());
    for (name, value) in styles {
        assert!(css.contains(&format!("{name}:{value};")));
    }
}

#[test]
fn emits_unique_custom_properties_used_by_state_rules() {
    let mut css =
        ".card:hover{background:var(--brand);box-shadow:0 0 0 2px var(--focus);}".to_string();
    append_custom_property_fallbacks(
        &[
            ".provider{--brand:#242424;--focus:#0f6cbd;}".into(),
            ".other{--brand:#242424;}".into(),
        ],
        &mut css,
    );
    assert!(css.contains(":root{--brand:#242424;--focus:#0f6cbd;}"));
}

#[test]
fn rejects_ambiguous_custom_property_fallbacks() {
    let mut css = ".card{color:var(--brand);}".to_string();
    append_custom_property_fallbacks(
        &[
            ".light{--brand:#fff;}".into(),
            ".dark{--brand:#000;}".into(),
        ],
        &mut css,
    );
    assert!(!css.contains(":root"));
}

#[test]
fn clipped_text_keeps_responsive_captured_heights() {
    let mut specification = crate::generate::project_test_support::specification();
    let path = specification.states[0].nodes[3].path.clone();
    for (index, state) in specification.states.iter_mut().enumerate() {
        state.nodes[3].rect.height = 20.0 + index as f64 * 20.0;
        state.nodes[3]
            .style
            .insert("overflow".into(), "hidden".into());
    }
    assert!(!fluid_height_paths(&specification).contains(&path));

    for state in &mut specification.states {
        state.nodes[3].style.remove("overflow");
    }
    assert!(fluid_height_paths(&specification).contains(&path));
}
