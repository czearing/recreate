use super::css_rule_groups::Groups;
use crate::model::{Relation, StateStyle};
use std::collections::{BTreeMap, BTreeSet};

pub fn append(
    styles: &[StateStyle],
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
    css: &mut String,
) {
    let mut groups = Groups::default();
    collect(styles, classes, assets, &BTreeSet::new(), &mut groups);
    emit(groups, css);
}

pub fn append_inherited(
    styles: &[StateStyle],
    base: &BTreeMap<String, String>,
    interactions: &[(&[StateStyle], &BTreeMap<String, String>)],
    assets: &BTreeMap<String, String>,
    css: &mut String,
) {
    let mut groups = Groups::default();
    collect(styles, base, assets, &BTreeSet::new(), &mut groups);
    for (overrides, classes) in interactions {
        let overrides = overrides.iter().map(style_key).collect();
        collect(styles, classes, assets, &overrides, &mut groups);
    }
    emit(groups, css);
}

type StyleKey<'a> = (
    &'a str,
    Option<&'a str>,
    Relation,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
);

fn collect(
    styles: &[StateStyle],
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
    overrides: &BTreeSet<StyleKey<'_>>,
    groups: &mut Groups,
) {
    for style in styles {
        if overrides.contains(&style_key(style)) {
            continue;
        }
        let Some(class) = classes.get(&style.target) else {
            continue;
        };
        let declarations = super::asset_urls::rewrite(&style.declarations, assets);
        let key = (
            style.pseudo.clone().unwrap_or_default(),
            style.media.clone(),
            declarations,
        );
        let target = format!(
            "{}{}",
            selector(class),
            style.target_pseudo.as_deref().unwrap_or_default()
        );
        let selector = match style.scope.as_deref().and_then(|scope| classes.get(scope)) {
            // The state and the element it styles are two elements, joined the way the author
            // joined them. Which combinator that was is the record's business; spelling it is
            // the relation's.
            Some(scope) => style.relation.join(
                &format!(
                    "{}{}",
                    selector(scope),
                    style.pseudo.as_deref().unwrap_or_default()
                ),
                &target,
            ),
            None => format!("{target}{}", style.pseudo.as_deref().unwrap_or_default()),
        };
        groups.add(key, selector);
    }
}

fn style_key(style: &StateStyle) -> StyleKey<'_> {
    (
        style.target.as_str(),
        style.scope.as_deref(),
        style.relation,
        style.pseudo.as_deref(),
        style.target_pseudo.as_deref(),
        style.media.as_deref(),
    )
}

fn selector(class: &str) -> String {
    class
        .split_whitespace()
        .map(|name| format!(".{name}"))
        .collect()
}

fn emit(groups: Groups, css: &mut String) {
    for ((_, media, declarations), selectors) in groups {
        let rule = format!(
            "{}{{{declarations}}}",
            selectors.into_iter().collect::<Vec<_>>().join(",")
        );
        match media {
            Some(media) => css.push_str(&format!("@media {media}{{{rule}}}\n")),
            None => css.push_str(&format!("{rule}\n")),
        }
    }
}

#[cfg(test)]
#[path = "state_style_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "state_style_relation_tests.rs"]
mod relation_tests;

#[cfg(test)]
#[path = "state_order_tests.rs"]
mod order_tests;
