//! The emitter never emits a measurement of a box it already reproduces.
//!
//! A captured style is a *used* value. `getComputedStyle` resolves `width: 50%`,
//! `flex: 1`, `clamp()` and `var()` to the pixels they happened to occupy at the captured
//! viewport, and it resolves an origin's percentages against the border box for the same
//! reason. Emitting such a value back out produces a page that is correct at exactly one
//! width and wrong everywhere else, and no per-tag guessing at "is this pixel real?" can
//! recover the authored intent, because the intent was destroyed before the emitter saw
//! the value.
//!
//! So both stages here ask one question — is this value still what the capture recorded?
//! `styles` starts as a clone of `node.style` and the stages above rewrite it, so a value
//! still equal to the captured one is untouched and is therefore the sample. Anything a
//! stage rewrote is emitter output and is left alone, however it happens to be spelled:
//! `preserve_space` reserves a thin scrollbar's gutter as a plain pixel width, and a
//! spelling test would delete it as if the capture had produced it.

use crate::model::{Node, Styles};

/// The size properties, in the two axes plus the shorthands that resolve to them.
const SIZE_PROPERTIES: [&str; 9] = [
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "flex-basis",
    "grid-template-columns",
    "grid-template-rows",
];

/// The origin properties: each names where a transformation is anchored on its box.
const ORIGIN_PROPERTIES: [&str; 2] = ["transform-origin", "perspective-origin"];

/// If the source authored a size, the authored value is used verbatim; otherwise nothing
/// is emitted and the box is sized by the same flow that sized it in the source.
///
/// Replaced elements are the one exception, and not a per-site one: they have no in-flow
/// content to reflow, and their box must be reserved or the page shifts as they load.
pub(in crate::generate) fn remove_sampled_sizes(
    styles: &mut Styles,
    node: &Node,
    css_rules: &crate::generate::authored_css::Index<'_>,
) {
    if is_replaced(node) {
        return;
    }
    for property in SIZE_PROPERTIES {
        if !is_sample(styles, &node.style, property) {
            continue;
        }
        match css_rules.authored_value(node, property) {
            Some(authored) => styles.insert(property.into(), authored),
            None => styles.remove(property),
        };
    }
}

/// The same properties, asked a different question: what may a later state withdraw?
///
/// Nothing, in either case. A size the source authored is re-emitted from the author's
/// own spelling, so it already says the right thing at every viewport and in every state,
/// and withdrawing it would replace an authored width with whatever the flow produces at
/// one edge of a band. A size the source did not author was never emitted as a
/// declaration at all, so there is nothing to withdraw. The same holds for the box a
/// replaced element reserves, which is a measurement the emitter keeps to stop the page
/// shifting rather than a statement the page made.
pub(in crate::generate) fn remove_resettable_sizes(styles: &mut Styles) {
    for property in SIZE_PROPERTIES {
        styles.remove(property);
    }
}

/// An authored `top left` and an element that declares no origin at all arrive spelled
/// identically, as pixels measured off the box, so nothing about the value distinguishes
/// them. The authored stage above replaces the sample with the author's own spelling
/// wherever the author wrote one, which leaves the sample test to answer it — no separate
/// lookup, and no exemption for replaced elements, whose intrinsic size is a statement
/// about their box and not about where a transformation is anchored on it.
///
/// This replaces asking whether anything is transformed. An origin is a parameter of a
/// transformation, not an effect of one, and the state that uses it need not be the state
/// that declares it: an anchor written once on a base rule aims a `:hover` transform that
/// the state record — a delta — never restates. Judged against the resting declarations
/// alone it looks inert in the only state where it is visible, and deleting it there
/// deletes it from the artifact, so the motion pivots from the box centre. Every route to
/// a deferred transform arrives at this same point, which is why no predicate over the
/// resting declarations can name them all.
///
/// On the pseudo-element path no authored stage runs, because the authored index holds no
/// pseudo-element rules. Every origin there is therefore a measurement, and `sample` is
/// the map itself so that each one tests as one.
pub(in crate::generate) fn remove_sampled_origins(styles: &mut Styles, sample: &Styles) {
    for property in ORIGIN_PROPERTIES {
        if is_sample(styles, sample, property) {
            styles.remove(property);
        }
    }
}

fn is_sample(styles: &Styles, sample: &Styles, property: &str) -> bool {
    styles.get(property).is_some() && styles.get(property) == sample.get(property)
}

/// Replaced elements are sized by their own intrinsic content rather than by the flow.
fn is_replaced(node: &Node) -> bool {
    matches!(
        node.tag.as_str(),
        "img" | "svg" | "video" | "canvas" | "iframe" | "embed" | "object"
    )
}
