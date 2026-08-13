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
//! React has no declarative modality — `<dialog open>` is *defined* to be non-modal — so the
//! emission is a marker the runtime finds after render, not a prop. The marker names no tag:
//! the record says the engine had this element in the top layer, and the runtime asks the
//! element itself whether it knows how to re-enter, so an element promoted by some other
//! route excludes itself instead of being excluded by a list here.

use crate::model::Node;

/// The marker a promoted element carries so the recreation can put it back in the top layer.
pub(super) const PROMOTION: &str = "data-recreate-modal";

/// The marker this element carries for a promotion the recreation must replay, if any.
pub(super) fn promotion(node: &Node) -> String {
    if node.modal {
        return format!(" {PROMOTION}={{true}}");
    }
    String::new()
}

/// Whether the replay makes `attribute` wrong to emit on this element.
///
/// `open` on a promoted element is not merely redundant with the replay, it defeats it: a
/// dialog React has already opened non-modally throws `InvalidStateError` when `showModal()`
/// is called on it, so the promotion would be lost and the element left exactly as the
/// defect left it.
pub(super) fn withholds(node: &Node, attribute: &str) -> bool {
    attribute == "open" && node.modal
}
