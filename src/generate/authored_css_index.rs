use crate::model::{Node, Styles};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use super::authored_css_rules::{
    compound_attributes, compound_classes, compound_id, compound_tag, directly_targets_node,
    terminal_compound,
};

pub struct Index<'a> {
    rules: Vec<Rule<'a>>,
    direct_by_class: HashMap<String, Vec<usize>>,
    direct_by_id: HashMap<String, Vec<usize>>,
    direct_by_tag: HashMap<String, Vec<usize>>,
    direct_by_attribute: HashMap<String, Vec<usize>>,
    direct_universal: Vec<usize>,
}

struct Rule<'a> {
    selectors: Cow<'a, str>,
    declarations: &'a str,
    position: super::css_layers::Position,
}

/// `:where()` and `:is()` say nothing about state: they match on structure alone, so a
/// rule using one applies in the base state exactly like the compound it wraps. Dropping
/// them loses real declarations — Fluent defines `--component-card-padding` only on
/// `.root:where(.size-medium)`, and without it every card that sizes its padding from
/// that variable collapses to zero padding.
///
/// State pseudo-classes such as `:hover` and pseudo-elements such as `::before` describe
/// a different state and must still be excluded, so this returns `None` whenever a colon
/// survives flattening.
fn flatten_static_pseudo_classes(selectors: &str) -> Option<Cow<'_, str>> {
    if !selectors.contains(':') {
        return Some(Cow::Borrowed(selectors));
    }
    let mut flattened = selectors.to_string();
    for name in [":where(", ":is("] {
        while let Some(start) = flattened.find(name) {
            let open = start + name.len();
            let mut depth = 1usize;
            let mut end = None;
            for (offset, character) in flattened[open..].char_indices() {
                match character {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(open + offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let end = end?;
            let inner = flattened[open..end].trim().to_string();
            // A selector list inside the wrapper would have to expand into several
            // selectors to stay correct, so it is left for the caller to skip.
            if inner.contains(',') {
                return None;
            }
            flattened.replace_range(start..=end, &inner);
        }
    }
    (!flattened.contains(':')).then_some(Cow::Owned(flattened))
}

/// A value made only of absolute pixel lengths. It resolves to itself, so comparing it
/// against the captured computed value is exact — unlike `1fr`, `auto`, or a percentage,
/// which resolve against the layout and legitimately differ from the sample.
fn absolute_length(value: &str) -> bool {
    !value.trim().is_empty()
        && value.split_whitespace().all(|token| {
            token == "0"
                || token
                    .strip_suffix("px")
                    .is_some_and(|number| number.parse::<f32>().is_ok())
        })
}

impl<'a> Index<'a> {
    pub fn new(rules: &'a [String]) -> Self {
        let mut index = Self {
            rules: Vec::new(),
            direct_by_class: HashMap::new(),
            direct_by_id: HashMap::new(),
            direct_by_tag: HashMap::new(),
            direct_by_attribute: HashMap::new(),
            direct_universal: Vec::new(),
        };
        let order = super::css_layers::Order::new(rules);
        for rule in rules {
            // A layer is cascade position, not a condition, so its wrapper is peeled and
            // remembered rather than skipped with the conditional groups below.
            let (layer, rule) = super::css_layers::peel(rule);
            let Some((selectors, declarations)) = rule.split_once('{') else {
                continue;
            };
            if selectors.starts_with('@') {
                continue;
            }
            let Some(selectors) = flatten_static_pseudo_classes(selectors) else {
                continue;
            };
            let rule_index = index.rules.len();
            let owned = selectors.clone();
            index.rules.push(Rule {
                selectors,
                declarations,
                position: order.position(layer.as_deref()),
            });
            for selector in owned.split(',').map(str::trim) {
                if terminal_compound(selector) != selector {
                    continue;
                }
                for class in compound_classes(selector) {
                    index
                        .direct_by_class
                        .entry(class)
                        .or_default()
                        .push(rule_index);
                }
                if let Some(id) = compound_id(selector) {
                    index
                        .direct_by_id
                        .entry(id.to_string())
                        .or_default()
                        .push(rule_index);
                }
                for (name, _) in compound_attributes(selector) {
                    index
                        .direct_by_attribute
                        .entry(name.to_string())
                        .or_default()
                        .push(rule_index);
                }
                let tag = compound_tag(selector);
                if tag == "*" {
                    index.direct_universal.push(rule_index);
                } else if !tag.is_empty() {
                    index
                        .direct_by_tag
                        .entry(tag.to_string())
                        .or_default()
                        .push(rule_index);
                }
            }
        }
        index
    }

    pub fn declarations(&self, node: &Node) -> Styles {
        let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut references: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for index in self.direct_indices(node) {
            let rule = &self.rules[index];
            for (name, value) in parsed_declarations(rule.declarations)
                .map(|(name, value)| {
                    let physical = super::authored_css_rules::physical_property(node, name);
                    (if physical.is_empty() { name } else { physical }, value)
                })
                .filter(|(name, value)| {
                    super::authored_css_rules::retained(name)
                        && !super::authored_css_rules::cascade_keyword(value)
                })
            {
                if value.contains("var(") && !super::authored_css_rules::fluid_authored_value(value)
                {
                    references
                        .entry(name.into())
                        .or_default()
                        .push(value.into());
                    continue;
                }
                values.entry(name.into()).or_default().push(value.into());
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

    /// The last authored value for a property, with none of the filtering `declarations`
    /// applies. A size written as `var(--card-width)` or `clamp(...)` is dropped there
    /// because it cannot be compared against the sampled value, and treating it as absent
    /// leaves the sample in its place — which pins the box to the captured viewport. The
    /// authored text is what the source actually says, so it is what gets emitted.
    ///
    /// Names are resolved to their physical equivalent first. A source that writes
    /// `max-inline-size` is authoring `max-width`, and a literal name comparison reports
    /// it as unauthored — which deletes the declaration instead of keeping it.
    ///
    /// CSS-wide keywords are skipped. They declare no value, so emitting one only
    /// clobbers a correct value the generator wrote in a lower-precedence rule.
    ///
    /// Several rules may declare the same property, and this index models neither
    /// `@layer` order nor specificity, so the textually last declaration is not
    /// reliably the cascade winner. The captured computed value settles it: a
    /// candidate that is a concrete literal disagreeing with the sample demonstrably
    /// lost, and emitting it replaces correct geometry with a losing declaration.
    /// Fluent gives a card `padding: var(--component-card-padding)` while the page
    /// also authors `.card { padding: 0px }`; the card computes to 12px, so the
    /// literal lost and only the custom-property reference may be emitted.
    pub fn authored_value(&self, node: &Node, property: &str) -> Option<String> {
        let candidates = self
            .direct_indices(node)
            .into_iter()
            .flat_map(|index| parsed_declarations(self.rules[index].declarations))
            .filter(|(name, _)| {
                let physical = super::authored_css_rules::physical_property(node, name);
                (if physical.is_empty() { *name } else { physical }) == property
            })
            .map(|(_, value)| value.trim().trim_end_matches('}').trim().to_string())
            .filter(|value| !value.is_empty() && !super::authored_css_rules::cascade_keyword(value))
            .collect::<Vec<_>>();
        let Some(sampled) = node.style.get(property) else {
            return candidates.into_iter().next_back();
        };
        if let Some(agreeing) = candidates.iter().rev().find(|value| *value == sampled) {
            return Some(agreeing.clone());
        }
        let last = candidates.last();
        if last.is_some_and(|value| absolute_length(value)) {
            return Some(sampled.clone());
        }
        last.cloned()
    }

    pub fn positive_integer_property(&self, node: &Node, property: &str) -> Option<u32> {
        self.direct_indices(node)
            .into_iter()
            .flat_map(|index| parsed_declarations(self.rules[index].declarations))
            .filter(|(name, _)| *name == property)
            .map(|(_, value)| {
                value
                    .trim_end_matches('}')
                    .trim()
                    .trim_end_matches("!important")
                    .trim()
            })
            .next_back()?
            .parse()
            .ok()
            .filter(|value| *value > 0)
    }

    /// Restores the value a stylesheet actually authored for `property`, in place of the
    /// computed value the browser baked. Selector shape decides which elements a rule reaches,
    /// never whether a matched declaration is real, so this uses the same matcher as every
    /// other direct lookup rather than a class-keyed one of its own.
    pub fn inherited_value(&self, node: &Node, property: &str) -> Option<String> {
        let values = self
            .direct_indices(node)
            .into_iter()
            .map(|index| &self.rules[index])
            .flat_map(|rule| parsed_declarations(rule.declarations))
            .filter(|(name, value)| *name == property && !value.contains("var("))
            .map(|(_, value)| value.to_string())
            .collect::<Vec<_>>();
        values.first().and_then(|first| {
            values
                .iter()
                .all(|value| value == first)
                .then(|| first.clone())
        })
    }

    fn direct_indices(&self, node: &Node) -> Vec<usize> {
        let mut candidates = self.direct_universal.clone();
        for class in node_classes(node) {
            if let Some(indices) = self.direct_by_class.get(class) {
                candidates.extend(indices.iter().copied());
            }
        }
        if let Some(id) = node.attributes.get("id")
            && let Some(indices) = self.direct_by_id.get(id)
        {
            candidates.extend(indices.iter().copied());
        }
        if let Some(indices) = self.direct_by_tag.get(&node.tag) {
            candidates.extend(indices.iter().copied());
        }
        for attribute in node.attributes.keys() {
            if let Some(indices) = self.direct_by_attribute.get(attribute) {
                candidates.extend(indices.iter().copied());
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        candidates.retain(|index| directly_targets_node(&self.rules[*index].selectors, node));
        // Every caller reads these as "the later candidate won", which is only true once
        // they are in cascade order rather than sheet order: a layered declaration loses
        // to an unlayered one whatever their specificity, and to a later layer whatever
        // their position in the sheet. Sorting is stable, so sheet order still breaks ties
        // within one layer, which is where it is the right answer.
        candidates.sort_by_key(|index| self.rules[*index].position.clone());
        candidates
    }
}

fn parsed_declarations(declarations: &str) -> impl DoubleEndedIterator<Item = (&str, &str)> {
    declarations
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .map(|(name, value)| (name.trim(), value.trim()))
}

fn node_classes(node: &Node) -> impl Iterator<Item = &str> {
    node.attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
}

#[cfg(test)]
mod tests {
    use super::Index;
    use crate::model::{Node, Rect, Styles};

    fn node() -> Node {
        let mut node = Node {
            path: "button".into(),
            parent: None,
            tag: "button".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            style: Styles::from([("display".into(), "flex".into())]),
            before: None,
            after: None,
        };
        node.attributes
            .insert("class".into(), "primary control".into());
        node.attributes.insert("data-kind".into(), "action".into());
        node
    }

    #[test]
    fn indexes_class_tag_and_attribute_selectors_without_changing_order() {
        let rules = vec![
            ".primary{display:block;width:40px;}".into(),
            "button[data-kind=\"action\"]{display:flex;width:50px;}".into(),
        ];
        let styles = Index::new(&rules).declarations(&node());
        assert_eq!(styles["display"], "flex");
        assert_eq!(styles["width"], "50px");
    }

    #[test]
    fn indexes_universal_selectors_for_every_node() {
        let rules = vec!["*{width:40px;}".into()];
        let index = Index::new(&rules);
        assert_eq!(index.direct_universal, vec![0]);
        assert_eq!(index.direct_indices(&node()), vec![0]);
        assert_eq!(index.declarations(&node())["width"], "40px");
    }
}
