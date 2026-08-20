//! The rule table every authored lookup walks.
//!
//! One shape serves the unconditional cascade and the document-answered conditional rules
//! alike, so both are built and matched by one piece of code and a conditional rule is out of
//! reach exactly when its unconditional twin is.

use crate::model::Node;
use std::borrow::Cow;
use std::collections::HashMap;

use super::compound::{
    Subject, compound_attributes, compound_classes, compound_id, compound_tag, terminal_compound,
};
/// Rules, plus their positions keyed by every part of a subject compound a node can be asked
/// about. One shape serves the unconditional cascade and the conditional rules alike, so both
/// are built and matched by one piece of code.
#[derive(Default)]
pub(super) struct Table<'a> {
    pub(super) rules: Vec<Rule<'a>>,
    by_class: HashMap<String, Vec<usize>>,
    by_id: HashMap<String, Vec<usize>>,
    by_tag: HashMap<String, Vec<usize>>,
    by_attribute: HashMap<String, Vec<usize>>,
    pub(super) universal: Vec<usize>,
}

impl<'a> Table<'a> {
    pub(super) fn add(
        &mut self,
        selectors: &'a str,
        declarations: &'a str,
        position: super::css_layers::Position,
    ) {
        let statics = super::selector_list::static_members(selectors).collect::<Vec<_>>();
        if statics.is_empty() {
            return;
        }
        // A list the static filter neither narrowed nor rewrote is the authored text itself,
        // so it is borrowed rather than rebuilt. A page carrying five figures of rules pays
        // for every allocation here once per state.
        let joined = match statics.as_slice() {
            [Cow::Borrowed(only)] if *only == selectors.trim() => Cow::Borrowed(*only),
            _ => Cow::Owned(
                statics
                    .iter()
                    .map(Cow::as_ref)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        };
        let rule_index = self.rules.len();
        self.rules.push(Rule {
            selectors: joined,
            declarations,
            position,
        });
        for selector in statics.iter().map(Cow::as_ref) {
            self.index(selector, rule_index);
        }
    }

    fn index(&mut self, selector: &str, rule_index: usize) {
        if terminal_compound(selector) != selector {
            return;
        }
        for class in compound_classes(selector) {
            self.by_class.entry(class).or_default().push(rule_index);
        }
        if let Some(id) = compound_id(selector) {
            self.by_id
                .entry(id.to_string())
                .or_default()
                .push(rule_index);
        }
        for (name, _) in compound_attributes(selector) {
            self.by_attribute
                .entry(name.to_string())
                .or_default()
                .push(rule_index);
        }
        let tag = compound_tag(selector);
        if tag == "*" {
            self.universal.push(rule_index);
        } else if !tag.is_empty() {
            self.by_tag
                .entry(tag.to_string())
                .or_default()
                .push(rule_index);
        }
    }

    /// Every rule whose subject compound this node satisfies. One matcher answers for both
    /// tables, so a conditional rule is out of reach exactly when its unconditional twin is,
    /// by construction rather than by a guard.
    pub(super) fn matching(&self, node: &Node) -> Vec<usize> {
        let mut candidates = self.universal.clone();
        for class in node_classes(node) {
            if let Some(indices) = self.by_class.get(class) {
                candidates.extend(indices.iter().copied());
            }
        }
        if let Some(id) = node.attributes.get("id")
            && let Some(indices) = self.by_id.get(id)
        {
            candidates.extend(indices.iter().copied());
        }
        if let Some(indices) = self.by_tag.get(&node.tag) {
            candidates.extend(indices.iter().copied());
        }
        for attribute in node.attributes.keys() {
            if let Some(indices) = self.by_attribute.get(attribute) {
                candidates.extend(indices.iter().copied());
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        let subject = Subject::new(node);
        candidates.retain(|index| subject.directly_targeted_by(&self.rules[*index].selectors));
        // Every caller reads these as "the later candidate won", which is only true once
        // they are in cascade order rather than sheet order: a layered declaration loses
        // to an unlayered one whatever their specificity, and to a later layer whatever
        // their position in the sheet. Sorting is stable, so sheet order still breaks ties
        // within one layer, which is where it is the right answer.
        candidates.sort_by_key(|index| self.rules[*index].position.clone());
        candidates
    }

    /// Every declaration every rule this node matches states, in cascade order. The one way
    /// any single-property reader reaches the rules, so none of them can match differently.
    pub(super) fn declarations_of<'n>(
        &'n self,
        node: &Node,
    ) -> impl DoubleEndedIterator<Item = (&'n str, &'n str)> {
        self.blocks(node)
            .into_iter()
            .flat_map(super::css_declaration::parsed)
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// The same walk, keeping each declaration's own block. A recorded shorthand division is
    /// keyed by block text, so a reader that needs one cannot work from declarations alone.
    pub(super) fn blocks<'n>(&'n self, node: &Node) -> Vec<&'n str> {
        self.matching(node)
            .into_iter()
            .map(|index| self.rules[index].declarations)
            .collect()
    }

    /// This table's last word on each of `properties` for `node`, taking each declaration at
    /// face value: no agreement test against the sample, and a property no rule here declares
    /// simply absent from the result.
    ///
    /// A property this table declares only through a value it cannot divide is present and
    /// `None`, which is a different answer from absent: the author did write an arm, so
    /// dropping the property would publish an initial value the source never takes.
    ///
    /// A CSS-wide keyword declares no value, so it is not a word this can report. Reporting it
    /// would publish `revert` or `inherit` as though it were the author's value, which the
    /// recreation's own cascade resolves against a different origin than the source's did.
    pub(super) fn declared_values(
        &self,
        shorthands: &super::shorthand::Shorthands,
        node: &Node,
        properties: &std::collections::BTreeSet<String>,
    ) -> std::collections::BTreeMap<String, Option<String>> {
        let mut resolved = std::collections::BTreeMap::new();
        for index in self.matching(node) {
            let block = self.rules[index].declarations;
            for (name, value) in super::css_declaration::parsed(block) {
                let value = value.trim().trim_end_matches('}').trim();
                if value.is_empty() || super::authored_css_rules::cascade_keyword(value) {
                    continue;
                }
                let physical = super::authored_css_rules::physical_property(node, name);
                for property in properties {
                    if physical.answers(name, property) {
                        resolved.insert(property.clone(), Some(value.to_string()));
                        continue;
                    }
                    match super::shorthand::claim(shorthands, block, name, value, property) {
                        super::shorthand::Claim::Value(share) => {
                            resolved.insert(property.clone(), Some(share.to_string()));
                        }
                        super::shorthand::Claim::Unsettled => {
                            resolved.insert(property.clone(), None);
                        }
                        super::shorthand::Claim::Elsewhere => (),
                    }
                }
            }
        }
        resolved
    }
}

pub(super) struct Rule<'a> {
    selectors: Cow<'a, str>,
    pub(super) declarations: &'a str,
    position: super::css_layers::Position,
}

fn node_classes(node: &Node) -> impl Iterator<Item = &str> {
    node.attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
}
