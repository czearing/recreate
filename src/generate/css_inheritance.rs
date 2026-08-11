use super::css::global_rule;
use super::css_identifiers::{declared_value, references};
use std::collections::BTreeSet;

/// Carries a value that reached a fragment by inheritance into a document where the scope
/// that declared it no longer exists.
///
/// A definition that names itself — `@keyframes`, `@font-face` — travels by copying its
/// text, because the relocated document looks it up by that same name. A custom property
/// does not. It reaches its user by inheritance, so copying its declaring rule carries a
/// selector matching nothing in the new document: the token is present, nothing inherits
/// it, and no paint changes. What has to travel is the *value*, re-declared on an ancestor
/// the fragment still has. Once the fragment is its own file its outermost element is that
/// document's root, so `:root` is the ancestor every element in it inherits from.
///
/// Only a declaration the fragment certainly inherits may be moved: one made on the
/// document root, an ancestor of every element. A name overridden on some intermediate
/// ancestor is indistinguishable, in text alone, from one that is not, so it is left
/// undeclared rather than guessed at — a wrong colour is worse than the absent one the
/// reference already produces.
pub(super) fn declarations(rules: &[String], wanted: &BTreeSet<String>) -> String {
    if wanted.is_empty() {
        return String::new();
    }
    rules
        .iter()
        .filter_map(|rule| root_declarations(rule, wanted))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The wanted names `rule` declares on the document root, re-declared as a `:root` rule.
///
/// A grouping at-rule is descended into and rebuilt around its own root declarations, so a
/// value stated only under a condition keeps that condition instead of becoming
/// unconditional.
fn root_declarations(rule: &str, wanted: &BTreeSet<String>) -> Option<String> {
    let body_start = rule.find('{')?;
    if !rule.ends_with('}') {
        return None;
    }
    let prelude = &rule[..body_start];
    if prelude.trim_start().starts_with('@') {
        if global_rule(rule) {
            return None;
        }
        let members = super::css_rule_split::top_level(&rule[body_start + 1..rule.len() - 1])
            .iter()
            .filter_map(|member| root_declarations(member, wanted))
            .collect::<Vec<_>>();
        return (!members.is_empty()).then(|| format!("{prelude}{{{}}}", members.join("\n")));
    }
    if !document_root(prelude) {
        return None;
    }
    let declarations = wanted
        .iter()
        .filter_map(|name| {
            let value = declared_value(&rule[body_start..], name)?;
            // An empty custom property is not a value: `var()` reading it substitutes
            // nothing, so the declaration is invalid at computed-value time and the
            // property silently takes its initial value instead of the authored one.
            (!value.is_empty()).then(|| format!("{name}:{value};"))
        })
        .collect::<String>();
    (!declarations.is_empty()).then(|| format!(":root{{{declarations}}}"))
}

/// Whether `prelude` selects the document root, which is the one scope a relocated
/// fragment is certain to still inherit from.
fn document_root(prelude: &str) -> bool {
    prelude
        .split(',')
        .any(|selector| matches!(selector.trim(), ":root" | "html"))
}

/// The names `carried` reads and no carried rule declares, added to `wanted`.
///
/// The set only grows. A re-declared value can itself read a further name, and that name
/// is then wanted even though the previous pass' own output declares it — recomputing the
/// set from scratch would drop the first name as satisfied, undeclare it, and want it
/// again next pass, which never settles.
pub(super) fn wanted(carried: &str, declared: &str, wanted: &mut BTreeSet<String>) -> bool {
    let mut grew = false;
    for name in references(carried) {
        if declared_value(declared, &name).is_none() {
            grew |= wanted.insert(name);
        }
    }
    grew
}
