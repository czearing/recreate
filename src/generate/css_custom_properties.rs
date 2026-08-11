use crate::model::Specification;
use std::collections::BTreeSet;

#[cfg(test)]
pub fn append(rules: &[String], declared: &BTreeSet<String>, css: &mut String) {
    let references = super::css_identifiers::references(css);
    append_values(rules, references, declared, css);
}

pub fn append_for_spec(
    specification: &Specification,
    rules: &[String],
    declared: &BTreeSet<String>,
    css: &mut String,
) {
    let mut references = super::css_identifiers::references(css);
    for state in &specification.states {
        for node in state.nodes.iter().chain(&state.startup_nodes) {
            for value in node.attributes.values() {
                references.extend(super::css_identifiers::references(value));
            }
        }
    }
    for interaction in &specification.interactions {
        for state in &interaction.states {
            for node in state.nodes.iter().chain(&state.startup_nodes) {
                for value in node.attributes.values() {
                    references.extend(super::css_identifiers::references(value));
                }
            }
        }
    }
    append_values(rules, references, declared, css);
}

/// Supplies an authored value only for a name no captured state declares. The
/// captured layer owns the rest and states them per viewport condition, so a
/// second unguarded declaration here would be a duplicate rule at best and an
/// uncancellable one at worst.
fn append_values(
    rules: &[String],
    references: BTreeSet<String>,
    declared: &BTreeSet<String>,
    css: &mut String,
) {
    let mut declarations = String::new();
    for name in references.difference(declared) {
        let values = rules
            .iter()
            .filter_map(|rule| super::css_identifiers::declared_value(rule, name))
            .collect::<BTreeSet<_>>();
        if values.len() == 1 {
            let value = values.into_iter().next().unwrap();
            // An empty custom property is not a value: `var()` reading it produces an
            // empty substitution, making the declaration invalid at computed-value time
            // so the property silently falls back to its initial value.
            if !value.trim().is_empty() {
                declarations.push_str(&format!("{name}:{value};"));
            }
        }
    }
    if !declarations.is_empty() {
        css.push_str(&format!(":root{{{declarations}}}\n"));
    }
}
