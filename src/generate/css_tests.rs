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
fn interaction_paint_overrides_authored_important_rules() {
    assert_eq!(
        important_interaction_paint(
            "color:white;fill:currentColor;background-color:black;width:10px;"
        ),
        "color:white!important;fill:currentColor!important;background-color:black!important;width:10px;"
    );
}

#[test]
fn rewrites_longer_protocol_relative_asset_urls_first() {
    let assets = BTreeMap::from([
        (
            "https://cdn.example/font.woff".to_string(),
            "/assets/font.woff".to_string(),
        ),
        (
            "https://cdn.example/font.woff2".to_string(),
            "/assets/font.woff2".to_string(),
        ),
    ]);
    assert_eq!(
        rewrite_rule_assets(
            r#"src:url("//cdn.example/font.woff2"),url("//cdn.example/font.woff")"#,
            &assets,
        ),
        r#"src:url("/assets/font.woff2"),url("/assets/font.woff")"#
    );
}

#[test]
fn rewrites_a_font_the_stylesheet_wrote_as_a_root_relative_path() {
    let assets = BTreeMap::from([(
        "https://local.example:8080/assets/font/segoe-sans.711fd8a54c.woff2".to_string(),
        "/assets/segoe-sans.woff2".to_string(),
    )]);
    assert_eq!(
        rewrite_rule_assets(
            r#"src:url("/assets/font/segoe-sans.711fd8a54c.woff2") format("woff2")"#,
            &assets,
        ),
        r#"src:url("/assets/segoe-sans.woff2") format("woff2")"#
    );
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
fn float_contract_is_captured_and_generated() {
    let styles = Styles::from([("float".into(), "right".into())]);
    assert_eq!(declarations(&styles, &BTreeMap::new()), "float:right;");
}

#[test]
fn svg_paint_contract_is_captured_and_generated() {
    let styles = Styles::from([
        ("fill".into(), "rgb(198, 225, 255)".into()),
        ("stroke".into(), "rgba(0, 0, 0, 0.427)".into()),
        ("stroke-width".into(), "1px".into()),
    ]);
    let css = declarations(&styles, &BTreeMap::new());
    assert!(css.contains("fill:rgb(198, 225, 255);"));
    assert!(css.contains("stroke:rgba(0, 0, 0, 0.427);"));
    assert!(css.contains("stroke-width:1px;"));
}

#[test]
fn emits_custom_properties_referenced_only_by_attributes() {
    let mut specification = crate::generate::project_test_support::specification();
    specification.states[0].nodes[1]
        .attributes
        .insert("fill".into(), "var(--card-fill)".into());
    specification.states[0].css_rules = vec![":root { --card-fill: rgb(198, 225, 255); }".into()];

    let output = build(&specification, &BTreeMap::new());

    assert!(
        output
            .css
            .contains(":root{--card-fill:rgb(198, 225, 255);}")
    );
}

#[test]
fn infers_missing_right_float_from_captured_geometry() {
    let mut parent =
        crate::generate::project_test_support::specification().states[0].nodes[1].clone();
    parent.rect.x = 20.0;
    parent.rect.width = 190.0;
    parent.style.insert("display".into(), "block".into());
    let mut node = parent.clone();
    node.rect.x = 210.0;
    node.rect.width = 0.0;
    node.style.insert("position".into(), "static".into());

    assert_eq!(visual_float(&node, Some(&parent)), Some("right"));
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
