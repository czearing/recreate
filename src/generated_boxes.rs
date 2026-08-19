/// The one place that decides which pseudo-elements a capture records and when each one
/// exists.
///
/// A handful of boxes the user agent generates on its own terms, and each of those
/// specifications states the condition — `::before` and `::after` on content, `::backdrop` on
/// top-layer membership, which no attribute and no computed style can express. Every other
/// pseudo-element exists because an author wrote a rule for one, and those are found by
/// reading the document's own selectors rather than by naming them here, because a list of
/// names reproduces exactly the pseudo-elements someone thought to write down.
///
/// Named here rather than at each reader so the probe that reverts these boxes, both node
/// record producers and every consumer answer from one list.
pub const SOURCE: &str = include_str!("generated_boxes.js");

#[cfg(test)]
#[path = "generated_boxes_tests.rs"]
mod discovery_tests;

#[cfg(test)]
mod tests {
    use super::SOURCE;

    /// The existence of a box is decided by the condition its own specification states, and
    /// the two conditions are genuinely different. Reading `content` for `::backdrop` would
    /// record a phantom scrim on every element, because a `::backdrop` never has content.
    /// The scrim's condition is top-layer membership, not inertness: the engine generates it
    /// for a popover too, whose default is transparent rather than absent.
    #[test]
    fn keys_the_backdrop_on_membership_and_the_content_boxes_on_content() {
        assert!(SOURCE.contains("'::backdrop': element => recreateTopLayer(element) !== ''"));
        assert!(!SOURCE.contains(":modal"));
        assert!(SOURCE.contains("'::before': (element, content) => content() !== ''"));
        assert!(SOURCE.contains("'::after': (element, content) => content() !== ''"));
    }

    /// `none` and `normal` are the two spellings of "this produced no content". A box that
    /// survives on either is one the engine generated for its own reasons, so redeclaring
    /// the value as authored content would assert something the page never said.
    #[test]
    fn treats_both_spellings_of_no_content_as_no_content() {
        assert!(SOURCE.contains("value !== 'none' && value !== 'normal'"));
    }
}
