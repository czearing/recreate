//! The values a fragment inherited, re-declared on the fragment itself, for the stage that
//! is about to move it into a document of its own.
//!
//! The capture records a declaration only where it differs from what the element would
//! compute with no author CSS, and reverting an element leaves its ancestors live, so an
//! inherited value is recorded on the ancestor that changed it and nowhere below. That
//! pruning is sound because the recreation keeps the same ancestors and the engine
//! recomputes the value for free — a premise that holds for the page and fails for exactly
//! one thing the generator does, which is relocate a subtree into its own file. An image
//! document inherits nothing from its host, so `currentcolor` there resolves against
//! `color`'s initial value and the graphic paints in `canvastext`.
//!
//! `css_inheritance` cannot supply this. It moves a declaration only from `:root` or
//! `html`, deliberately, because a name overridden on an intermediate ancestor is
//! indistinguishable in text alone — and an inherited paint is normally declared on an
//! intermediate ancestor. The value is not in the stylesheet's text at all; it is in the
//! captured tree, where the engine already answered it. So the answer is read from the
//! tree and handed to the relocation stage as ordinary rules, which the closure then
//! selects by class like any other.
//!
//! Nothing here reaches the page's own stylesheet. A rule naming a class no fragment
//! carries is never selected, so a page with no relocated subtree emits what it always did.

use crate::model::{Node, PageState};
use std::collections::{BTreeMap, HashMap};

/// Rules binding, for each class whose declarations resolve against an inherited property,
/// the value that property actually had where the element stood.
///
/// What an element reads is asked of its declarations and of its attributes together. A
/// presentation attribute is a paint the element names in the other syntax the file
/// carries, and it travels into the asset as text just as a rule does, so a fragment whose
/// only mention of the keyword is an attribute is unbound in exactly the same way.
pub fn rules(states: &[(&PageState, &BTreeMap<String, String>)], css: &str) -> String {
    let bodies = bodies_by_class(css);
    let mut bound = BTreeMap::<&str, String>::new();
    for (state, classes) in states {
        let nodes = state
            .nodes
            .iter()
            .map(|node| (node.path.as_str(), node))
            .collect::<HashMap<_, _>>();
        for node in &state.nodes {
            let Some(class) = classes.get(&node.path) else {
                continue;
            };
            let body = bodies.get(class.as_str()).map(String::as_str).unwrap_or("");
            if bound.contains_key(class.as_str()) {
                continue;
            }
            let read = format!(
                "{body} {}",
                node.attributes
                    .values()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let declarations = super::css_identifiers::implicit_references(&read)
                .iter()
                .filter(|property| {
                    super::css_identifiers::declared_value(body, property).is_none()
                })
                .filter_map(|property| {
                    Some(format!("{property}:{};", resolved(&nodes, node, property)?))
                })
                .collect::<String>();
            if !declarations.is_empty() {
                bound.insert(class, declarations);
            }
        }
    }
    bound
        .iter()
        .map(|(class, declarations)| format!(".{class}{{{declarations}}}\n"))
        .collect()
}

/// The value `property` had at `node`: the one recorded on the nearest element at or above
/// it, which is where the differencing capture put it.
fn resolved(nodes: &HashMap<&str, &Node>, node: &Node, property: &str) -> Option<String> {
    let mut current = Some(node);
    while let Some(node) = current {
        if let Some(value) = node.style.get(property) {
            return Some(value.clone());
        }
        current = node.parent.as_deref().and_then(|path| nodes.get(path).copied());
    }
    None
}

/// Every class the stylesheet writes declarations for, against the text of all of them.
///
/// Collected in one pass rather than searched per class, and across grouping at-rules as
/// well as the top level, so a keyword spelled only inside a responsive arm is still seen.
fn bodies_by_class(css: &str) -> HashMap<String, String> {
    let mut bodies = HashMap::new();
    collect(css, &mut bodies);
    bodies
}

fn collect(css: &str, bodies: &mut HashMap<String, String>) {
    for rule in super::css_rule_split::top_level(css) {
        let Some(start) = rule.find('{') else {
            continue;
        };
        if !rule.ends_with('}') {
            continue;
        }
        let prelude = &rule[..start];
        let body = &rule[start + 1..rule.len() - 1];
        if prelude.trim_start().starts_with('@') {
            collect(body, bodies);
            continue;
        }
        for class in classes(prelude) {
            bodies.entry(class.to_string()).or_default().push_str(body);
        }
    }
}

/// The class tokens a selector names. Generated class names are hashes, so a token is
/// enough to identify the rule's subject without parsing the selector.
fn classes(prelude: &str) -> Vec<&str> {
    let mut classes = Vec::new();
    let mut remaining = prelude;
    while let Some(index) = remaining.find('.') {
        remaining = &remaining[index + 1..];
        let end = remaining
            .find(|character: char| {
                !(character.is_alphanumeric() || character == '-' || character == '_')
            })
            .unwrap_or(remaining.len());
        if end > 0 {
            classes.push(&remaining[..end]);
        }
        remaining = &remaining[end..];
    }
    classes
}
