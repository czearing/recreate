use super::*;

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

/// The wrapper axis is one half of the grid; this is the other.
///
/// `retain` classifies a nested rule by itself rather than by what encloses it, so the set
/// of definition kinds it carries is whatever `global_rule` accepts, with no per-construct
/// branch to fall out of date. Asserting that across kinds is what separates a real fix from
/// one that special-cases the construct that happened to be reported: `@keyframes` and
/// `@font-face` are covered elsewhere, and `@property` and `@counter-style` are the kinds
/// nothing exercised — both register a name that a computed style can only point at, so
/// losing either is the same silent failure under a different spelling.
#[test]
fn a_conditional_group_carries_every_kind_of_definition_it_holds() {
    for definition in [
        "@property --shade{syntax:\"<color>\";inherits:false;initial-value:red;}",
        "@counter-style ticks{system:cyclic;symbols:\"*\";}",
        "@font-face{font-family:Vorplish;src:url(a.woff2);}",
        "@keyframes spin{from{rotate:0deg;}}",
    ] {
        for wrapper in ["@media (min-width: 1px)", "@supports (rotate: 0deg)"] {
            let kept =
                retain(&format!("{wrapper}{{{definition}}}"), &mut global_rule).unwrap_or_default();
            assert!(
                kept.contains(definition),
                "{wrapper} dropped the definition it held: {kept:?}"
            );
            assert!(
                kept.starts_with(wrapper),
                "{wrapper} was flattened away, publishing an unconditional definition: {kept:?}"
            );
        }
    }
}

/// The converse, so widening reach does not become re-emitting the wrapper wholesale. A
/// style rule inside a group is already baked into a hashed class, so lifting it out with
/// its neighbouring definition would apply it a second time.
#[test]
fn a_conditional_group_carries_none_of_the_style_rules_it_holds() {
    let kept = retain(
        "@media (min-width: 1px){@keyframes spin{from{rotate:0deg;}}.card{color:red;}}",
        &mut global_rule,
    )
    .unwrap_or_default();
    assert!(kept.contains("@keyframes spin"), "{kept:?}");
    assert!(
        !kept.contains("color:red"),
        "a baked style rule was re-emitted: {kept:?}"
    );
}
