use super::*;
use std::collections::BTreeSet;

#[test]
fn preserves_global_font_and_keyframe_rules() {
    assert!(global_rule("@font-face { font-family: Test; }"));
    assert!(global_rule("@keyframes pulse { from { opacity: 0 } }"));
    assert!(global_rule(
        "@-webkit-keyframes pulse { from { opacity: 0 } }"
    ));
    assert!(!global_rule(".card { color: red; }"));
}

/// Every definition at-rule names an entity a computed style refers to by name and cannot
/// carry, so the emitter keeps them all rather than the three that happened to be listed.
/// `@counter-style` is the discriminator: an allow-list extended by one string passes the
/// `@property` case and still fails here.
#[test]
fn preserves_every_at_rule_that_defines_a_name_a_computed_style_cannot_carry() {
    assert!(global_rule(
        "@property --angle { syntax: '<angle>'; initial-value: 0deg; inherits: false; }"
    ));
    assert!(global_rule("@counter-style dashes { system: cyclic; }"));
    assert!(global_rule(
        "@font-feature-values Font One { @styleset { nice: 1 } }"
    ));
    assert!(global_rule(
        "@font-palette-values --pal { font-family: Test; }"
    ));
    assert!(global_rule("@page { margin: 1cm; }"));
}

/// A grouping rule's body is style rules whose winner `getComputedStyle` already returned
/// and the emitter already baked into a hashed class, so re-emitting one double-applies.
#[test]
fn discards_grouping_rules_whose_effect_the_baked_classes_already_carry() {
    assert!(!global_rule(
        "@media (min-width: 1px) { .dial { color: red } }"
    ));
    assert!(!global_rule(
        "@supports (display: grid) { .dial { display: grid } }"
    ));
    assert!(!global_rule(
        "@container (min-width: 1px) { .dial { color: red } }"
    ));
    assert!(!global_rule("@layer base { .dial { color: red } }"));
    assert!(!global_rule("@scope (.a) { .dial { color: red } }"));
    assert!(!global_rule("@starting-style { .dial { opacity: 0 } }"));
}

/// A statement at-rule has no block. Every one of them is position-constrained, and
/// `@import` names a sheet the capture already walked and baked, so re-emitting it would
/// refetch the sheet and apply every rule in it a second time.
#[test]
fn discards_statement_at_rules_that_carry_placement_rather_than_definition() {
    assert!(!global_rule("@import url(\"palette.css\");"));
    assert!(!global_rule("@charset \"utf-8\";"));
    assert!(!global_rule(
        "@namespace svg url(http://www.w3.org/2000/svg);"
    ));
    assert!(!global_rule("@layer base, components;"));
}

/// Layer membership is a wrapper the capture rebuilds around every rule it records, so a
/// definition authored inside a layer must be judged by what it defines and a style rule
/// inside one must still be discarded.
#[test]
fn judges_a_layered_rule_by_the_rule_the_layer_wraps() {
    assert!(global_rule(
        "@layer base { @font-face { font-family: Test; } }"
    ));
    assert!(global_rule(
        "@layer a { @layer b { @property --x { syntax: '*'; } } }"
    ));
    assert!(!global_rule(
        "@layer a { @layer b { .card { color: red } } }"
    ));
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
        &BTreeSet::new(),
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
        &BTreeSet::new(),
        &mut css,
    );
    assert!(!css.contains(":root"));
}

/// The captured layer states a name once per viewport condition. Restating it
/// unconditionally duplicates the rule and reasserts, below a breakpoint, a
/// value the source declared only above it.
#[test]
fn leaves_a_captured_custom_property_to_the_captured_layer() {
    let mut css = ".card{width:var(--gap);height:var(--edge);}".to_string();
    append_custom_property_fallbacks(
        &[".provider{--gap:8px;--edge:2px;}".into()],
        &BTreeSet::from(["--gap".to_string()]),
        &mut css,
    );
    assert!(css.contains(":root{--edge:2px;}"), "{css}");
    assert!(!css.contains("--gap:"), "{css}");
}

/// Inverse guard: owning every referenced name must not silence the fallback
/// entirely, and owning none must leave it exactly as it was.
#[test]
fn keeps_every_fallback_the_captured_layer_never_declares() {
    let mut css = ".card{width:var(--gap);}".to_string();
    append_custom_property_fallbacks(
        &[".provider{--gap:8px;}".into()],
        &BTreeSet::new(),
        &mut css,
    );
    assert!(css.contains(":root{--gap:8px;}"), "{css}");
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
