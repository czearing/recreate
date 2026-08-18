//! Which authored at-rule preludes name a condition the recreation must keep asking.
//!
//! One vocabulary, consulted by the re-emitter, by the chain walker, and by the index, so a
//! single definition decides what counts as a condition everywhere it is read.

/// The grouping at-rules whose condition the **document** answers, so no baked computed
/// style can stand in for them.
///
/// A capture bakes each element's computed style, which is the answer the condition had at
/// the instant of capture. That is the whole answer only for a condition with one answer for
/// the run — `@supports` asks the engine about its own feature support, and the artifact does
/// not reproduce the engine. These two are re-answered by whoever views the recreation: a
/// media condition by the viewport, and a container condition by the used inline-size of the
/// nearest ancestor with `container-type`, which layout re-produces on every resize and which
/// two instances of one component answer differently in the same paint. Baking either away
/// publishes the branch that happened to hold as though the author had written it
/// unconditionally.
///
/// `@layer` and `@scope` are carriers at capture but are not listed here: their preludes name
/// authored cascade positions and authored selectors, neither of which survives into the
/// generated document, so re-emitting them verbatim would reference names that do not exist.
/// `@starting-style` has its own owner in `before_change`.
const DOCUMENT_ANSWERED_AT_RULES: &[&str] = &["@media", "@container"];

/// Whether the **document** answers this at-rule's condition, so no baked computed style can
/// stand in for it.
pub(super) fn document_answered(prelude: &str) -> bool {
    DOCUMENT_ANSWERED_AT_RULES
        .iter()
        .any(|name| starts_with_at_rule(prelude, name))
}

/// Whether this condition has a false branch at all.
///
/// Withdrawal is owed only where the recreation can be asked the condition again and get a
/// different answer. `all` is the media type Media Queries 4 defines as matching every device,
/// so `@media all` is the identity condition: it is what a capture writes around a sheet linked
/// with `media="all"`, it holds at every width and in every container, and there is no arm
/// below any breakpoint for the unconditional cascade to restore. Withdrawing against it would
/// take a declaration out of the base rule to answer a question that is never asked.
pub(super) fn falsifiable(prelude: &str) -> bool {
    document_answered(prelude) && !identity_media(prelude)
}

fn identity_media(prelude: &str) -> bool {
    prelude
        .get(..6)
        .is_some_and(|name| name.eq_ignore_ascii_case("@media"))
        && prelude[6..].trim().eq_ignore_ascii_case("all")
}

/// Matched on the at-rule name and not on a bare prefix, so `@media-hypothetical` — any
/// future at-rule whose name merely begins with one of these — is not swept in.
fn starts_with_at_rule(prelude: &str, name: &str) -> bool {
    prelude.len() > name.len()
        && prelude.is_char_boundary(name.len())
        && prelude[..name.len()].eq_ignore_ascii_case(name)
        && !prelude[name.len()..]
            .starts_with(|character: char| character.is_alphanumeric() || character == '-')
}
