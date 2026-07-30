use super::source_style_support::{brace_delta, format_css};

pub fn formatted(source: &str, maximum_bytes: usize, maximum_lines: usize) -> Vec<String> {
    let blocks = top_level_blocks(source);
    let mut shards = Vec::new();
    let mut current = String::new();
    let mut current_lines = 0_usize;
    for block in blocks {
        for formatted in split_oversized_at_rule(format_css(&block), maximum_bytes, maximum_lines) {
            let lines = formatted.lines().count();
            if !current.is_empty()
                && (current.len() + formatted.len() > maximum_bytes
                    || current_lines + lines > maximum_lines)
            {
                shards.push(std::mem::take(&mut current));
                current_lines = 0;
            }
            current.push_str(&formatted);
            current_lines += lines;
        }
    }
    if !current.is_empty() {
        shards.push(current);
    }
    shards
}

fn split_oversized_at_rule(
    formatted: String,
    maximum_bytes: usize,
    maximum_lines: usize,
) -> Vec<String> {
    if formatted.len() <= maximum_bytes && formatted.lines().count() <= maximum_lines {
        return vec![formatted];
    }
    let lines = formatted.lines().collect::<Vec<_>>();
    let Some(header) = lines.first().copied() else {
        return Vec::new();
    };
    if !["@media", "@supports", "@container"]
        .iter()
        .any(|prefix| header.trim_start().starts_with(prefix))
    {
        return vec![formatted];
    }
    let mut children = Vec::<String>::new();
    let mut current = String::new();
    let mut depth = brace_delta(header);
    for line in lines.iter().skip(1) {
        let delta = brace_delta(line);
        if depth == 1 && delta == -1 {
            break;
        }
        current.push_str(line);
        current.push('\n');
        depth += delta;
        if depth == 1 {
            children.push(std::mem::take(&mut current));
        }
    }
    if children.is_empty() || !current.trim().is_empty() {
        return vec![formatted];
    }
    let wrap = |body: &str| format!("{header}\n{body}}}\n");
    let mut shards = Vec::new();
    let mut body = String::new();
    for child in children {
        let candidate = format!("{body}{child}");
        let wrapped = wrap(&candidate);
        if !body.is_empty()
            && (wrapped.len() > maximum_bytes || wrapped.lines().count() > maximum_lines)
        {
            shards.push(wrap(&body));
            body.clear();
        }
        body.push_str(&child);
    }
    if !body.is_empty() {
        shards.push(wrap(&body));
    }
    shards
}

fn top_level_blocks(source: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut depth = 0_i32;
    for line in source.lines() {
        current.push_str(line);
        current.push('\n');
        depth += brace_delta(line);
        if depth == 0 {
            blocks.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        blocks.push(current);
    }
    blocks
}

#[cfg(test)]
mod tests {
    #[test]
    fn packs_formatted_rules_without_splitting_blocks() {
        let css = ".a{x:1;y:2;}\n.b{x:1;y:2;}\n.c{x:1;y:2;}\n";
        let shards = super::formatted(css, 1_000, 8);
        assert_eq!(shards.len(), 2);
        assert!(shards[0].contains(".a"));
        assert!(shards[0].contains(".b"));
        assert!(shards[1].contains(".c"));
    }

    #[test]
    fn splits_large_media_blocks_without_exceeding_line_budget() {
        let rules = (0..8)
            .map(|index| format!(".item-{index}{{color:red;background:white;}}"))
            .collect::<String>();
        let shards = super::formatted(&format!("@media(max-width:800px){{{rules}}}"), 1_000, 12);
        assert!(shards.len() > 1);
        assert!(
            shards
                .iter()
                .all(|shard| { shard.starts_with("@media") && shard.lines().count() <= 12 })
        );
        assert_eq!(
            shards
                .iter()
                .map(|shard| shard.matches(".item-").count())
                .sum::<usize>(),
            8
        );
    }
}
