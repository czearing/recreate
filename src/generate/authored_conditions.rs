use super::selector_scope::Scope;
use crate::model::Node;
use std::collections::{BTreeSet, HashSet};

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

/// The authored condition rules this node keeps, and the compounds their selectors name.
///
/// A compound is reported only when a rule survives deduplication, so a page with no
/// authored condition rule reports none and gains no markers.
pub fn rules(
    node: &Node,
    scope: &Scope<'_>,
    rules: &[String],
    compounds: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut output = Vec::new();
    let mut seen = HashSet::new();
    for rule in rules {
        // `@layer` is a carrier, so a condition rule authored inside one arrives still
        // wrapped in it. The layer is the rule's cascade position and is settled elsewhere;
        // what this stage is asking is whether a document-answered condition is present, so
        // it reads through the wrapper exactly as `css::global_rule` does.
        let (_, rule) = super::css_layers::peel(rule);
        let Some((prefix, mut body, _)) = super::css_scan::block(rule) else {
            continue;
        };
        // The prelude travels verbatim rather than being taken apart and rebuilt, so a
        // container query's name and a `style()` query survive without this stage knowing
        // the grammar of either.
        let prelude = prefix.trim();
        if !DOCUMENT_ANSWERED_AT_RULES
            .iter()
            .any(|name| starts_with_at_rule(prelude, name))
        {
            continue;
        }
        while let Some((selectors, declarations, rest)) = super::css_scan::block(body) {
            body = rest;
            let Some(scoped) = super::selector_list::static_members(selectors)
                .find_map(|member| scope.rewrite(&member, node))
            else {
                continue;
            };
            let rule = format!(
                "{prelude}{{{}{{{}}}}}",
                scoped.selector,
                declarations.trim()
            );
            if seen.insert(rule.clone()) {
                output.push(rule);
                compounds.extend(scoped.compounds);
            }
        }
    }
    output
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

#[cfg(test)]
#[path = "authored_conditions_tests.rs"]
mod tests;
