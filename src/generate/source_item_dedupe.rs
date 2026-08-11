use super::source_dedupe_support::{jsx_blocks, normalize, replace_ranges};
use std::collections::{BTreeSet, HashMap};

pub struct GeneratedItem {
    pub name: String,
    pub source: String,
}

/// `components` is what the destination package exports: the set a lifted item may render, and
/// the set its own generated name must not shadow. One set answers both, because a name the
/// destination exports is exactly a name it would collide with.
pub fn extract(sources: &mut [&mut String], components: &BTreeSet<String>) -> Vec<GeneratedItem> {
    let entity = super::source_item_names::collection_entity(sources);
    let mut groups = HashMap::<String, Vec<Occurrence>>::new();
    for (source_index, source) in sources.iter().enumerate() {
        for (start, end, block) in jsx_blocks(source) {
            if reusable(&block) {
                let (signature, values) = fields(&normalize(&block));
                groups.entry(signature).or_default().push(Occurrence {
                    source: source_index,
                    start,
                    end,
                    values,
                });
            }
        }
    }
    let mut groups = groups
        .into_iter()
        .filter(|(_, occurrences)| occurrences.len() > 1)
        .collect::<Vec<_>>();
    groups.sort_by(|(left, _), (right, _)| {
        right.len().cmp(&left.len()).then_with(|| left.cmp(right))
    });
    let mut occupied = vec![Vec::<(usize, usize)>::new(); sources.len()];
    let mut replacements = vec![Vec::<(usize, usize, String)>::new(); sources.len()];
    let mut generated = Vec::new();
    let mut names = HashMap::<String, usize>::new();
    for (signature, occurrences) in groups {
        let available = occurrences
            .into_iter()
            .filter(|item| {
                !occupied[item.source]
                    .iter()
                    .any(|(start, end)| item.start < *end && item.end > *start)
            })
            .collect::<Vec<_>>();
        if available.len() < 2 {
            continue;
        }
        let varying = varying_fields(&available);
        if varying.is_empty() {
            continue;
        }
        let fields =
            super::source_item_names::prop_fields(&signature, &available[0].values, &varying);
        let props = super::source_item_names::prop_names(&signature, &available[0].values, &fields);
        let template = template(&signature, &available[0].values, &props);
        if !super::source_item_component::unresolved("", &template, &props, components).is_empty() {
            continue;
        }
        let base = super::source_item_names::item_name(
            &signature,
            &available[0].values,
            entity.as_deref(),
        );
        let variant = names.entry(base.clone()).or_default();
        *variant += 1;
        let mut name = if *variant == 1 {
            base.clone()
        } else {
            format!("{base}Variant{variant}")
        };
        while components.contains(&name) {
            *variant += 1;
            name = format!("{base}Variant{variant}");
        }
        for item in available {
            occupied[item.source].push((item.start, item.end));
            replacements[item.source].push((
                item.start,
                item.end,
                invocation(&name, &item.values, &props),
            ));
        }
        generated.push(GeneratedItem {
            source: super::source_item_component::render(&name, &template, &props, components),
            name,
        });
    }
    for (source, replacements) in sources.iter_mut().zip(&mut replacements) {
        if replacements.is_empty() {
            continue;
        }
        replace_ranges(source, replacements);
        source.insert_str(
            0,
            "import * as CollectionItems from './components/CollectionItems/index.js';\n",
        );
    }
    generated
}

struct Occurrence {
    source: usize,
    start: usize,
    end: usize,
    values: Vec<String>,
}

/// The cheap shape gate: a fragment worth naming as a collection item. Whether it can be lifted
/// soundly is a separate question, answered once the props are known, since a name this module
/// cannot resolve is only unbound relative to what the destination re-establishes.
pub(super) fn reusable(block: &str) -> bool {
    let root = block.lines().next().unwrap_or_default();
    (160..=60_000).contains(&block.len())
        && (root.contains("data-testid=") || root.contains("role={\"button\"}"))
}

fn fields(source: &str) -> (String, Vec<String>) {
    let mut signature = String::new();
    let mut values = Vec::new();
    let mut remaining = source;
    while let Some(start) = remaining.find("{\"") {
        signature.push_str(&remaining[..start]);
        let Some(end) = remaining[start + 2..].find("\"}") else {
            break;
        };
        let end = start + 2 + end + 2;
        values.push(remaining[start + 1..end - 1].to_string());
        signature.push_str(&format!("{{{{FIELD{}}}}}", values.len() - 1));
        remaining = &remaining[end..];
    }
    signature.push_str(remaining);
    (signature, values)
}

fn varying_fields(items: &[Occurrence]) -> Vec<usize> {
    (0..items[0].values.len())
        .filter(|index| {
            items
                .iter()
                .skip(1)
                .any(|item| item.values.get(*index) != items[0].values.get(*index))
        })
        .collect()
}

fn template(signature: &str, values: &[String], props: &[(usize, String)]) -> String {
    let mut output = signature.to_string();
    for index in 0..128 {
        let marker = format!("{{{{FIELD{index}}}}}");
        if !output.contains(&marker) {
            continue;
        }
        let value = if let Some((_, name)) = props.iter().find(|(field, _)| *field == index) {
            format!("{{{name}}}")
        } else {
            values
                .get(index)
                .map(|value| format!("{{{value}}}"))
                .unwrap_or_else(|| "{\"\"}".into())
        };
        output = output.replace(&marker, &value);
    }
    output
}

fn invocation(name: &str, values: &[String], props: &[(usize, String)]) -> String {
    let props = props
        .iter()
        .filter_map(|(index, name)| {
            values
                .get(*index)
                .map(|value| format!(" {name}={{{value}}}"))
        })
        .collect::<String>();
    format!("<CollectionItems.{name}{props} />")
}
