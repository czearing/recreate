use crate::model::StateStyle;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
pub fn append(
    styles: &[StateStyle],
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
    css: &mut String,
) {
    let mut groups = Vec::new();
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
    let mut groups = Vec::new();
    collect(styles, base, assets, &BTreeSet::new(), &mut groups);
    for (overrides, classes) in interactions {
        let overrides = overrides.iter().map(style_key).collect();
        collect(styles, classes, assets, &overrides, &mut groups);
    }
    emit(groups, css);
}

type RuleKey = (String, Option<String>, String);
type StyleKey<'a> = (
    &'a str,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
);

fn collect(
    styles: &[StateStyle],
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
    overrides: &BTreeSet<StyleKey<'_>>,
    groups: &mut Vec<(RuleKey, BTreeSet<String>)>,
) {
    for style in styles {
        if overrides.contains(&style_key(style)) {
            continue;
        }
        let Some(class) = classes.get(&style.target) else {
            continue;
        };
        let declarations = if style.declarations.contains("url(") {
            assets
                .iter()
                .fold(style.declarations.clone(), |text, (url, local)| {
                    text.replace(url, local)
                })
        } else {
            style.declarations.clone()
        };
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
            Some(scope) => format!(
                "{}{} {target}",
                selector(scope),
                style.pseudo.as_deref().unwrap_or_default()
            ),
            None => format!("{target}{}", style.pseudo.as_deref().unwrap_or_default()),
        };
        if let Some((_, selectors)) = groups.iter_mut().find(|(current, _)| current == &key) {
            selectors.insert(selector);
        } else {
            groups.push((key, BTreeSet::from([selector])));
        }
    }
}

fn style_key(style: &StateStyle) -> StyleKey<'_> {
    (
        style.target.as_str(),
        style.scope.as_deref(),
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

fn emit(groups: Vec<(RuleKey, BTreeSet<String>)>, css: &mut String) {
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
