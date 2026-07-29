use super::{
    source_css_compact::{compact, css_brace_delta, rule_classes},
    tree::Components,
};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct PartitionedCss {
    pub shared: String,
    pub components: BTreeMap<String, String>,
}

pub fn dedupe_exact(source: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let mut depth = 0_i32;
    let mut last = HashMap::<&str, usize>::new();
    for (index, line) in lines.iter().enumerate() {
        let delta = css_brace_delta(line);
        if depth == 0 && delta == 0 {
            last.insert(line, index);
        }
        depth += delta;
    }
    depth = 0;
    let mut output = String::new();
    for (index, line) in lines.into_iter().enumerate() {
        let delta = css_brace_delta(line);
        if depth != 0 || delta != 0 || last.get(line) == Some(&index) {
            output.push_str(line);
            output.push('\n');
        }
        depth += delta;
    }
    output
}

pub fn partition(source: &str, components: &Components) -> PartitionedCss {
    let mut owners = HashMap::<String, HashSet<String>>::new();
    let mut global = HashSet::<String>::new();
    for (path, class_names) in &components.classes {
        let path_owners = components
            .items
            .iter()
            .filter(|component| {
                component.roots.iter().any(|root| {
                    path == root
                        || path
                            .strip_prefix(root)
                            .is_some_and(|suffix| suffix.starts_with('>'))
                })
            })
            .map(|component| component.name.clone())
            .collect::<HashSet<_>>();
        for class_name in class_names.split_whitespace() {
            if path_owners.is_empty() {
                global.insert(class_name.to_string());
            }
            owners
                .entry(class_name.to_string())
                .or_default()
                .extend(path_owners.iter().cloned());
        }
    }
    let mut shared = String::new();
    let mut by_component = BTreeMap::<String, String>::new();
    let mut depth = 0_i32;
    for line in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let delta = css_brace_delta(line);
        let owner = (depth == 0 && delta == 0)
            .then(|| rule_classes(line))
            .flatten()
            .and_then(|classes| {
                let mut resolved = None;
                for class_name in classes {
                    if global.contains(&class_name) {
                        return None;
                    }
                    let class_owners = owners.get(&class_name)?;
                    if class_owners.len() != 1 {
                        return None;
                    }
                    let current = class_owners.iter().next()?.clone();
                    if resolved.as_ref().is_some_and(|owner| owner != &current) {
                        return None;
                    }
                    resolved = Some(current);
                }
                resolved
            });
        if let Some(owner) = owner {
            by_component
                .entry(owner)
                .or_default()
                .push_str(&format!("{line}\n"));
        } else {
            shared.push_str(line);
            shared.push('\n');
        }
        depth += delta;
    }

    PartitionedCss {
        shared: compact(&shared),
        components: by_component
            .into_iter()
            .map(|(name, css)| (name, compact(&css)))
            .collect(),
    }
}
