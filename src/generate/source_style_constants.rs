use std::collections::BTreeMap;

/// A generated rule is a computed-style snapshot, so most of what it declares is a property that
/// holds the same value everywhere in the stylesheet. Such a declaration cannot conflict with
/// itself, so emitting it once for the selectors that share it changes nothing any element
/// computes while removing every other copy.
///
/// Only unconditional rules take part. A declaration inside an at-rule is conditional on it, and
/// hoisting it out would apply it at widths and in environments the rule excluded.
pub fn group_constants(css: &str) -> String {
    let rules = unconditional_rules(css);
    let mut values = BTreeMap::<&str, Vec<&str>>::new();
    let mut appearances = BTreeMap::<&str, usize>::new();
    for (_, body) in &rules {
        for (property, value) in declarations(body) {
            let seen = values.entry(property).or_default();
            if !seen.contains(&value) {
                seen.push(value);
            }
            *appearances.entry(property).or_default() += 1;
        }
    }

    let mut blocks = BTreeMap::<String, Vec<&str>>::new();
    for (selector, body) in &rules {
        let shared = declarations(body)
            .filter(|(property, _)| {
                values.get(property).is_some_and(|seen| seen.len() == 1)
                    && appearances.get(property).is_some_and(|count| *count > 1)
            })
            .map(|(property, value)| format!("{property}:{value}"))
            .collect::<Vec<_>>();
        if shared.is_empty() {
            continue;
        }
        blocks.entry(shared.join(";")).or_default().push(selector);
    }
    blocks.retain(|_, selectors| selectors.len() > 1);
    if blocks.is_empty() {
        return css.to_string();
    }

    let hoisted = blocks
        .keys()
        .flat_map(|block| block.split(';').map(|entry| entry.to_string()))
        .collect::<std::collections::BTreeSet<_>>();
    let mut output = String::new();
    for (block, selectors) in &blocks {
        output.push_str(&format!("{}{{{block}}}\n", selectors.join(",")));
    }
    let mut depth = 0;
    for line in css.lines() {
        let rule = (depth == 0).then(|| split_rule(line)).flatten();
        match rule {
            Some((selector, body)) if blocks.values().any(|list| list.contains(&selector)) => {
                let kept = declarations(body)
                    .filter(|(property, value)| !hoisted.contains(&format!("{property}:{value}")))
                    .map(|(property, value)| format!("{property}:{value}"))
                    .collect::<Vec<_>>();
                if !kept.is_empty() {
                    output.push_str(&format!("{selector}{{{}}}\n", kept.join(";")));
                }
            }
            _ => {
                depth += super::source_style_support::brace_delta(line);
                output.push_str(line);
                output.push('\n');
            }
        }
    }
    output
}

fn unconditional_rules(css: &str) -> Vec<(&str, &str)> {
    let mut rules = Vec::new();
    let mut depth = 0;
    for line in css.lines() {
        if depth == 0 {
            if let Some(rule) = split_rule(line) {
                rules.push(rule);
                continue;
            }
        }
        depth += super::source_style_support::brace_delta(line);
    }
    rules
}

/// A rule this pass may touch is one whole line: a selector, one brace pair, and nothing else. A
/// line that opens a block it does not close belongs to an at-rule and is left alone.
fn split_rule(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.starts_with('@') || !trimmed.ends_with('}') {
        return None;
    }
    let open = trimmed.find('{')?;
    let body = &trimmed[open + 1..trimmed.len() - 1];
    if body.contains('{') || body.contains('}') {
        return None;
    }
    Some((trimmed[..open].trim(), body))
}

fn declarations(body: &str) -> impl Iterator<Item = (&str, &str)> {
    body.split(';').filter_map(|entry| {
        let entry = entry.trim();
        let colon = entry.find(':')?;
        Some((entry[..colon].trim(), entry[colon + 1..].trim()))
    })
}

#[cfg(test)]
#[path = "source_style_constants_tests.rs"]
mod tests;
