//! An emitted declaration's job is not to describe what the source painted. It is to beat
//! the user-agent rule that applies again when the recreation re-emits the same tag.
//!
//! That makes a reset whose value equals the CSS initial value load-bearing rather than
//! redundant. Omitting `border-style: none` from a `<button>`'s class does not leave the
//! button with no border; it leaves it with the user agent's `2px outset`, four pixels
//! taller than the source. Which values are redundant depends on the element's
//! surroundings, and the emitter cannot see them - the capture already answered the
//! question by measuring against the element's no-author-CSS baseline.

use crate::generate::responsive::{base_declarations, output_declarations};
use crate::generate::style_reversion_tests::box_node;
use crate::model::{Node, Styles, Viewport};
use std::collections::BTreeMap;

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    dpr: 1.0,
};

fn render(pairs: &[(&str, &str)]) -> String {
    let styles: Styles = pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    output_declarations(&styles, &BTreeMap::new())
}

/// The fixture the old test could not build. A bare property map has no element, so it
/// cannot express the only case that matters: one whose user agent supplies the very
/// thing the author removed.
fn emitted(tag: &str, pairs: &[(&str, &str)]) -> String {
    let node = Node {
        tag: tag.into(),
        ..box_node("html>body>form>control", pairs)
    };
    base_declarations(&node, None, &VIEWPORT, &BTreeMap::new(), &[], false)
}

/// The filed defect, at the three tags whose user-agent stylesheet draws a border the
/// author has to remove by hand: `<button>` is `2px outset`, `<input>` `2px inset`,
/// `<fieldset>` `2px groove`. Each arm's reset must reach the class, and its width and
/// colour must not come back with it - the repair may not restore the duplication the
/// deletion was written to remove.
#[test]
fn a_reset_of_a_user_agent_border_reaches_the_stylesheet() {
    for tag in ["button", "input", "fieldset"] {
        let css = emitted(
            tag,
            &[
                ("border-top-style", "none"),
                ("border-top-width", "0px"),
                ("border-top-color", "rgb(16, 16, 16)"),
                ("padding-top", "8px"),
            ],
        );
        assert!(css.contains("border-top-style:none"), "{tag}: {css}");
        assert!(!css.contains("border-top-width"), "{tag}: {css}");
        assert!(!css.contains("border-top-color"), "{tag}: {css}");
        assert!(css.contains("padding-top:8px"), "{tag}: {css}");
    }
}

/// `hidden` is not a spelling of `none`. It differs from it in border-conflict resolution
/// on collapsed tables, so a recreation that drops it loses which of two adjacent cell
/// borders wins - a difference no width or colour can express. It does share the half of
/// the rule that is true: CSS Backgrounds 3 4.3 zeroes the used width for `hidden` as
/// well, so the dependents still go.
#[test]
fn a_hidden_border_side_is_not_treated_as_an_absent_one() {
    let css = emitted(
        "td",
        &[
            ("border-right-style", "hidden"),
            ("border-right-width", "4px"),
            ("border-right-color", "rgb(1, 2, 3)"),
        ],
    );
    assert!(css.contains("border-right-style:hidden"), "{css}");
    assert!(!css.contains("border-right-width"), "{css}");
    assert!(!css.contains("border-right-color"), "{css}");
}

/// The rule stated without naming a property. Every value below is the initial value of
/// its own property and none of them has a companion in the map that makes it inert, so
/// all of them are the author overruling a user-agent rule and all must survive.
///
/// This is what a widened condition breaks: `outline-style:none` on a focused control and
/// `text-decoration-line:none` on a link are the same shape as the border reset, and a
/// fix that generalises by deleting more rather than deleting less loses them too.
#[test]
fn a_declaration_is_dropped_only_when_a_companion_makes_it_inert() {
    for (tag, name, value) in [
        ("button", "outline-style", "none"),
        ("a", "text-decoration-line", "none"),
        ("ul", "list-style-type", "none"),
        ("table", "border-spacing", "0px"),
        ("div", "background-image", "none"),
        ("h1", "margin-top", "0px"),
    ] {
        let css = emitted(tag, &[(name, value)]);
        assert!(css.contains(&format!("{name}:{value}")), "{tag}: {css}");
    }
}

/// The same loss had a second, silent victim inside the tool itself.
///
/// `responsive_geometry_scroll::remove_border` writes exactly this pair to take back a
/// border a narrower band no longer reserves a gutter for. It runs before this stage, so
/// deleting the style keyword by value deleted the whole instruction and left the band
/// saying nothing - which lets the base rule keep painting the border the band removed.
/// The function was dead code that no test covered and no reader could tell from live
/// code. It only does anything if the reset survives here.
#[test]
fn the_scroll_bands_own_border_removal_is_not_erased_before_emission() {
    let css = render(&[
        ("border-right-width", "0px"),
        ("border-right-style", "none"),
    ]);
    assert!(css.contains("border-right-style:none"), "{css}");
}
