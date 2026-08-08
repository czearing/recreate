//! The single owner of "can React express this captured attribute on the element that
//! carried it, and if not, where does it go instead?".
//!
//! `jsx_attr_names` answers what an attribute is *called*. That is not enough: React
//! discards some DOM-legal authoring outright. A `<option selected>` is the clearest case —
//! React derives selection solely from `value`/`defaultValue` on the parent `<select>`, so
//! the captured attribute reaches the generated file intact and is thrown away at render.
//! A grep for it succeeds precisely when the screen is wrong.
//!
//! A name table structurally cannot express this repair, because it writes to an element
//! *other than* the one carrying the attribute. So a relocation is declared once, as a
//! whole, and both halves are derived from it: the attribute is suppressed where it was
//! captured, and the prop is emitted on the ancestor. Declaring the halves separately would
//! let them drift into a dropped attribute with no replacement, or a duplicated one.

use super::tree::Components;
use crate::model::Node;

/// An attribute whose effect React expresses on a different element than HTML does.
struct Relocation {
    /// The captured attribute, inert on `from`.
    attribute: &'static str,
    /// The element that carries the attribute in HTML.
    from: &'static str,
    /// The ancestor React reads the state from.
    to: &'static str,
    /// The prop that ancestor carries instead.
    prop: &'static str,
}

const RELOCATIONS: &[Relocation] = &[Relocation {
    attribute: "selected",
    from: "option",
    to: "select",
    prop: "defaultValue",
}];

/// Whether `attribute` is inert on `tag` because an ancestor carries it instead. Emitting it
/// anyway would be a prop React ignores.
pub(super) fn relocated(tag: &str, attribute: &str) -> bool {
    RELOCATIONS
        .iter()
        .any(|rule| rule.from == tag && rule.attribute == attribute)
}

/// The props `path` gains from descendants that cannot express them themselves.
pub(super) fn adopted(path: &str, components: &Components) -> String {
    let Some(node) = components.nodes.get(path) else {
        return String::new();
    };
    RELOCATIONS
        .iter()
        .filter(|rule| rule.to == node.tag)
        .filter_map(|rule| {
            let mut values = Vec::new();
            collect(path, components, rule, &mut values);
            // Nothing marked means the host already agrees with the browser: a single
            // control shows its first option and a multiple control shows none. Emitting a
            // selection here would assert something the source never did.
            (!values.is_empty()).then(|| format!(" {}={}", rule.prop, literal(node, values)))
        })
        .collect()
}

/// A control that accepts several selections takes a list; one that does not takes the
/// single value, and React silently selects nothing if given the wrong shape.
fn literal(node: &Node, values: Vec<String>) -> String {
    let json = if node.attributes.contains_key("multiple") {
        serde_json::to_string(&values)
    } else {
        serde_json::to_string(&values[0])
    };
    format!("{{{}}}", json.unwrap())
}

/// Walks descendants rather than children, because options are routinely grouped inside
/// `<optgroup>`. A nested adopting element ends the walk, since it owns its own descendants.
fn collect(path: &str, components: &Components, rule: &Relocation, values: &mut Vec<String>) {
    for child in components.children.get(path).into_iter().flatten() {
        let Some(node) = components.nodes.get(child) else {
            continue;
        };
        if node.tag == rule.from && node.attributes.contains_key(rule.attribute) {
            values.push(value_of(child, node, components));
        }
        if node.tag != rule.to {
            collect(child, components, rule, values);
        }
    }
}

/// An option's value is its `value` attribute, or its text content when that is absent.
/// Reading only the attribute would emit an empty selection for the commonest form.
fn value_of(path: &str, node: &Node, components: &Components) -> String {
    node.attributes
        .get("value")
        .cloned()
        .unwrap_or_else(|| text_of(path, components))
}

fn text_of(path: &str, components: &Components) -> String {
    let mut text = String::new();
    for child in components.children.get(path).into_iter().flatten() {
        let Some(node) = components.nodes.get(child) else {
            continue;
        };
        if node.tag == "#text" {
            text.push_str(&node.text);
        } else {
            text.push_str(&text_of(child, components));
        }
    }
    text
}
