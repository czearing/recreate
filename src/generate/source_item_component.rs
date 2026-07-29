use super::source_dedupe_support::uppercase_tags;
use std::collections::BTreeSet;

pub fn render(name: &str, template: &str, props: &[(usize, String)]) -> String {
    let components = uppercase_tags(template)
        .into_iter()
        .filter(|tag| {
            !tag.starts_with("Generated") && tag != "SharedComponents" && tag != "CollectionItems"
        })
        .collect::<BTreeSet<_>>();
    format!(
        "import React from 'react';\n{}{}\nexport function {name}({{{}}}) {{\n  return (\n{}\n  );\n}}\n",
        if template.contains("SharedComponents.") {
            "import * as SharedComponents from '../SharedComponents.jsx';\n"
        } else {
            ""
        },
        named_import(&components, "../index.js"),
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
