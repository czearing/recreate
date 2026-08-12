/// The top-level components of a CSS value.
///
/// A component may contain whitespace of its own — `calc(1rem + 2px)` is one component and
/// not three — so this counts nesting rather than cutting at every space. Splitting on
/// whitespace alone tears such a value into pieces that mean something else, which is how a
/// two-component shorthand and a one-component function call become indistinguishable.
pub fn components(value: &str) -> Vec<&str> {
    let mut components = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if character.is_whitespace() && depth == 0 {
            if let Some(from) = start.take() {
                components.push(&value[from..index]);
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(from) = start {
        components.push(&value[from..]);
    }
    components
}

#[cfg(test)]
#[path = "css_value_tests.rs"]
mod css_value_tests;
