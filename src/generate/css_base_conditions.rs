//! Emitting the authored conditional rules a page's elements were matched by.

use crate::model::Specification;
use std::collections::{BTreeMap, BTreeSet};

/// Rules sharing a condition chain and a declaration block are emitted once, on one selector
/// list. A sheet linked `media="all"` wraps every rule it holds in the identity condition, so
/// without the merge a page's resets arrive once per element that carries them.
pub(super) fn append_authored_conditions(
    specification: &Specification,
    prefix: &str,
    classes: &BTreeMap<String, String>,
    authored_rules: &super::authored_css::Index<'_>,
    css: &mut String,
) -> BTreeSet<String> {
    let base = &specification.states[0];
    let measured = super::authored_conditions_measured::Measured::new(&specification.states);
    let mut groups = super::css_rule_groups::Groups::default();
    let mut compounds = BTreeSet::new();
    let scope = super::selector_scope::Scope::new(&base.nodes, classes, prefix);
    for node in &base.nodes {
        if classes.get(&node.path).is_none() {
            continue;
        };
        for rule in super::authored_conditions::rules(
            node,
            &scope,
            &base.css_rules,
            authored_rules,
            &measured,
            &mut compounds,
        ) {
            groups.add(
                (String::new(), Some(rule.opening), rule.declarations),
                rule.selector,
            );
        }
    }
    for ((_, opening, declarations), selectors) in groups {
        let opening = opening.unwrap_or_default();
        css.push_str(
            &super::authored_conditions::Emitted {
                selector: selectors.into_iter().collect::<Vec<_>>().join(","),
                opening,
                declarations,
            }
            .text(),
        );
        css.push('\n');
    }
    compounds
}
