/// The one place that decides which pseudo-elements a capture records and when each one
/// exists. A pseudo-element is not a property of the element that originates it: the user
/// agent either generates the box or does not, and each pseudo-element's own specification
/// states the condition — `::before` and `::after` on content, `::backdrop` on top-layer
/// membership, which no attribute and no computed style can express.
///
/// Named here rather than at each reader so the probe that reverts these boxes, both node
/// record producers and every consumer answer from one list. A further pseudo-element is
/// one entry.
pub const SOURCE: &str = include_str!("generated_boxes.js");

#[cfg(test)]
mod tests {
    use super::SOURCE;

    /// The existence of a box is decided by the condition its own specification states, and
    /// the two conditions are genuinely different. Reading `content` for `::backdrop` would
    /// record a phantom scrim on every element, because a `::backdrop` never has content.
    #[test]
    fn keys_the_backdrop_on_membership_and_the_content_boxes_on_content() {
        assert!(SOURCE.contains("'::backdrop': element => element.matches(':modal')"));
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
