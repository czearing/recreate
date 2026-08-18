//! The document-answered conditional rules, kept apart from the unconditional cascade.
//!
//! Both are the same table of rules matched by the same matcher, so a conditional rule is out
//! of reach for a node exactly when its unconditional twin is. What differs is when they are
//! built: most readers of the index never ask about a condition, so flattening the groups is
//! deferred until one does.

use super::authored_css_table::Table;
use crate::model::Node;
use std::sync::OnceLock;

#[derive(Default)]
pub(super) struct Conditional<'a> {
    /// The `@media` and `@container` groups, kept whole. Flattening them is a second pass over
    /// every rule on the page, and it is wasted for every reader that only wants the
    /// unconditional cascade.
    groups: Vec<(&'a str, super::css_layers::Position)>,
    table: OnceLock<Table<'a>>,
}

impl<'a> Conditional<'a> {
    /// Records a grouping at-rule whose condition the document answers. A prelude the
    /// recreation does not re-emit is not recorded, so nothing downstream can withdraw a
    /// branch that nothing puts back.
    pub(super) fn add(&mut self, rule: &'a str, position: super::css_layers::Position) {
        if super::authored_conditions::document_answered(
            rule.split('{').next().unwrap_or("").trim(),
        ) {
            self.groups.push((rule, position));
        }
    }

    /// The declarations every recorded condition rule that directly targets this node states,
    /// in cascade order.
    pub(super) fn declarations(&self, node: &Node) -> Vec<&'a str> {
        if self.groups.is_empty() {
            return Vec::new();
        }
        let table = self.table.get_or_init(|| self.flattened());
        table
            .matching(node)
            .iter()
            .map(|index| table.rules[*index].declarations)
            .collect()
    }

    /// Keeping the groups whole and re-scanning every one for every node is quadratic; folding
    /// them into the table every reader consults makes every unconditional lookup wade through
    /// them. A second table, built at most once, is neither.
    ///
    /// Only a rule some layer of whose condition chain can be false is recorded. A rule guarded
    /// by nothing but the identity condition applies everywhere the base rule does, so there is
    /// no other arm to restore and withdrawing against it would delete the only one.
    fn flattened(&self) -> Table<'a> {
        let mut table = Table::default();
        for (rule, position) in &self.groups {
            super::authored_condition_chain::for_each_rule(rule, &mut |conditions,
                                                                  selectors,
                                                                  declarations| {
                if conditions.falsifiable() {
                    table.add(selectors, declarations, position.clone());
                }
            });
        }
        table
    }
}
