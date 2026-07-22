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
fn float_contract_is_captured_and_generated() {
    assert!(crate::style_contract::contains("float"));
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
    for name in styles.keys() {
        assert!(crate::style_contract::contains(name));
    }
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

#[test]
fn compact_interaction_states_keep_baseline_authored_css() {
    let mut baseline = crate::generate::project_test_support::specification()
        .states
        .remove(0);
    baseline.css_rules = vec![".composer { width: 100%; }".into()];
    let mut interaction = baseline.clone();
    interaction.css_rules.clear();

    let merged = with_baseline_css(interaction, &baseline);

    assert_eq!(merged.css_rules, baseline.css_rules);
}

#[test]
fn sibling_topology_changes_rebuild_existing_child_classes() {
    let baseline = crate::generate::project_test_support::specification()
        .states
        .remove(0);
    let parent = baseline
        .nodes
        .iter()
        .filter_map(|node| node.parent.as_deref())
        .find(|parent| {
            baseline
                .nodes
                .iter()
                .filter(|node| node.parent.as_deref() == Some(*parent))
                .count()
                > 1
        })
        .unwrap();
    let existing = baseline
        .nodes
        .iter()
        .find(|node| node.parent.as_deref() == Some(parent))
        .unwrap()
        .clone();
    let mut state = baseline.clone();
    let mut inserted = existing.clone();
    inserted.path.push_str(">button:nth-of-type(99)");
    state.nodes.push(inserted);

    let changed = topology_changed_paths(&state, &baseline);

    assert!(changed.contains(&existing.path));
}

#[test]
fn sibling_geometry_changes_rebuild_existing_child_classes() {
    let baseline = crate::generate::project_test_support::specification()
        .states
        .remove(0);
    let parent = baseline
        .nodes
        .iter()
        .filter_map(|node| node.parent.as_deref())
        .find(|parent| {
            baseline
                .nodes
                .iter()
                .filter(|node| node.parent.as_deref() == Some(*parent))
                .count()
                > 1
        })
        .unwrap();
    let existing = baseline
        .nodes
        .iter()
        .find(|node| node.parent.as_deref() == Some(parent))
        .unwrap()
        .clone();
    let sibling = baseline
        .nodes
        .iter()
        .find(|node| node.parent == existing.parent && node.path != existing.path)
        .unwrap()
        .clone();
    let mut state = baseline.clone();
    state
        .nodes
        .iter_mut()
        .find(|node| node.path == sibling.path)
        .unwrap()
        .rect
        .width += 20.0;
    state
        .nodes
        .iter_mut()
        .find(|node| node.path == existing.path)
        .unwrap()
        .rect
        .x += 20.0;

    let changed = topology_changed_paths(&state, &baseline);

    assert!(changed.contains(&existing.path));
}

#[test]
fn contextual_widths_do_not_reuse_fluid_cache_entries() {
    let mut specification = crate::generate::project_test_support::text_entry_specification();
    let parent = specification.states[0]
        .nodes
        .iter()
        .find(|node| node.path == specification.interactions[0].trigger_path)
        .and_then(|node| node.parent.clone())
        .unwrap();
    let mut wrapper = specification.states[0].nodes[5].clone();
    wrapper.path = format!("{parent}>div:nth-of-type(3)");
    wrapper.parent = Some(parent);
    wrapper.rect.width = 36.0;
    wrapper.rect.height = 36.0;
    wrapper.style.insert("width".into(), "100%".into());
    wrapper.style.insert("position".into(), "static".into());
    specification.states[0].nodes.push(wrapper.clone());
    specification.interactions[0].states[0]
        .nodes
        .push(wrapper.clone());
    specification.interactions[0].states[0]
        .nodes
        .last_mut()
        .unwrap()
        .rect
        .x += 44.0;
    let mut prior = specification.interactions[0].clone();
    prior.states = vec![specification.states[0].clone()];
    specification.interactions.insert(0, prior);

    let output = build(&specification, &BTreeMap::new());
    let fluid = &output.interaction_classes[0][&wrapper.path];
    let contextual = &output.interaction_classes[1][&wrapper.path];
    let declaration = output
        .css
        .split(&format!(".{contextual}{{"))
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();

    assert_ne!(fluid, contextual);
    assert!(declaration.contains("width:36px;"));
}
