use super::source_free_names::free_names;
use std::collections::BTreeSet;

/// The module a lifted item lands in re-establishes three things and nothing else: the components
/// the destination package exports, the shared-block namespace, and its own props. Every other
/// name the fragment carries would arrive unbound, so `unresolved` reports them and lets the
/// caller refuse a fragment this module could not honestly render.
pub fn unresolved(
    name: &str,
    template: &str,
    props: &[(usize, String)],
    exports: &BTreeSet<String>,
) -> BTreeSet<String> {
    free_names(template)
        .into_iter()
        .filter(|free| !resolves(free, name, template, props, exports))
        .collect()
}

/// Whether the destination module re-establishes a name the fragment carries.
fn resolves(
    free: &str,
    name: &str,
    template: &str,
    props: &[(usize, String)],
    exports: &BTreeSet<String>,
) -> bool {
    free == name
        || exports.contains(free)
        || props.iter().any(|(_, prop)| prop == free)
        || (free == "SharedComponents" && template.contains("SharedComponents."))
}

pub fn render(
    name: &str,
    template: &str,
    props: &[(usize, String)],
    exports: &BTreeSet<String>,
) -> String {
    let imported = free_names(template)
        .into_iter()
        .filter(|free| free != name && exports.contains(free))
        .collect::<BTreeSet<_>>();
    format!(
        "import React from 'react';\n{}{}\nexport function {name}({{{}}}) {{\n  return (\n{}\n  );\n}}\n",
        if template.contains("SharedComponents.") {
            "import * as SharedComponents from '../SharedComponents.jsx';\n"
        } else {
            ""
        },
        named_import(&imported, "../index.js"),
        props
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(","),
        indent(template)
    )
}

fn named_import(names: &BTreeSet<String>, path: &str) -> String {
    if names.is_empty() {
        String::new()
    } else {
        format!(
            "import {{{}}} from '{path}';\n",
            names.iter().cloned().collect::<Vec<_>>().join(",")
        )
    }
}

fn indent(source: &str) -> String {
    source
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
