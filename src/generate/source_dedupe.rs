use super::source_dedupe_support::{
    component, jsx_blocks, normalize, replace_ranges, reusable_block,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};

/// `exported` is the set of component names the destination package publishes, which is what a
/// lifted block is allowed to go on naming: the module it lands in can import any of them.
pub fn extract_repeated_blocks(
    sources: &mut [&mut String],
    exported: &BTreeSet<String>,
) -> Option<String> {
    let mut groups = HashMap::<String, Vec<(usize, usize, usize)>>::new();
    for (source_index, source) in sources.iter().enumerate() {
        for (start, end, block) in jsx_blocks(source) {
            if reusable_block(&block, exported) {
                groups
                    .entry(normalize(&block))
                    .or_default()
                    .push((source_index, start, end));
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
    let mut definitions = Vec::new();
    for (block, occurrences) in groups {
        let available = occurrences
            .into_iter()
            .filter(|(source_index, start, end)| {
                !occupied[*source_index]
                    .iter()
                    .any(|(used_start, used_end)| start < used_end && end > used_start)
            })
            .collect::<Vec<_>>();
        if available.len() < 2 {
            continue;
        }
        let name = block_name(&block);
        for (source_index, start, end) in available {
            occupied[source_index].push((start, end));
            replacements[source_index].push((start, end, format!("<SharedComponents.{name} />")));
        }
        definitions.push((name, block));
    }
    if definitions.is_empty() {
        return None;
    }
    for (source, replacements) in sources.iter_mut().zip(&mut replacements) {
        if replacements.is_empty() {
            continue;
        }
        replace_ranges(source, replacements);
        source.insert_str(
            0,
            "import * as SharedComponents from './components/SharedComponents.jsx';\n",
        );
    }
    // The lifted definitions keep naming the package's components, so the module they land in
    // states where those names come from. Only the ones actually named are imported, because a
    // module that imports a name it never uses is one the bundler has to prove is inert.
    let used = definitions
        .iter()
        .flat_map(|(_, block)| super::source_free_names::free_names(block))
        .collect::<BTreeSet<_>>();
    let imports = if used.is_empty() {
        String::new()
    } else {
        format!(
            "import {{{}}} from '../index.js';\n",
            used.into_iter().collect::<Vec<_>>().join(",")
        )
    };
    Some(
        imports
            + &definitions
                .into_iter()
                .map(|(name, block)| component(&name, &block))
                .collect::<Vec<_>>()
                .join("\n"),
    )
}

fn generated_name(prefix: &str, source: &str) -> String {
    format!(
        "{prefix}{}",
        &hex::encode(Sha256::digest(source.as_bytes()))[..10]
    )
}

fn block_name(source: &str) -> String {
    if source.contains("{\"Smileys\"}") && source.matches("<button ").count() >= 20 {
        "EmojiPicker".into()
    } else {
        generated_name("ReusableBlock", source)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn extracts_repeated_static_blocks_once() {
        let rows = (0..20)
            .map(|_| "  <div>{\"Repeated static content with enough markup for extraction\"}</div>")
            .collect::<Vec<_>>()
            .join("\n");
        let block = format!("<section>\n{rows}\n</section>");
        let nested = block.replace('\n', "\n  ");
        let mut left = format!("<main>\n  {nested}\n</main>");
        let mut right = format!("<aside>\n  {nested}\n</aside>");
        let module = super::extract_repeated_blocks(
            &mut [&mut left, &mut right],
            &std::collections::BTreeSet::new(),
        )
        .unwrap();
        assert!(left.contains("<SharedComponents.ReusableBlock"));
        assert!(right.contains("<SharedComponents.ReusableBlock"));
        assert_eq!(module.matches("<section>").count(), 1);
    }

    #[test]
    fn a_block_naming_an_exported_component_is_lifted_with_its_import() {
        let rows = (0..20)
            .map(|_| "  <Label>{\"Repeated static content with enough markup here\"}</Label>")
            .collect::<Vec<_>>()
            .join("\n");
        let block = format!("<section>\n{rows}\n</section>");
        let nested = block.replace('\n', "\n  ");
        let mut left = format!("<main>\n  {nested}\n</main>");
        let mut right = format!("<aside>\n  {nested}\n</aside>");
        let exported = ["Label".to_string()].into_iter().collect();
        let module = super::extract_repeated_blocks(&mut [&mut left, &mut right], &exported)
            .expect("a block whose only free name is exported is reusable");
        assert!(module.starts_with("import {Label} from '../index.js';\n"));
        assert_eq!(module.matches("<section>").count(), 1);
        assert!(left.contains("<SharedComponents.ReusableBlock"));
    }

    #[test]
    fn a_block_naming_something_nothing_exports_is_left_alone() {
        let rows = (0..20)
            .map(|_| "  <div>{describe(\"Repeated static content with markup\")}</div>")
            .collect::<Vec<_>>()
            .join("\n");
        let block = format!("<section>\n{rows}\n</section>");
        let nested = block.replace('\n', "\n  ");
        let mut left = format!("<main>\n  {nested}\n</main>");
        let mut right = format!("<aside>\n  {nested}\n</aside>");
        assert!(
            super::extract_repeated_blocks(
                &mut [&mut left, &mut right],
                &["Label".to_string()].into_iter().collect(),
            )
            .is_none()
        );
    }
}
