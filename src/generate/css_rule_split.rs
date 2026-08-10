//! Splitting a stylesheet into the units the cascade actually has.

/// Splits a stylesheet into balanced top-level rules.
///
/// The unit has to be the rule, not the line. The emitter writes its own baked rules one
/// per line, which makes a line-based copy look correct, but a definition rule
/// re-serialised from `cssText` and a responsive group both span several lines. Copying a
/// line out of one of those yields an opening brace with no close, which corrupts every
/// rule after it rather than dropping one.
pub(super) fn top_level(css: &str) -> Vec<String> {
    let mut rules = Vec::new();
    let mut depth = 0_usize;
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    for (index, character) in css.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, character) {
            (_, '\\') => escaped = true,
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(character),
            (None, '{') => depth += 1,
            (None, '}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    rules.push(css[start..=index].trim().to_string());
                    start = index + 1;
                }
            }
            // A statement at-rule has no block, so its terminator ends the rule.
            (None, ';') if depth == 0 => {
                rules.push(css[start..=index].trim().to_string());
                start = index + 1;
            }
            _ => {}
        }
    }
    rules.retain(|rule| !rule.is_empty());
    rules
}
