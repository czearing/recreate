//! The authored `@starting-style` declarations, read out of either authoring form.
//!
//! The construct has two shapes and the capture preserves both verbatim rather than
//! normalising one into the other. Written at the top level the block wraps a style rule
//! that carries its own selector; written inside a style rule it carries bare declarations
//! and is reached by the rule enclosing it. A reader that recognises only the outermost
//! prelude sees the first and silently skips the second, which is the whole of the loss this
//! module exists to prevent.
//!
//! One walk covers both, by remembering the prelude of every block it descends into and
//! treating that prelude as the selector context. An at-rule prelude names no element, so
//! descending through one keeps the context it was found in — which is what makes a
//! `@media` wrapper around either form transparent instead of a third case.

/// Every `@starting-style` block, paired with the selector that reaches its declarations.
pub(super) fn declarations(rules: &[String]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for rule in rules {
        if !rule.contains("@starting-style") {
            continue;
        }
        let (_, rule) = super::css_layers::peel(rule);
        walk(rule, "", &mut found);
    }
    found
}

fn walk(text: &str, selector: &str, found: &mut Vec<(String, String)>) {
    for (prelude, body) in blocks(text) {
        if prelude.eq_ignore_ascii_case("@starting-style") {
            gather(body, selector, found);
        } else {
            walk(body, context(prelude, selector), found);
        }
    }
}

/// The declarations of one `@starting-style` block and of every style rule inside it.
fn gather(body: &str, selector: &str, found: &mut Vec<(String, String)>) {
    let loose = outside_blocks(body);
    if !loose.trim().is_empty() && !selector.is_empty() {
        found.push((selector.to_string(), loose));
    }
    for (prelude, inner) in blocks(body) {
        gather(inner, context(prelude, selector), found);
    }
}

/// The selector a block's contents are reached by. An at-rule prelude names no element, so a
/// grouping rule leaves the context it was found in unchanged.
fn context<'a>(prelude: &'a str, enclosing: &'a str) -> &'a str {
    if prelude.starts_with('@') {
        enclosing
    } else {
        prelude
    }
}

/// The `prelude { body }` pairs at the top level of some rule text.
///
/// A prelude runs from the end of the previous declaration or block, not from the start of
/// the text: CSS nesting lets declarations precede a nested rule, and a scanner that takes
/// everything before the brace swallows them into the prelude and loses both.
fn blocks(text: &str) -> Vec<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut start = 0;
    let mut open = None;
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'{' => {
                depth += 1;
                if depth == 1 {
                    open = Some(index);
                }
            }
            b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let brace = open.take().unwrap_or(index);
                    found.push((text[start..brace].trim(), &text[brace + 1..index]));
                    start = index + 1;
                }
            }
            b';' if depth == 0 => start = index + 1,
            _ => {}
        }
    }
    found
}

/// The text of a block that belongs to no nested block, which is where its own declarations
/// live. Declarations may sit either side of a nested rule, so they are collected rather
/// than sliced.
fn outside_blocks(text: &str) -> String {
    let mut loose = String::new();
    let mut cursor = 0;
    for (prelude, body) in blocks(text) {
        let end = prelude_start(text, prelude);
        loose.push_str(&text[cursor..end]);
        cursor = body_end(text, body);
    }
    loose.push_str(&text[cursor..]);
    loose
}

fn prelude_start(text: &str, prelude: &str) -> usize {
    offset(text, prelude)
}

fn body_end(text: &str, body: &str) -> usize {
    offset(text, body) + body.len() + 1
}

fn offset(text: &str, part: &str) -> usize {
    part.as_ptr() as usize - text.as_ptr() as usize
}

#[cfg(test)]
#[path = "starting_style_tests.rs"]
mod tests;
