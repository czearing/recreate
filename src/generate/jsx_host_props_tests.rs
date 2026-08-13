//! A captured attribute the host framework cannot honour where it was captured.
//!
//! These tests assert the emitted JSX text, because the defect they guard is invisible to a
//! presence check: `selected={true}` reaches the file intact and React discards it at render.
//! The proof is therefore the conjunction — the inert prop gone, the compensating prop
//! present on the ancestor React actually reads.

use super::jsx_render::render;
use super::tree::{Component, Components};
use crate::model::{Node, Rect};
use std::collections::BTreeMap;

pub(super) fn node(path: &str, parent: Option<&str>, tag: &str, text: &str) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: parent.map(Into::into),
        tag: tag.into(),
        text: text.into(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        },
        style: Default::default(),
        before: None,
        after: None,
        ..Default::default()
    }
}

/// Builds a `<select>` whose options are described as `(value attribute, text, selected)`.
/// A `None` value attribute exercises the spec fallback to the option's text content.
fn select(options: &[(Option<&str>, &str, bool)], multiple: bool) -> Components {
    let mut nodes = BTreeMap::new();
    let mut children = BTreeMap::new();
    let mut select = node("s", None, "select", "");
    if multiple {
        select.attributes.insert("multiple".into(), String::new());
    }
    nodes.insert("s".to_string(), select);
    let mut option_paths = Vec::new();
    for (index, (value, text, selected)) in options.iter().enumerate() {
        let path = format!("s>option:nth-of-type({})", index + 1);
        let text_path = format!("{path}>text");
        let mut option = node(&path, Some("s"), "option", "");
        if let Some(value) = value {
            option.attributes.insert("value".into(), (*value).into());
        }
        if *selected {
            option.attributes.insert("selected".into(), String::new());
        }
        nodes.insert(path.clone(), option);
        nodes.insert(
            text_path.clone(),
            node(&text_path, Some(&path), "#text", text),
        );
        children.insert(path.clone(), vec![text_path]);
        option_paths.push(path);
    }
    children.insert("s".to_string(), option_paths);
    Components {
        items: Vec::<Component>::new(),
        by_root: BTreeMap::new(),
        children,
        classes: BTreeMap::new(),
        nodes,
    }
}

pub(super) fn rendered(components: &Components) -> String {
    render(
        "s",
        components,
        &Default::default(),
        0,
        true,
        &Default::default(),
    )
}

#[test]
fn relocates_selection_from_option_to_select() {
    let output = rendered(&select(
        &[
            (Some("bronze"), "Bronze plan", false),
            (Some("silver"), "Silver plan", false),
            (Some("gold"), "Gold plan", true),
        ],
        false,
    ));
    assert!(
        output.contains("defaultValue={\"gold\"}"),
        "select did not adopt the selected option's value: {output}"
    );
    assert!(
        !output.contains("selected"),
        "inert selected prop survived into the emitted JSX: {output}"
    );
}

/// An option with no `value` attribute takes its value from its text content, which is the
/// commonest authoring form. Reading the attribute alone would emit an empty selection.
#[test]
fn takes_the_option_value_from_its_text_when_the_attribute_is_absent() {
    let output = rendered(&select(
        &[(None, "First", false), (None, "Second", true)],
        false,
    ));
    assert!(
        output.contains("defaultValue={\"Second\"}"),
        "value did not fall back to the option's text: {output}"
    );
}

/// React requires an array on a multiple select, and a bare string silently selects nothing.
#[test]
fn adopts_every_selection_as_a_list_for_a_multiple_select() {
    let output = rendered(&select(
        &[
            (Some("a"), "A", true),
            (Some("b"), "B", false),
            (Some("c"), "C", true),
        ],
        true,
    ));
    assert!(
        output.contains("defaultValue={[\"a\",\"c\"]}"),
        "multiple select did not adopt a list: {output}"
    );
}

/// Browser and React both display the first option when nothing is marked, so emitting a
/// selection here would be noise rather than fidelity.
#[test]
fn adopts_nothing_when_no_option_is_selected() {
    let output = rendered(&select(
        &[(Some("a"), "A", false), (Some("b"), "B", false)],
        false,
    ));
    assert!(
        !output.contains("defaultValue"),
        "emitted a selection the source never made: {output}"
    );
}

/// Options are routinely nested inside `<optgroup>`, so the search must be a descendant walk.
#[test]
fn finds_a_selected_option_nested_in_a_group() {
    let mut components = select(&[(Some("a"), "A", false), (Some("b"), "B", true)], false);
    let group = node("s>optgroup:nth-of-type(1)", Some("s"), "optgroup", "");
    let moved = components.children.remove("s").unwrap();
    components.nodes.insert(group.path.clone(), group);
    components
        .children
        .insert("s>optgroup:nth-of-type(1)".into(), moved);
    components
        .children
        .insert("s".into(), vec!["s>optgroup:nth-of-type(1)".into()]);
    let output = rendered(&components);
    assert!(
        output.contains("defaultValue={\"b\"}"),
        "a grouped option's selection was not adopted: {output}"
    );
}
