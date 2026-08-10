use super::css::global_rule;

/// Builds the stylesheet a fragment needs to render on its own, once it has been moved
/// into a document the page's CSS cannot reach.
///
/// Relocation is survivable because the emitter bakes each element's computed style, so
/// every value an ancestor supplied by inheritance is already written onto the element
/// itself. What baking cannot resolve is a value that is an *identifier standing for a
/// definition elsewhere*: `getComputedStyle` reports `animation-name` as a bare name,
/// `font-family` as a stack, a custom property as a raw token. The referent is looked up
/// later, by name, in whichever document is rendering — so moving the fragment moves the
/// pointer and leaves the target behind.
///
/// Selecting rules that mention one of the fragment's classes cannot work, in either unit.
/// A definition rule is named after *itself* — `@keyframes` selectors are percentages and
/// its block holds no class token — so the set of definitions such a filter can select is
/// empty for every fragment, always.
///
/// The selection is inverted instead: take the rules that reach the fragment, then ask of
/// each definition whether the name it defines is mentioned by what has been taken so far,
/// and repeat until nothing new is added. That needs no table of referencing properties —
/// such a table is a per-property branch list already missing recent additions — and it
/// covers fonts, counter styles and palettes at no extra cost. The loose direction is the
/// safe one: a definition nothing names is inert, while a name with no definition silently
/// renders nothing.
pub(super) fn self_contained(css: &str, classes: &[String]) -> String {
    let rules = super::css_rule_split::top_level(css);
    let mut carried = rules
        .iter()
        .map(|rule| consumer(rule, classes))
        .collect::<Vec<_>>();
    // A carried definition can name a further one, so this runs to a fixed point rather
    // than for one pass. It terminates because every pass either carries a rule that was
    // not carried before or stops.
    loop {
        let carried_text = joined(&carried);
        let mut grew = false;
        for (index, rule) in rules.iter().enumerate() {
            if carried[index].is_some() || !global_rule(rule) {
                continue;
            }
            if defined_names(rule)
                .iter()
                .any(|name| mentions(&carried_text, name))
            {
                carried[index] = Some(rule.clone());
                grew = true;
            }
        }
        if !grew {
            return carried_text;
        }
    }
}

fn joined(carried: &[Option<String>]) -> String {
    carried
        .iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The rule as it should be carried, if it reaches the fragment at all.
///
/// A grouping at-rule is descended into and rebuilt around only the members that reach the
/// fragment, preserving the group's condition without dragging in rules for elements that
/// stayed behind. Definitions are not selected here: they reach the fragment by being
/// named, not by matching it.
fn consumer(rule: &str, classes: &[String]) -> Option<String> {
    let body_start = rule.find('{')?;
    if !rule.ends_with('}') {
        return None;
    }
    let prelude = &rule[..body_start];
    if !prelude.trim_start().starts_with('@') {
        return classes
            .iter()
            .any(|class_name| mentions(prelude, &format!(".{class_name}")))
            .then(|| rule.to_string());
    }
    if global_rule(rule) {
        return None;
    }
    let members = super::css_rule_split::top_level(&rule[body_start + 1..rule.len() - 1])
        .iter()
        .filter_map(|member| consumer(member, classes))
        .collect::<Vec<_>>();
    (!members.is_empty()).then(|| format!("{prelude}{{{}}}", members.join("\n")))
}

/// The names a definition rule introduces.
///
/// Almost every definition at-rule spells its name in the prelude. The exception needs no
/// naming: an at-rule that has no prelude identifier declares its name through a
/// `font-family` descriptor instead, which is what `@font-face` does.
fn defined_names(rule: &str) -> Vec<String> {
    let (_, rule) = super::css_layers::peel(rule);
    let Some(prelude) = rule.strip_prefix('@') else {
        return Vec::new();
    };
    let Some(body_start) = prelude.find('{') else {
        return Vec::new();
    };
    let named = prelude[..body_start]
        .split_once(char::is_whitespace)
        .map(|(_, name)| name.trim())
        .unwrap_or_default();
    if !named.is_empty() {
        return named
            .split(',')
            .map(unquote)
            .filter(|n| !n.is_empty())
            .collect();
    }
    descriptor(&prelude[body_start..], "font-family")
        .into_iter()
        .collect()
}

fn descriptor(body: &str, name: &str) -> Option<String> {
    let start = mention_index(body, name)? + name.len();
    let value = body[start..].trim_start().strip_prefix(':')?;
    let end = value.find([';', '}']).unwrap_or(value.len());
    Some(unquote(&value[..end])).filter(|value| !value.is_empty())
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches(['"', '\'']).trim().to_string()
}

/// Whether `name` occurs as a whole identifier rather than inside a longer one, so that
/// `spin` is not read out of `arcspin`.
fn mentions(text: &str, name: &str) -> bool {
    mention_index(text, name).is_some()
}

fn mention_index(text: &str, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let mut from = 0;
    while let Some(relative) = text[from..].find(name) {
        let start = from + relative;
        let end = start + name.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if !before.is_some_and(identifier_character) && !after.is_some_and(identifier_character) {
            return Some(start);
        }
        from = end;
    }
    None
}

fn identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '-' || character == '_'
}
