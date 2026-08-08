use std::collections::HashMap;

/// A rule's position in the cascade layer order, ordered weakest first.
///
/// The leading flag is what makes layers different from specificity. An unlayered
/// declaration outranks a layered one at *every* specificity — that is the whole point of
/// layers, so an author can override a framework without a specificity war — which makes
/// "is this unlayered" the strongest key and sorts unlayered last.
pub type Position = (bool, Vec<usize>);

/// Each dotted prefix of a layer path, outermost first. Nesting is a path rather than a
/// name, and the outer layer's order dominates the inner one's, so `a.b` is positioned by
/// where `a` was first seen and only then by where `a.b` was.
fn prefixes(path: &str) -> impl Iterator<Item = String> + '_ {
    let mut current = String::new();
    path.split('.').map(move |segment| {
        if !current.is_empty() {
            current.push('.');
        }
        current.push_str(segment);
        current.clone()
    })
}

/// The prelude of a block whose text opens with `@layer`, and the text it wraps.
fn strip_layer(text: &str) -> Option<(&str, &str)> {
    let open = text.find('{')?;
    let remainder = text[..open].trim().strip_prefix("@layer")?;
    // `@layers` is not `@layer`, and a comma list is an order statement rather than a
    // block that holds rules.
    if !remainder.is_empty() && !remainder.starts_with(char::is_whitespace) {
        return None;
    }
    if remainder.contains(',') {
        return None;
    }
    let inner = text[open + 1..].trim_end().strip_suffix('}')?;
    Some((remainder.trim(), inner.trim()))
}

/// Splits a rule's text into the dotted layer path that positions it and the rule it
/// wraps. `@layer a{@layer b{.x{}}}` is `.x{}` at `a.b`.
///
/// An anonymous layer contributes an empty segment. It cannot be re-opened by name, so
/// two anonymous layers share a path and therefore a position; that costs their order
/// relative to each other and keeps the far stronger layered-versus-unlayered distinction.
pub fn peel(text: &str) -> (Option<String>, &str) {
    let mut path: Vec<&str> = Vec::new();
    let mut rest = text.trim();
    while let Some((name, inner)) = strip_layer(rest) {
        path.push(name);
        rest = inner;
    }
    ((!path.is_empty()).then(|| path.join(".")), rest)
}

/// The order the page's layers were declared in.
///
/// Layer precedence is the order in which layer *names* are first encountered, not the
/// order of the blocks, because `@layer a, b;` fixes the order up front however late each
/// block appears. So both shapes register a name and only the first registration counts.
pub struct Order {
    first_seen: HashMap<String, usize>,
}

impl Order {
    pub fn new(rules: &[String]) -> Self {
        let mut first_seen = HashMap::new();
        for text in rules {
            for name in declared(text) {
                let next = first_seen.len();
                first_seen.entry(name).or_insert(next);
            }
        }
        Self { first_seen }
    }

    pub fn position(&self, path: Option<&str>) -> Position {
        let Some(path) = path else {
            return (true, Vec::new());
        };
        let ranks = prefixes(path)
            .map(|prefix| self.first_seen.get(&prefix).copied().unwrap_or(usize::MAX))
            .collect();
        (false, ranks)
    }
}

/// The layer names a rule declares, in declaration order.
fn declared(text: &str) -> Vec<String> {
    let text = text.trim();
    if let Some(names) = text
        .strip_prefix("@layer")
        .filter(|remainder| remainder.starts_with(char::is_whitespace))
        .and_then(|remainder| remainder.trim_end().strip_suffix(';'))
    {
        return names
            .split(',')
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();
    }
    peel(text)
        .0
        .map(|path| prefixes(&path).collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "css_layers_tests.rs"]
mod tests;
