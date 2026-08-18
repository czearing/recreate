use crate::model::{Node, Styles};
use std::collections::{BTreeMap, BTreeSet};

use super::authored_css_conditional::Conditional;
use super::authored_css_table::Table;
use super::authored_css_value::absolute_length;

pub struct Index<'a> {
    table: Table<'a>,
    conditional: Conditional<'a>,
}

impl<'a> Index<'a> {
    pub fn new(rules: &'a [String]) -> Self {
        let mut index = Self {
            table: Table::default(),
            conditional: Conditional::default(),
        };
        let order = super::css_layers::Order::new(rules);
        for rule in rules {
            // A layer is cascade position, not a condition, so its wrapper is peeled and
            // remembered rather than skipped with the conditional groups below.
            let (layer, rule) = super::css_layers::peel(rule);
            let position = order.position(layer.as_deref());
            let Some((selectors, declarations)) = rule.split_once('{') else {
                continue;
            };
            if selectors.starts_with('@') {
                index.conditional.add(rule, position);
                continue;
            }
            index.table.add(selectors, declarations, position);
        }
        index
    }

    /// The declarations every document-answered condition rule that directly targets this
    /// node states, in cascade order.
    pub(super) fn conditional_declarations(&self, node: &Node) -> Vec<&'a str> {
        self.conditional.declarations(node)
    }

    pub fn declarations(&self, node: &Node) -> Styles {
        let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for index in self.direct_indices(node) {
            let rule = &self.table.rules[index];
            for (name, value) in parsed_declarations(rule.declarations)
                .flat_map(|(name, value)| {
                    super::authored_css_rules::physical_property(node, name)
                        .into_declarations(name, value)
                })
                .filter(|(name, value)| {
                    super::authored_css_rules::retained(name)
                        && !super::authored_css_rules::cascade_keyword(value)
                })
            {
                if super::authored_css_rules::deferred_binding(&value)
                    && !super::fluid_value::fluid(&value)
                {
                    continue;
                }
                values.entry(name).or_default().push(value);
            }
        }
        values
            .into_iter()
            .filter_map(|(name, values)| {
                let picked = values.iter().rev().find(|value| {
                    super::authored_css_rules::resolved_matches(node, &name, value)
                })?;
                // A pixel literal that disagrees with the captured computed value lost
                // the cascade to a rule this index cannot order, such as one in a later
                // `@layer`. The author did declare the property, so it is authored — but
                // the value that won is the one the capture measured, not this literal.
                if let Some(computed) = node
                    .style
                    .get(&name)
                    .filter(|computed| *computed != picked && absolute_length(picked))
                {
                    return Some((name, computed.clone()));
                }
                Some((name, picked.clone()))
            })
            .collect()
    }

    pub fn has_property(&self, node: &Node, property: &str) -> bool {
        self.authored_value(node, property).is_some()
    }

    pub fn authored_value(&self, node: &Node, property: &str) -> Option<String> {
        super::authored_css_value::authored(&self.table, node, property)
    }

    /// The unconditional cascade's last word on each of `properties`, with the agreement
    /// test in `authored_value` deliberately absent, and with a property the author never
    /// declared unconditionally simply absent from the result.
    ///
    /// That test exists to abstain where a literal demonstrably lost to a rule this index
    /// cannot order. Its only caller here has already identified the winner — a declaration
    /// inside a document-answered condition, whose value it matched against the sample — so
    /// the disagreement is explained rather than unexplained, and abstaining would restore
    /// the very value the caller is withdrawing.
    pub(super) fn unconditional_values(
        &self,
        node: &Node,
        properties: &BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        self.table.declared_values(node, properties)
    }

    pub fn positive_integer_property(&self, node: &Node, property: &str) -> Option<u32> {
        super::authored_css_value::positive_integer(&self.table, node, property)
    }

    pub fn inherited_value(&self, node: &Node, property: &str) -> Option<String> {
        super::authored_css_value::inherited(&self.table, node, property)
    }
    fn direct_indices(&self, node: &Node) -> Vec<usize> {
        self.table.matching(node)
    }
}

fn parsed_declarations(declarations: &str) -> impl DoubleEndedIterator<Item = (&str, &str)> {
    super::css_declaration::parsed(declarations)
}

#[cfg(test)]
#[path = "authored_css_index_tests.rs"]
mod tests;
