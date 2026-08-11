use super::css::global_rule;
use super::css_identifiers::mentions;
use std::collections::BTreeSet;

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
///
/// Carrying is not the same act for every kind of definition, and the difference is where
/// the definition lives rather than what names it. A self-naming one travels as text; one
/// that reaches its user by inheritance travels as a value re-declared on an ancestor the
/// fragment keeps, which `css_inheritance` owns. Both feed the same loop, because a block
/// carried verbatim is text like any other and the names *it* spells are still unmet.
pub(super) fn self_contained(css: &str, classes: &[String]) -> String {
    let rules = super::css_rule_split::top_level(css);
    let mut wanted = BTreeSet::new();
    let mut inherited = String::new();
    // A carried definition can name a further one, so this runs to a fixed point rather
    // than for one pass. Selection only widens as the carried text grows, so each pass
    // takes a superset of the last and a finite rule set forces the text to stop changing.
    let mut carried = String::new();
    loop {
        let declared = rules
            .iter()
            .filter_map(|rule| {
                super::css::retain(rule, &|member| reaches(member, classes, &carried))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let carried_text = join(&inherited, &declared);
        let mut grew = carried_text != carried;
        if super::css_inheritance::wanted(&carried_text, &declared, &mut wanted) {
            inherited = super::css_inheritance::declarations(&rules, &wanted);
            grew = true;
        }
        if !grew {
            return carried_text;
        }
        carried = carried_text;
    }
}

/// Whether one rule — already reduced by `retain` to something that is not a grouping
/// at-rule — reaches the fragment.
///
/// The two kinds arrive by different routes and neither implies the other: a style rule by
/// selecting an element that moved, a definition by having a name it defines mentioned in
/// what has been taken so far. Asking both in one predicate is what lets a single walk
/// serve both, so a group holding a style rule and a definition side by side yields each of
/// them rather than being claimed by whichever was seen first.
fn reaches(rule: &str, classes: &[String], carried: &str) -> bool {
    if global_rule(rule) {
        return defined_names(rule).iter().any(|name| mentions(carried, name));
    }
    let prelude = rule.split('{').next().unwrap_or_default();
    classes
        .iter()
        .any(|class_name| mentions(prelude, &format!(".{class_name}")))
}

/// Inherited values are placed first so a rule the fragment carries for itself, which is a
/// nearer scope, still wins.
fn join(inherited: &str, declared: &str) -> String {
    match inherited.is_empty() {
        true => declared.to_string(),
        false => format!("{inherited}\n{declared}"),
    }
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
    let start = super::css_identifiers::mention_index(body, name)? + name.len();
    let value = body[start..].trim_start().strip_prefix(':')?;
    let end = value.find([';', '}']).unwrap_or(value.len());
    Some(unquote(&value[..end])).filter(|value| !value.is_empty())
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches(['"', '\'']).trim().to_string()
}

#[cfg(test)]
#[path = "css_closure_tests.rs"]
mod tests;
