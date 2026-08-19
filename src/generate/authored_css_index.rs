use crate::model::{Node, Styles};
use std::collections::{BTreeMap, BTreeSet};

use super::authored_css_table::Table;
use super::authored_css_value::absolute_length;
use super::shorthand::Shorthands;

/// Everything one capture recorded about the page's authored stylesheets: the rule texts, and
/// how the engine divided each block that spells a shorthand.
///
/// The two travel together because the division is keyed by the block text — indexing one
/// without the other is what leaves a shorthand-authored declaration unreadable. A caller
/// holding a captured state hands the state over and gets both; one holding bare rule text
/// converts in and gets no divisions, which is the answer for text nothing ever parsed.
#[derive(Clone, Copy)]
pub struct Authored<'a> {
    pub rules: &'a [String],
    pub shorthands: &'a Shorthands,
}

fn no_shorthands() -> &'static Shorthands {
    static EMPTY: std::sync::OnceLock<Shorthands> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Shorthands::new)
}

impl<'a> From<&'a [String]> for Authored<'a> {
    fn from(rules: &'a [String]) -> Self {
        Self {
            rules,
            shorthands: no_shorthands(),
        }
    }
}

impl<'a> From<&'a Vec<String>> for Authored<'a> {
    fn from(rules: &'a Vec<String>) -> Self {
        rules.as_slice().into()
    }
}

impl<'a, const N: usize> From<&'a [String; N]> for Authored<'a> {
    fn from(rules: &'a [String; N]) -> Self {
        rules.as_slice().into()
    }
}

impl<'a> From<&'a crate::model::PageState> for Authored<'a> {
    fn from(state: &'a crate::model::PageState) -> Self {
        Self {
            rules: &state.css_rules,
            shorthands: &state.css_shorthands,
        }
    }
}

pub struct Index<'a> {
    table: Table<'a>,
    shorthands: &'a Shorthands,
}

impl<'a> Index<'a> {
    pub fn new(authored: impl Into<Authored<'a>>) -> Self {
        let Authored { rules, shorthands } = authored.into();
        let mut index = Self {
            table: Table::default(),
            shorthands,
        };
        let order = super::css_layers::Order::new(rules);
        for rule in rules {
            // A layer is cascade position, not a condition, so its wrapper is peeled and
            // remembered rather than skipped with the grouping at-rules below. A grouping
            // rule's declarations are not the unconditional cascade's, and which of them a
            // condition decided is the engine's answer rather than this table's.
            let (layer, rule) = super::css_layers::peel(rule);
            let position = order.position(layer.as_deref());
            let Some((selectors, declarations)) = rule.split_once('{') else {
                continue;
            };
            if selectors.starts_with('@') {
                continue;
            }
            index.table.add(selectors, declarations, position);
        }
        index
    }

    /// The declarations of every rule this node matches, in cascade order.
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
    /// A property declared only through a value this stage cannot divide between longhands is
    /// present and `None`, so a caller can tell "the author wrote no arm" from "the author
    /// wrote one this cannot read" and refuse to delete on the second.
    ///
    /// That test exists to abstain where a literal demonstrably lost to a rule this index
    /// cannot order. Its only caller here has already identified the winner — a declaration
    /// inside a document-answered condition, which the engine itself reported as deciding
    /// this property — so the disagreement is explained rather than unexplained, and
    /// abstaining would restore the very value the caller is withdrawing.
    pub(super) fn unconditional_values(
        &self,
        node: &Node,
        properties: &BTreeSet<String>,
    ) -> BTreeMap<String, Option<String>> {
        self.table
            .declared_values(self.shorthands, node, properties)
    }

    /// How the engine divided each authored block, for the stages that must read an authored
    /// declaration against a sampled longhand.
    pub(super) fn shorthands(&self) -> &'a Shorthands {
        self.shorthands
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
