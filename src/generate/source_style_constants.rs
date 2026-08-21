use std::collections::{BTreeMap, BTreeSet};

/// A generated rule is a computed-style snapshot, so the same declaration is repeated in hundreds
/// of rules. Emitting one copy for the selectors that share it changes nothing any element
/// computes while removing every other copy.
///
/// What may be shared is a property AND value together, not a property that happens to hold one
/// value everywhere: requiring global constancy loses a declaration repeated in 317 rules because
/// a single 318th rule disagrees.
///
/// Only unconditional rules take part. A declaration inside an at-rule is conditional on it, and
/// hoisting it out would apply it at widths and in environments the rule excluded.
pub fn group_constants(css: &str) -> String {
    let rules = unconditional_rules(css);
    let mut appearances = BTreeMap::<(&str, &str), usize>::new();
    let mut resolved = BTreeMap::<(&str, &str), BTreeSet<&str>>::new();
    for (selector, body) in &rules {
        for (property, value) in declarations(body) {
            *appearances.entry((property, value)).or_default() += 1;
            resolved
                .entry((*selector, property))
                .or_default()
                .insert(value);
        }
    }

    // Moving a declaration to the front only preserves the cascade when nothing else can win it
    // back. A selector carried by two rules that disagree on a property is decided by their order,
    // so such a property is left where it is.
    let shareable = |selector: &str, property: &str, value: &str| {
        appearances.get(&(property, value)).is_some_and(|n| *n > 1)
            && resolved
                .get(&(selector, property))
                .is_some_and(|values| values.len() == 1)
    };

    let mut blocks = BTreeMap::<String, Vec<String>>::new();
    for (selector, body) in &rules {
        for (property, value) in declarations(body) {
            if shareable(selector, property, value) {
                blocks
                    .entry(format!("{property}:{value}"))
                    .or_default()
                    .push((*selector).to_string());
            }
        }
    }
    blocks.retain(|_, selectors| selectors.len() > 1);

    // Group by the exact set of rules a declaration covers, so one selector list carries every
    // declaration those rules share. Grouping the other way round -- rules by the declarations they
    // share -- splinters as soon as values differ, and each splinter pays for the selector list
    // again; measured, that made the stylesheet larger rather than smaller.
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for (declaration, selectors) in blocks {
        grouped
            .entry(selectors.join(","))
            .or_default()
            .push(declaration);
    }

    // A group only earns its place if stating it once costs less than the copies it removes.
    grouped.retain(|selectors, declarations| {
        let block = declarations.iter().map(|entry| entry.len() + 1).sum::<usize>();
        let count = selectors.split(',').count();
        block * count > block + selectors.len()
    });
    if grouped.is_empty() {
        return css.to_string();
    }

    let hoisted = grouped
        .iter()
        .flat_map(|(selectors, declarations)| {
            selectors.split(',').flat_map(move |selector| {
                declarations
                    .iter()
                    .map(move |entry| (selector, entry.as_str()))
            })
        })
        .collect::<BTreeSet<_>>();
    let touched = hoisted
        .iter()
        .map(|(selector, _)| *selector)
        .collect::<BTreeSet<_>>();
    let mut output = String::new();
    for (selectors, declarations) in &grouped {
        output.push_str(&format!("{selectors}{{{}}}\n", declarations.join(";")));
    }
    let mut depth = 0;
    for line in css.lines() {
        let rule = (depth == 0).then(|| split_rule(line)).flatten();
        match rule {
            Some((selector, body)) if touched.contains(selector) => {
                let kept = declarations(body)
                    .filter(|(property, value)| {
                        !hoisted.contains(&(selector, format!("{property}:{value}").as_str()))
                    })
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
