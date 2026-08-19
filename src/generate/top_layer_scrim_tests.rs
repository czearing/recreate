//! The scrim a promoted element gets, in the emitted CSS.
//!
//! `::backdrop` exists because the element is in the top layer, not because anything authored
//! `content` for it, so it reaches the output on a different condition from every other
//! pseudo-element the emitter writes. Split from the props and replay tests beside it because
//! it is a separate stage: those decide what the element becomes, this decides what its box
//! declares.

use super::css_pseudo;
use crate::model::{Node, Pseudo, Styles};
use crate::top_layer::Promotion;

fn scrim() -> Pseudo {
    let mut style = Styles::new();
    style.insert("background-color".into(), "rgba(0, 0, 0, 0.5)".into());
    Pseudo {
        content: String::new(),
        style,
    }
}

/// The scrim is authored CSS on a box only a promoted element has. It reaches the output as
/// captured computed style on that element, the way `::before` already does — not by widening
/// the selector filter, which keys classes on the element's own computed style and would leak
/// one element's scrim onto another sharing that class.
///
/// Written for a popover, because that is the promotion whose scrim was never even looked
/// for: the engine generates the box for the top layer, and a gate spelled from inertness
/// answers no for the widest member of it.
#[test]
fn writes_the_authored_scrim_of_a_promoted_element() {
    let mut promoted = Node {
        tag: "div".into(),
        promotion: Promotion("popover".into()),
        ..Default::default()
    };
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
