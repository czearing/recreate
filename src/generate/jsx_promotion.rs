//! The single owner of a promotion the recreation has to replay rather than declare.
//!
//! Some of what a page did to an element is not a property of the element at all. Calling
//! `showModal()` puts a `<dialog>` in the **top layer**, a user-agent-managed list that
//! paints above every stacking context in the document and grows a `::backdrop`. No element
//! anywhere declares that membership: `show()`, `showModal()` and a hand-authored
//! `<dialog open>` all set the same `open` attribute, and top-layer membership is not a CSS
//! property, so the capture can only learn it by asking the engine and can only reproduce it
//! by making the same call.
//!
//! React has no declarative modality — `<dialog open>` is *defined* to be non-modal, and a
//! popover has no open attribute at all — so the emission is a marker the runtime finds after
//! render, not a prop. The marker names no tag and no reason: it carries the **call** the page
//! made, and the runtime makes that call on the element, so an element promoted by a route
//! with no replay excludes itself instead of being excluded by a list here.

use crate::model::Node;

/// The marker a promoted element carries so the recreation can put it back in the top layer.
pub(super) const PROMOTION: &str = "data-recreate-promotion";

/// The marker this element carries for a promotion the recreation must replay, if any.
pub(super) fn promotion(node: &Node) -> String {
    match node.promotion.replay() {
        Some(call) => format!(" {PROMOTION}=\"{call}\""),
        None => String::new(),
    }
}

/// Whether the replay makes `attribute` wrong to emit on this element.
///
/// `open` on a promoted element is not merely redundant with the replay, it defeats it: a
/// dialog React has already opened non-modally throws `InvalidStateError` when `showModal()`
/// is called on it, so the promotion would be lost and the element left exactly as the
/// defect left it.
pub(super) fn withholds(node: &Node, attribute: &str) -> bool {
    attribute == "open" && node.promotion.replay().is_some()
}
