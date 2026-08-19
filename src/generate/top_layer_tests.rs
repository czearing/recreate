//! Top-layer membership: what separates an element the page promoted from one it merely
//! rendered, in the emitted files.
//!
//! The failure this pins is a *collapse*, not a missing field. `show()`, `showModal()` and a
//! hand-authored `<dialog open>` set the identical `open` attribute, and a popover the page
//! invoked carries the same `popover` attribute as one it never invoked, so two elements
//! differing only in how they were shown produced byte-identical props and the recreation
//! rendered both in flow, under any positioned content, with no scrim. Every test here
//! therefore asserts on the *pair*: one member alone can be satisfied by a fix that promotes
//! everything.
//!
//! Stated over the promotion rather than over dialogs. A test named for one element cannot
//! fail for another, which is how the same collapse survived one construct over.

use super::jsx_attrs;
use crate::model::Node;
use crate::top_layer::Promotion;

/// Two elements alike in every recorded field except the one under study.
fn pair(tag: &str, attribute: &str, reason: &str) -> (Node, Node) {
    let rendered = Node {
        tag: tag.into(),
        attributes: [(attribute.to_string(), String::new())]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    let mut promoted = rendered.clone();
    promoted.promotion = Promotion(reason.into());
    (rendered, promoted)
}

fn dialogs() -> (Node, Node) {
    pair("dialog", "open", "modal")
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
    assert!(promoted_props.contains(" data-recreate-promotion=\"showModal\""));
    assert!(!shown_props.contains("data-recreate-promotion"));
}

/// The same collapse one construct over, and the one the boolean could never have caught. A
/// popover the page invoked is in the top layer and is deliberately never `:modal`, so a
/// record spelled from inertness made it identical to a popover that was never invoked — the
/// element the page painted and the element it never showed emitting the same props. The
/// marker must also carry a *different* call, because `showModal()` on a popover is not the
/// thing the page did.
#[test]
fn separates_a_popover_the_page_invoked_from_one_it_never_showed() {
    let (closed, promoted) = pair("div", "popover", "popover");
    let assets = Default::default();

    let closed_props = jsx_attrs::attributes(&closed, &assets);
    let promoted_props = jsx_attrs::attributes(&promoted, &assets);

    assert_ne!(
        closed_props, promoted_props,
        "an invoked popover and one that was never shown emitted the same props"
    );
    assert!(promoted_props.contains(" data-recreate-promotion=\"showPopover\""));
    assert!(!closed_props.contains("data-recreate-promotion"));
}

/// A popover that was never invoked must stay hidden, so its `popover` attribute survives
/// untouched and nothing marks it for replay. Pinned beside the case above because a repair
/// that promotes every popover satisfies that one and breaks this.
#[test]
fn leaves_a_popover_the_page_never_showed_closed() {
    let (closed, _) = pair("div", "popover", "popover");

    let props = jsx_attrs::attributes(&closed, &Default::default());

    assert!(props.contains("popover"), "{props}");
    assert!(!props.contains("data-recreate-promotion"), "{props}");
}

/// Fullscreen is recorded and never replayed: `requestFullscreen()` needs transient user
/// activation a recreation rendering itself does not have. It therefore carries no marker,
/// and it must not borrow another entrant's call — the clause that forces the record to name
/// the reason instead of widening a predicate to "in the top layer".
#[test]
fn emits_no_call_for_a_promotion_that_has_none() {
    let (_, fullscreen) = pair("div", "id", "fullscreen");

    let props = jsx_attrs::attributes(&fullscreen, &Default::default());

    assert!(!props.contains("data-recreate-promotion"), "{props}");
    assert!(!props.contains("showModal"), "{props}");
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
    assert!(props.contains("data-recreate-promotion"));
}

/// The promotion is recorded as a fact about the element, never as a tag test, so the
/// emitter says nothing about which elements can be promoted. The scene's own losses were a
/// `<dialog popover>` and a `<div popover>`, so a repair that reads the tag repairs one.
#[test]
fn marks_promotion_without_naming_an_element() {
    let mut node = Node {
        tag: "div".into(),
        promotion: Promotion("popover".into()),
        ..Default::default()
    };

    assert!(jsx_attrs::attributes(&node, &Default::default()).contains("data-recreate-promotion"));

    node.promotion = Promotion::default();
    assert!(!jsx_attrs::attributes(&node, &Default::default()).contains("data-recreate-promotion"));
}

/// The recreation restores the promotion by making the call the page made, because no prop
/// can: `<dialog open>` is defined to be non-modal and a popover has no open attribute at
/// all. The call is read off the marker rather than chosen here, so the runtime carries no
/// branch per entrant and an element promoted by a route with no replay is never marked.
#[test]
fn replays_the_promotion_as_the_call_the_page_made() {
    let template = include_str!("templates/app_component.jsx");

    assert!(
        template.contains("[data-recreate-promotion=\"${call}\"]"),
        "{template}"
    );
    assert!(template.contains("element[call]?.()"));
    // Naming a call here would be the branch the marker exists to remove, and would reach
    // whichever entrant this file happened to be written against.
    assert!(!template.contains("showModal"), "{template}");
    assert!(!template.contains("showPopover"), "{template}");
    // A large z-index would order the dialog inside its own stacking context and still paint
    // under positioned content in another, which is the repair this rules out by construction.
    assert!(!template.contains("2147483647"));
}

/// One promotion can delete another, so replaying them all is not the same as replaying them
/// in any order. Showing a dialog modally hides every auto popover in the document, so a
/// recreation that walks the document opens the popover, then opens the modal, then has a
/// popover it opened and closed again — indistinguishable in the output from the one the page
/// never opened at all. The order is a property of the calls, taken from their one owner.
#[tokio::test]
async fn replays_a_dismissing_promotion_before_the_one_it_would_dismiss() {
    let directory = tempfile::tempdir().unwrap();
    super::write_project(
        &super::project_test_support::specification(),
        directory.path(),
        &[],
    )
    .await
    .unwrap();

    let app = std::fs::read_to_string(directory.path().join("react/src/App.jsx")).unwrap();
    let calls = app
        .split_once("for(const call of ")
        .expect("the replay iterates the owner's ordered call list")
        .1;

    assert!(
        calls.starts_with("[\"showModal\",\"showPopover\"]"),
        "{}",
        &calls[..calls.len().min(80)]
    );
}
