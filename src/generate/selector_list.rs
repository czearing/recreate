use std::borrow::Cow;

/// Every character of a selector that is not inside a quoted string, paired with its byte
/// offset and the paren nesting depth it sits at.
///
/// A selector is grammar interleaved with data: quoted attribute values hold arbitrary text
/// that may spell out any punctuation the grammar uses. Each question this module asks — is
/// this comma a list separator, is this colon a pseudo-class, is this paren the one that
/// closes a wrapper — is a question about the grammar, so all of them must skip the data.
/// One scanner answers them the same way, because a selector read two ways is a selector
/// some caller reads wrongly.
///
/// The depth reported for a paren is the depth outside it, so in `:is(.a)` the colon, the
/// name and both parens sit at depth 0 while `.a` sits at depth 1.
fn unquoted(selector: &str) -> impl Iterator<Item = (usize, char, usize)> + '_ {
    let mut depth = 0usize;
    let mut quote = None;
    selector
        .char_indices()
        .filter_map(move |(offset, character)| match (quote, character) {
            (Some(open), _) if character == open => {
                quote = None;
                None
            }
            (Some(_), _) => None,
            (None, '"' | '\'') => {
                quote = Some(character);
                None
            }
            (None, '(') => {
                depth += 1;
                Some((offset, character, depth - 1))
            }
            (None, ')') => {
                depth = depth.saturating_sub(1);
                Some((offset, character, depth))
            }
            (None, _) => Some((offset, character, depth)),
        })
}

/// The members of a selector list.
///
/// A list is separated by top-level commas only. Commas also appear inside a member — in a
/// functional pseudo-class such as `:is(.a, .b)` or `:nth-child(2n of .a, .b)`, and inside a
/// quoted attribute value such as `[title="a,b"]`. Cutting those produces fragments that are
/// themselves valid selectors matching things the member does not, so a naive split can
/// admit a selector the correct split rejects.
pub(super) fn members(selectors: &str) -> impl Iterator<Item = &str> {
    let mut start = 0usize;
    let mut cuts = Vec::new();
    for (offset, _, _) in
        unquoted(selectors).filter(|(_, character, depth)| *character == ',' && *depth == 0)
    {
        cuts.push(&selectors[start..offset]);
        start = offset + 1;
    }
    cuts.push(&selectors[start..]);
    cuts.into_iter()
        .map(str::trim)
        .filter(|member| !member.is_empty())
}

/// The offset of the first colon that belongs to the selector's own grammar, if any.
fn grammatical_colon(selector: &str) -> Option<usize> {
    unquoted(selector)
        .find(|(_, character, _)| *character == ':')
        .map(|(offset, _, _)| offset)
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
/// Anything else introducing a pseudo-class must go. State pseudo-classes such as `:hover`
/// describe a different state, and structural ones such as `:first-child` describe a
/// position; neither survives being rewritten onto a class that is shared by every node with
/// the same computed style, because the rewritten selector no longer carries the test.
///
/// What identifies a pseudo-class is a colon the selector's own grammar owns, as opposed to
/// one a quoted attribute value merely contains: a value is delimited by its quotes, so a
/// colon inside one is data, and `[data-when="09:00"]` says no more than the exact-value test
/// the generated class already encodes. No depth test is needed alongside it — parentheses
/// reach a member only through a functional pseudo-class, whose own colon is at the top level
/// and already refuses the member, so a colon this function sees is always the compound's.
/// The test is asked again after flattening because flattening lifts a wrapper's argument
/// into the compound, which is how `:is(:hover)` is still refused.
pub(super) fn static_member(member: &str) -> Option<Cow<'_, str>> {
    if grammatical_colon(member).is_none() {
        return Some(Cow::Borrowed(member));
    }
    let mut flattened = member.to_string();
    for name in [":where(", ":is("] {
        while let Some(start) = wrapper(&flattened, name) {
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
    grammatical_colon(&flattened)
        .is_none()
        .then_some(Cow::Owned(flattened))
}

/// The offset of the next forgiving wrapper the selector actually opens, as opposed to one
/// its quoted text merely spells.
fn wrapper(selector: &str, name: &str) -> Option<usize> {
    unquoted(selector)
        .find(|(offset, character, _)| *character == ':' && selector[*offset..].starts_with(name))
        .map(|(offset, _, _)| offset)
}

/// The paren closing the wrapper opened just before `open`, ignoring any parenthesis that a
/// quoted attribute value merely spells out.
fn closing_paren(selector: &str, open: usize) -> Option<usize> {
    unquoted(&selector[open..])
        .find(|(_, character, depth)| *character == ')' && *depth == 0)
        .map(|(offset, _, _)| open + offset)
}

/// The members of a list that a generated class can carry, each reduced to its static form.
pub(super) fn static_members(selectors: &str) -> impl Iterator<Item = Cow<'_, str>> {
    members(selectors).filter_map(static_member)
}
