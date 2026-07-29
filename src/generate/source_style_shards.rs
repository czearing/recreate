use super::source_style_support::{brace_delta, format_css};

pub fn formatted(source: &str, maximum_bytes: usize, maximum_lines: usize) -> Vec<String> {
    let blocks = top_level_blocks(source);
    let mut shards = Vec::new();
    let mut current = String::new();
    let mut current_lines = 0_usize;
    for block in blocks {
        let formatted = format_css(&block);
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
    if !current.is_empty() {
        shards.push(current);
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
}
