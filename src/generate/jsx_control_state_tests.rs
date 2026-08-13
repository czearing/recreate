//! A control whose live state diverges from the markup default it was authored with.
//!
//! These tests assert the emitted binding rather than the presence of the value string,
//! because three distinct wrong emissions all contain it. The value can survive as the
//! markup default (`MARKUP-ALPHA` where the page shows something else), as a `value` prop
//! with no `onChange` (React pins the field read-only), or as a synthetic text child of a
//! void element (renders as stray text beside an empty control). All three paint a
//! plausible page, so only a check on which prop holds the string separates them.

use super::jsx_host_props_tests::{node, rendered};
use super::tree::{Component, Components};
use std::collections::BTreeMap;

/// A single control rooted at `s`, described as its tag, its markup attributes and the live
/// state the page ended up in.
fn control(tag: &str, attributes: &[(&str, &str)], live: &[(&str, Option<&str>)]) -> Components {
    let mut control = node("s", None, tag, "");
    for (name, value) in attributes {
        control
            .attributes
            .insert((*name).into(), (*value).to_string());
    }
    for (name, value) in live {
        control
            .control_state
            .insert((*name).into(), value.map(str::to_owned));
    }
    let mut nodes = BTreeMap::new();
    nodes.insert("s".to_string(), control);
    Components {
        items: Vec::<Component>::new(),
        by_root: BTreeMap::new(),
        children: BTreeMap::new(),
        classes: BTreeMap::new(),
        nodes,
    }
}

/// The defect this file exists for: the page showed one string and the markup authored
/// another, and only the markup one reached the file.
#[test]
fn seeds_the_input_from_the_live_value_rather_than_the_markup_default() {
    let output = rendered(&control(
        "input",
        &[("value", "MARKUP-ALPHA")],
        &[("value", Some("LIVE-BRAVO"))],
    ));
    assert!(
        output.contains("defaultValue={\"LIVE-BRAVO\"}"),
        "live value was not bound as the uncontrolled default: {output}"
    );
    assert!(
        !output.contains("MARKUP-ALPHA"),
        "the superseded markup default was emitted as well: {output}"
    );
}

/// A `value` prop without an `onChange` makes the control read-only in React while painting
/// the identical pixel, so the rename is load-bearing rather than cosmetic. This must hold
/// for a control the page never touched too, which is the case with no live state at all.
#[test]
fn never_emits_a_controlled_value_prop() {
    for live in [&[("value", Some("LIVE-BRAVO"))][..], &[][..]] {
        let output = rendered(&control("input", &[("value", "MARKUP-ALPHA")], live));
        assert!(
            !output.contains(" value="),
            "emitted a controlled value prop, freezing the control: {output}"
        );
        assert!(
            output.contains("defaultValue="),
            "the control lost its value entirely: {output}"
        );
    }
}

/// An untouched control still needs its authored default carried across, so the fix must not
/// make the markup path conditional on a live reading being present.
#[test]
fn falls_back_to_the_markup_default_when_the_page_never_changed_it() {
    let output = rendered(&control("input", &[("value", "MARKUP-ALPHA")], &[]));
    assert!(
        output.contains("defaultValue={\"MARKUP-ALPHA\"}"),
        "an untouched control lost its authored value: {output}"
    );
}

/// A checkbox is the case an absent entry cannot express. The markup authored `checked`, the
/// page cleared it, and a record that merely omitted the live state would let the attribute
/// win and re-check the box.
#[test]
fn clears_a_checkbox_the_page_turned_off() {
    let output = rendered(&control(
        "input",
        &[("type", "checkbox"), ("checked", "")],
        &[("checked", None)],
    ));
    assert!(
        !output.contains("defaultChecked"),
        "a cleared checkbox was still emitted as checked: {output}"
    );
    assert!(
        !output.contains(" checked"),
        "the superseded checked attribute survived: {output}"
    );
}

/// The opposite direction, and the reason the boolean is not spelled the way HTML spells it:
/// React reads `defaultChecked=""` as false, so the empty string that means "present" in an
/// attribute would silently mean "off" as a prop.
#[test]
fn checks_a_box_the_page_turned_on() {
    let output = rendered(&control(
        "input",
        &[("type", "checkbox")],
        &[("checked", Some(""))],
    ));
    assert!(
        output.contains("defaultChecked={true}"),
        "a live-checked box was not emitted as checked: {output}"
    );
}

/// Selection is the case where the state and the prop live on different elements, so the
/// live reading has to reach the ancestor's prop. A markup-only source would emit nothing
/// here, since the page authored no `selected` attribute at all.
#[test]
fn adopts_a_selection_the_page_made_and_the_markup_never_authored() {
    let mut components = control("select", &[], &[]);
    for (index, (value, live)) in [("bronze", false), ("gold", true)].iter().enumerate() {
        let path = format!("s>option:nth-of-type({})", index + 1);
        let mut option = node(&path, Some("s"), "option", "");
        option.attributes.insert("value".into(), (*value).into());
        if *live {
            option
                .control_state
                .insert("selected".into(), Some(String::new()));
        }
        components.nodes.insert(path.clone(), option);
        components
            .children
            .entry("s".into())
            .or_default()
            .push(path);
    }
    let output = rendered(&components);
    assert!(
        output.contains("defaultValue={\"gold\"}"),
        "a live selection never reached the select's prop: {output}"
    );
}

/// Suppressing children is licensed by the value arriving as a prop, not by the element
/// being a control. A `<textarea>` the page never touched still spells its value as its
/// children, and dropping them would erase the content outright.
#[test]
fn keeps_the_children_of_a_control_whose_value_is_not_bound() {
    let mut components = control("textarea", &[], &[]);
    let text = node("s>text", Some("s"), "#text", "MARKUP-ALPHA");
    components.nodes.insert("s>text".into(), text);
    components
        .children
        .insert("s".into(), vec!["s>text".to_string()]);
    let output = rendered(&components);
    assert!(
        output.contains("MARKUP-ALPHA"),
        "an untouched textarea lost its content: {output}"
    );
}

/// textarea that carries both children and `defaultValue`. So binding the value has to
/// suppress the children that used to spell it.
#[test]
fn moves_a_textarea_value_out_of_its_children() {
    let mut components = control("textarea", &[], &[("value", Some("LIVE-BRAVO"))]);
    let text = node("s>text", Some("s"), "#text", "MARKUP-ALPHA");
    components.nodes.insert("s>text".into(), text);
    components
        .children
        .insert("s".into(), vec!["s>text".to_string()]);
    let output = rendered(&components);
    assert!(
        output.contains("defaultValue={\"LIVE-BRAVO\"}"),
        "textarea value was not bound as a prop: {output}"
    );
    assert!(
        !output.contains("MARKUP-ALPHA"),
        "textarea kept the children React refuses alongside defaultValue: {output}"
    );
}
