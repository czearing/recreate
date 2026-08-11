use crate::model::Specification;
use std::collections::BTreeSet;

#[cfg(test)]
pub fn append(rules: &[String], declared: &BTreeSet<String>, css: &mut String) {
    let references = references(css);
    append_values(rules, references, declared, css);
}

pub fn append_for_spec(
    specification: &Specification,
    rules: &[String],
    declared: &BTreeSet<String>,
    css: &mut String,
) {
    let mut references = references(css);
    for state in &specification.states {
        for node in state.nodes.iter().chain(&state.startup_nodes) {
            for value in node.attributes.values() {
                references.extend(self::references(value));
            }
        }
    }
    for interaction in &specification.interactions {
        for state in &interaction.states {
            for node in state.nodes.iter().chain(&state.startup_nodes) {
                for value in node.attributes.values() {
                    references.extend(self::references(value));
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
            .filter_map(|rule| value(rule, name))
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

fn references(css: &str) -> BTreeSet<String> {
    let mut references = BTreeSet::new();
    let mut remaining = css;
    while let Some(index) = remaining.find("var(--") {
        remaining = &remaining[index + 4..];
        let end = remaining
            .find([',', ')', ' ', '\t'])
            .unwrap_or(remaining.len());
        references.insert(remaining[..end].to_string());
        remaining = &remaining[end..];
    }
    references
}

fn value(rule: &str, name: &str) -> Option<String> {
    let mut remaining = rule;
    while let Some(index) = remaining.find(name) {
        remaining = &remaining[index + name.len()..];
        let candidate = remaining.trim_start();
        if let Some(value) = candidate.strip_prefix(':') {
            let end = value.find([';', '}']).unwrap_or(value.len());
            return Some(value[..end].trim().to_string());
        }
    }
    None
}
