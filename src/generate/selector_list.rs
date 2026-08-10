use std::borrow::Cow;

/// The members of a selector list.
///
/// A list is separated by top-level commas only. Commas also appear inside a member — in a
/// functional pseudo-class such as `:is(.a, .b)` or `:nth-child(2n of .a, .b)`, and inside a
/// quoted attribute value such as `[title="a,b"]`. Cutting those produces fragments that are
/// themselves valid selectors matching things the member does not, so a naive split can
/// admit a selector the correct split rejects.
pub(super) fn members(selectors: &str) -> impl Iterator<Item = &str> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut start = 0usize;
    let mut cuts = Vec::new();
    for (offset, character) in selectors.char_indices() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(character),
            (None, '(') => depth += 1,
            (None, ')') => depth = depth.saturating_sub(1),
            (None, ',') if depth == 0 => {
                cuts.push(&selectors[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    cuts.push(&selectors[start..]);
    cuts.into_iter()
        .map(str::trim)
        .filter(|member| !member.is_empty())
}

/// One member reduced to the compound facts a generated class encodes — tag, id, classes and
/// attributes — or `None` when the member says more than that.
///
/// `:where()` and `:is()` say nothing about state: they match on structure alone, so a rule
/// using one applies in the base state exactly like the compound it wraps. Dropping them
/// loses real declarations — Fluent defines `--component-card-padding` only on
/// `.root:where(.size-medium)`, and without it every card that sizes its padding from that
/// variable collapses to zero padding.
///
/// Anything else carrying a colon must go. State pseudo-classes such as `:hover` describe a
/// different state, and structural ones such as `:first-child` describe a position; neither
/// survives being rewritten onto a class that is shared by every node with the same computed
/// style, because the rewritten selector no longer carries the test.
pub(super) fn static_member(member: &str) -> Option<Cow<'_, str>> {
    if !member.contains(':') {
        return Some(Cow::Borrowed(member));
    }
    let mut flattened = member.to_string();
    for name in [":where(", ":is("] {
        while let Some(start) = flattened.find(name) {
            let open = start + name.len();
            let end = closing_paren(&flattened, open)?;
            let inner = flattened[open..end].trim().to_string();
            // A list inside the wrapper would have to expand into several selectors to stay
            // correct, so the member is left alone rather than narrowed to one branch.
            if members(&inner).count() > 1 {
                return None;
            }
            flattened.replace_range(start..=end, &inner);
        }
    }
    (!flattened.contains(':')).then_some(Cow::Owned(flattened))
}

fn closing_paren(selector: &str, open: usize) -> Option<usize> {
    let mut depth = 1usize;
    for (offset, character) in selector[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// The members of a list that a generated class can carry, each reduced to its static form.
pub(super) fn static_members(selectors: &str) -> impl Iterator<Item = Cow<'_, str>> {
    members(selectors).filter_map(static_member)
}
