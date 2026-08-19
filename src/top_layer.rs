//! The one place that decides why an element was in the top layer.
//!
//! Both node-record producers and the generated-box gate ask this question, and all three
//! need the same answer: a promotion the capture does not record cannot be replayed, and a
//! `::backdrop` box is generated for a top-layer element whatever put it there, so a gate
//! that asks a narrower question declines to look for a box the engine really generated.
//! Rendered into the one bundle every reader already renders, so the three cannot drift.

pub const SOURCE: &str = include_str!("top_layer.js");

/// The promotion a node record carries, and what the recreation does about it.
///
/// A string rather than a flag because the three ways into the top layer replay as three
/// different calls, and an empty string because that is what the engine's own answer reduces
/// to for an element the page never promoted.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Promotion(pub String);

impl Promotion {
    /// The calls that put an element back in the top layer, in the order a recreation has to
    /// make them.
    ///
    /// Ordered by dismissal, which is a fact about the platform rather than a preference:
    /// showing a dialog modally hides every auto popover in the document, while showing a
    /// popover hides no dialog. Replaying in document order therefore deletes whichever
    /// popover happens to precede a modal — and a popover the recreation opened and then
    /// closed again is indistinguishable from the one it never opened. Making the dismissing
    /// call first leaves it nothing to dismiss.
    ///
    /// Fullscreen is absent because it has no call, which is the same reason it carries no
    /// marker: `requestFullscreen()` needs transient user activation a recreation rendering
    /// itself does not have.
    pub const REPLAY: [(&'static str, &'static str); 2] =
        [("modal", "showModal"), ("popover", "showPopover")];

    /// Whether the page had this element in the top layer at all.
    pub fn promoted(&self) -> bool {
        !self.0.is_empty()
    }

    /// The serde companion to [`Promotion::promoted`], so an unpromoted element costs the
    /// record nothing.
    pub fn absent(&self) -> bool {
        !self.promoted()
    }

    /// The method the recreation calls to put the element back, if there is one.
    ///
    /// Fullscreen has none: `requestFullscreen()` needs transient user activation, which a
    /// recreation rendering itself does not have. It is still recorded, because a reader that
    /// cannot see the promotion cannot see that it was lost either.
    pub fn replay(&self) -> Option<&'static str> {
        Self::REPLAY
            .iter()
            .find(|(reason, _)| *reason == self.0)
            .map(|(_, call)| *call)
    }
}

#[cfg(test)]
#[path = "top_layer_tests.rs"]
mod tests;
