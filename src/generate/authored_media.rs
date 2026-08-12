use super::selector_scope::Scope;
use crate::model::Node;
use std::collections::{BTreeSet, HashSet};

/// The authored `@media` rules this node keeps, and the compounds their selectors name.
///
/// A compound is reported only when a rule survives deduplication, so a page with no
/// authored media rule reports none and gains no markers.
pub fn rules(
    node: &Node,
    scope: &Scope<'_>,
    rules: &[String],
    compounds: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut output = Vec::new();
    let mut seen = HashSet::new();
    for rule in rules {
        // `@layer` is a carrier, so a media rule authored inside one arrives still wrapped
        // in it. The layer is the rule's cascade position and is settled elsewhere; what
        // this stage is asking is whether a media condition is present, so it reads through
        // the wrapper exactly as `css::global_rule` does.
        let (_, rule) = super::css_layers::peel(rule);
        let Some((prefix, mut body, _)) = super::css_scan::block(rule) else {
            continue;
        };
        let prefix = prefix.trim();
        if !prefix.starts_with("@media") {
            continue;
        }
        let condition = prefix.trim_start_matches("@media").trim();
        while let Some((selectors, declarations, rest)) = super::css_scan::block(body) {
            body = rest;
            let Some(scoped) = super::selector_list::static_members(selectors)
                .find_map(|member| scope.rewrite(&member, node))
            else {
                continue;
            };
            let rule = format!(
                "@media {condition}{{{}{{{}}}}}",
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

#[cfg(test)]
#[path = "authored_media_tests.rs"]
mod tests;
