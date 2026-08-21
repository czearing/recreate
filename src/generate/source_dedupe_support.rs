pub fn replace_ranges(source: &mut String, replacements: &mut Vec<(usize, usize, String)>) {
    replacements.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    for (start, end, replacement) in replacements.drain(..) {
        source.replace_range(start..end, &replacement);
    }
}

pub fn svg_blocks(source: &str) -> Vec<(usize, usize, String)> {
    let mut blocks = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = source[offset..].find("<svg") {
        let start = offset + relative_start;
        let Some(relative_end) = source[start..].find("</svg>") else {
            break;
        };
        let end = start + relative_end + "</svg>".len();
        blocks.push((start, end, source[start..end].to_string()));
        offset = end;
    }
    blocks
}

pub fn jsx_blocks(source: &str) -> Vec<(usize, usize, String)> {
    let mut lines = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let content = line.trim_end_matches('\n');
        lines.push((offset, content));
        offset += line.len();
    }
    let mut blocks = Vec::new();
    for (index, (start, line)) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let Some(tag) = opening_tag(trimmed) else {
            continue;
        };
        let closing = format!("{}</{tag}>", " ".repeat(indent));
        if let Some((end, _)) = lines
            .iter()
            .skip(index + 1)
            .find(|(_, candidate)| *candidate == closing)
        {
            let end = *end + closing.len();
            blocks.push((*start, end, source[*start..end].to_string()));
        }
    }
    blocks
}

fn opening_tag(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('<')?;
    let length = rest
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '-')
        .map(char::len_utf8)
        .sum();
    (length > 0 && line.ends_with('>') && !line.ends_with("/>")).then_some(&rest[..length])
}

pub fn reusable_svg(svg: &str) -> bool {
    svg.len() >= 80
        && !svg.contains("data-recreate-trigger")
        && !svg.contains("onClick=")
        && !svg.contains("onKeyDown=")
}

/// A block this stage relocates lands in a module that defines nothing itself, so every name it
/// uses must come from somewhere. A name the package exports is one the module can simply import,
/// so it is no obstacle; only a name nothing exports, such as a local binding of the view it came
/// from, would arrive undefined. The trigger attribute is a separate hazard: it is a
/// document-scoped value rather than a free name, and duplicating it would point two elements at
/// one identity.
///
/// Size is deliberately not the test of whether lifting is worth it: what a lift saves depends on
/// how many copies it removes, which is only known once a block's occurrences are grouped. This
/// checks that a block is SAFE to lift and large enough to be worth naming; the caller decides
/// whether it pays.
pub fn reusable_block(block: &str, exported: &std::collections::BTreeSet<String>) -> bool {
    (120..=100_000).contains(&block.len())
        && !block.contains("data-recreate-trigger")
        && super::source_free_names::free_names(block)
            .iter()
            .all(|name| exported.contains(name))
}

pub fn normalize(source: &str) -> String {
    let indentation = source
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or_default();
    source
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                line.trim_start()
            } else {
                line.get(indentation..).unwrap_or(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn component(name: &str, source: &str) -> String {
    format!(
        "export function {name}() {{\n  return (\n{}\n  );\n}}\n",
        indent(source, 2)
    )
}

fn indent(source: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    source
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
