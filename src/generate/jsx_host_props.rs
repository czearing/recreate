//! The single owner of "can React express this captured control state on the element that
//! carried it, and if not, where does it go instead?".
//!
//! `jsx_attr_names` answers what an attribute is *called*. That is not enough for control
//! state, for two reasons React imposes and HTML does not.
//!
//! It rebinds. A `<option selected>` is the clearest case — React derives selection solely
//! from `value`/`defaultValue` on the parent `<select>`, so the captured attribute reaches
//! the generated file intact and is thrown away at render. A grep for it succeeds precisely
//! when the screen is wrong. A name table structurally cannot express that repair, because
//! it writes to an element *other than* the one carrying the attribute.
//!
//! It renames by intent. `value` emitted as `value` makes a *controlled* input: with no
//! `onChange` React pins the field and it becomes read-only. A recreation is a snapshot of
//! a page in a state, not an app owning that state, so every control here is uncontrolled
//! and the prop is `defaultValue`/`defaultChecked`. The read-only form renders an identical
//! pixel, which is why nothing downstream can catch it.
//!
//! Both are declared as whole bindings and both halves derived from each: the attribute is
//! suppressed where it was captured, and the prop is emitted where React reads it. Declaring
//! the halves separately would let them drift into a dropped attribute with no replacement.
//!
//! Every binding reads [`state`], never the attribute map directly. That is the one place
//! that knows a live value outranks the markup default, so a control the page never touched
//! and one the user typed into travel the same path.

use super::tree::Components;
use crate::model::Node;

/// A binding React expresses somewhere other than where HTML wrote it, or under another
/// name. `from == to` is the rename case; they differ when React reads an ancestor.
struct Binding {
    /// The content attribute whose default this state overrides.
    attribute: &'static str,
    /// The element that holds the state in HTML.
    from: &'static str,
    /// The element React reads it from.
    to: &'static str,
    /// The prop that element carries instead.
    prop: &'static str,
}

const BINDINGS: &[Binding] = &[
    Binding {
        attribute: "selected",
        from: "option",
        to: "select",
        prop: "defaultValue",
    },
    Binding {
        attribute: "value",
        from: "input",
        to: "input",
        prop: "defaultValue",
    },
    Binding {
        attribute: "value",
        from: "textarea",
        to: "textarea",
        prop: "defaultValue",
    },
    Binding {
        attribute: "checked",
        from: "input",
        to: "input",
        prop: "defaultChecked",
    },
];

/// The effective value of a control-state attribute: what the page currently says, falling
/// back to what its markup authored. `Some(None)` is a default the page turned off, which is
/// why this cannot be a plain `or_else` over two maps.
fn state(node: &Node, attribute: &str) -> Option<String> {
    match node.control_state.get(attribute) {
        Some(live) => live.clone(),
        None => node.attributes.get(attribute).cloned(),
    }
}

/// Whether `attribute` is inert on this element because React reads the state from a prop,
/// or because the recreation restores it by other means. Emitting it anyway would be a prop
/// React ignores, or one that freezes the control.
pub(super) fn relocated(node: &Node, attribute: &str) -> bool {
    if super::jsx_promotion::withholds(node, attribute) {
        return true;
    }
    BINDINGS
        .iter()
        .any(|rule| rule.from == node.tag && rule.attribute == attribute)
}

/// Whether this element's value arrives as a prop, so the children that spelled it in HTML
/// must not also be emitted. A `<textarea>`'s default value *is* its child text, and React
/// rejects a textarea that carries both.
pub(super) fn binds_value(node: &Node) -> bool {
    BINDINGS.iter().any(|rule| {
        rule.from == node.tag && rule.to == node.tag && state(node, rule.attribute).is_some()
    })
}

/// The props `path` carries for state React will not read where HTML put it.
pub(super) fn adopted(path: &str, components: &Components) -> String {
    let Some(node) = components.nodes.get(path) else {
        return String::new();
    };
    BINDINGS
        .iter()
        .filter(|rule| rule.to == node.tag)
        .filter_map(|rule| {
            if rule.from == rule.to {
                return state(node, rule.attribute)
                    .map(|value| format!(" {}={}", rule.prop, own(rule, &value)));
            }
            let mut values = Vec::new();
            collect(path, components, rule, &mut values);
            // Nothing marked means the host already agrees with the browser: a single
            // control shows its first option and a multiple control shows none. Emitting a
            // selection here would assert something the source never did.
            (!values.is_empty()).then(|| format!(" {}={}", rule.prop, literal(node, values)))
        })
        .collect()
}

/// A boolean binding is spelled as a boolean, not as the empty string HTML uses to mean
/// present, because React reads `defaultChecked=""` as false.
fn own(rule: &Binding, value: &str) -> String {
    if rule.prop == "defaultChecked" {
        return "{true}".into();
    }
    format!("{{{}}}", serde_json::to_string(value).unwrap())
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
fn collect(path: &str, components: &Components, rule: &Binding, values: &mut Vec<String>) {
    for child in components.children.get(path).into_iter().flatten() {
        let Some(node) = components.nodes.get(child) else {
            continue;
        };
        if node.tag == rule.from && state(node, rule.attribute).is_some() {
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
