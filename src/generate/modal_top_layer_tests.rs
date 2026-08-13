//! Top-layer membership: what separates a dialog the page opened modally from one it merely
//! showed, in the emitted files.
//!
//! The failure this pins is a *collapse*, not a missing field. `show()`, `showModal()` and a
//! hand-authored `<dialog open>` set the identical `open` attribute, so two dialogs differing
//! only in how they were opened produced byte-identical props and the recreation rendered both
//! in flow, under any positioned content, with no scrim. Every test here therefore asserts on
//! the *pair*: one member alone can be satisfied by a fix that promotes everything.

use super::{css_pseudo, jsx_attrs};
use crate::model::{Node, Pseudo, Styles};

/// Two dialogs alike in every recorded field except the one under study.
fn dialogs() -> (Node, Node) {
    let shown = Node {
        tag: "dialog".into(),
        attributes: [("open".to_string(), String::new())].into_iter().collect(),
        ..Default::default()
    };
    let mut promoted = shown.clone();
    promoted.modal = true;
    (shown, promoted)
}

fn scrim() -> Pseudo {
    let mut style = Styles::new();
    style.insert("background-color".into(), "rgba(0, 0, 0, 0.5)".into());
    Pseudo {
        content: String::new(),
        style,
    }
}

/// The defect verbatim: the two dialogs must not emit the same props. Asserted as a
/// difference between the pair rather than as the presence of a named field, because the
/// identity is what was wrong — a fix that emits a marker on *both* is equally broken.
#[test]
fn separates_a_dialog_the_page_promoted_from_one_it_only_showed() {
    let (shown, promoted) = dialogs();
    let assets = Default::default();

    let shown_props = jsx_attrs::attributes(&shown, &assets);
    let promoted_props = jsx_attrs::attributes(&promoted, &assets);

    assert_ne!(
        shown_props, promoted_props,
        "both dialogs emitted the same props, so the recreation cannot distinguish a modal \
         from a floating panel"
    );
    assert!(promoted_props.contains(" data-recreate-modal={true}"));
    assert!(!shown_props.contains("data-recreate-modal"));
}

/// The non-modal dialog must keep rendering non-modal. A repair that promotes every open
/// dialog restores the reported scene and breaks the other half of the same page.
#[test]
fn leaves_a_dialog_the_page_only_showed_open_and_in_flow() {
    let (shown, _) = dialogs();

    let props = jsx_attrs::attributes(&shown, &Default::default());

    assert!(props.contains(" open={true}"));
}

/// `open` is not merely redundant on a promoted dialog: React would open it non-modally
/// during render, and `showModal()` on an already-open dialog throws, so the attribute
/// defeats the replay rather than duplicating it. The marker must arrive in its place.
#[test]
fn withholds_the_open_attribute_from_a_dialog_the_runtime_must_promote() {
    let (_, promoted) = dialogs();

    let props = jsx_attrs::attributes(&promoted, &Default::default());

    assert!(!props.contains("open"), "{props}");
    assert!(props.contains("data-recreate-modal"));
}

/// The promotion is recorded as a fact about the element, never as a tag test, so the
/// emitter says nothing about which elements can be promoted. `:modal` also matches a
/// fullscreen element, and the runtime — not this table — is what lets one self-exclude.
#[test]
fn marks_promotion_without_naming_an_element() {
    let mut node = Node {
        tag: "div".into(),
        modal: true,
        ..Default::default()
    };

    assert!(jsx_attrs::attributes(&node, &Default::default()).contains("data-recreate-modal"));

    node.modal = false;
    assert!(!jsx_attrs::attributes(&node, &Default::default()).contains("data-recreate-modal"));
}

/// The scrim is authored CSS on a box only a promoted element has. It reaches the output as
/// captured computed style on that element, the way `::before` already does — not by widening
/// the selector filter, which keys classes on the element's own computed style and would leak
/// one element's scrim onto another sharing that class.
#[test]
fn writes_the_authored_scrim_of_a_promoted_element() {
    let (_, mut promoted) = dialogs();
    promoted.pseudos.insert("::backdrop".into(), scrim());
    let mut css = String::new();

    css_pseudo::append(&promoted, "c1", &Default::default(), &mut css);

    assert!(css.contains(".c1::backdrop{"), "{css}");
    assert!(css.contains("background-color:rgba(0, 0, 0, 0.5)"), "{css}");
}

/// A user-agent generated box has no `content` of its own, so declaring one asserts something
/// the page never said. This is the artifact the `::before` shape would have produced, since
/// that slot always writes its reason for existing.
#[test]
fn declares_no_content_for_a_box_the_user_agent_generated() {
    let css = css_pseudo::declarations(&scrim(), &Default::default());

    assert!(!css.contains("content"), "{css}");
}

/// A box that exists only because `content` produced it still declares that value, and
/// exactly once. Pinned beside the case above so a fix for one cannot silently drop the other.
#[test]
fn still_declares_content_for_a_box_that_content_generated() {
    let mut style = Styles::new();
    style.insert("content".into(), "\"MARK\"".into());
    style.insert("color".into(), "red".into());
    let marker = Pseudo {
        content: "\"MARK\"".into(),
        style,
    };

    let css = css_pseudo::declarations(&marker, &Default::default());

    assert_eq!(css.matches("content:").count(), 1, "{css}");
    assert!(css.starts_with("content:\"MARK\";"), "{css}");
}

/// The recreation restores the promotion by making the call the page made, because no prop
/// can: `<dialog open>` is defined to be non-modal. Guarded on the marker so a dialog the
/// page only showed is never promoted, and asked of the element so a non-dialog self-excludes.
#[test]
fn replays_the_promotion_as_the_call_the_page_made() {
    let template = include_str!("templates/app_component.jsx");

    assert!(template.contains("document.querySelectorAll('[data-recreate-modal]')"));
    assert!(template.contains("if(!element.open)element.showModal?.()"));
    // A large z-index would order the dialog inside its own stacking context and still paint
    // under positioned content in another, which is the repair this rules out by construction.
    assert!(!template.contains("2147483647"));
}
