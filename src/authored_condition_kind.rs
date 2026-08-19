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

/// The same two questions in JavaScript, generated from the list above rather than written a
/// second time, so the conditions a capture withdraws and the conditions the emitter
/// re-publishes cannot drift into being different sets.
///
/// It answers for a carrier stack rather than for one prelude, because that is the unit both
/// sides key on: the chain spelled as the text that opens it, which is what
/// [`crate::generate`]'s re-emitter puts back around a rewritten rule.
pub fn js_source() -> String {
    let names = DOCUMENT_ANSWERED_AT_RULES
        .iter()
        .map(|name| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"
  const documentAnsweredAtRules = [{names}];
  const documentAnswered = prelude => documentAnsweredAtRules.some(name =>
    prelude.length > name.length
      && prelude.slice(0, name.length).toLowerCase() === name
      && !/[a-z0-9-]/i.test(prelude[name.length]));
  const identityMedia = prelude =>
    prelude.slice(0, 6).toLowerCase() === '@media' && prelude.slice(6).trim().toLowerCase() === 'all';
  // The chain of document-answered conditions a rule sits under, spelled as the text that
  // opens it — the key the re-emitter groups its rewritten rules by. Empty when no layer of
  // the chain has a false branch at all, which is the whole of what withdrawal is owed for.
  const conditionOpening = carriers => {{
    const chain = carriers.map(prelude => prelude.trim()).filter(documentAnswered);
    return chain.length && chain.some(prelude => !identityMedia(prelude)) ? chain.join('{{') : '';
  }};
"#
    )
}

/// Whether the **document** answers this at-rule's condition, so no baked computed style can
/// stand in for it.
pub(crate) fn document_answered(prelude: &str) -> bool {
    DOCUMENT_ANSWERED_AT_RULES
        .iter()
        .any(|name| starts_with_at_rule(prelude, name))
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
