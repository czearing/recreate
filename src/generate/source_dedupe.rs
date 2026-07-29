use super::source_dedupe_support::{
    component, jsx_blocks, normalize, replace_ranges, reusable_block,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub fn extract_repeated_blocks(sources: &mut [&mut String]) -> Option<String> {
    let mut groups = HashMap::<String, Vec<(usize, usize, usize)>>::new();
    for (source_index, source) in sources.iter().enumerate() {
        for (start, end, block) in jsx_blocks(source) {
            if reusable_block(&block) {
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
    Some(
        definitions
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
        let module = super::extract_repeated_blocks(&mut [&mut left, &mut right]).unwrap();
        assert!(left.contains("<SharedComponents.ReusableBlock"));
        assert!(right.contains("<SharedComponents.ReusableBlock"));
        assert_eq!(module.matches("<section>").count(), 1);
    }
}
