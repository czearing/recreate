//! What a generated box needs restated at a viewport it was not measured at.

use super::super::style_delta::changed_styles;
use crate::{generate::css::declarations, model::Pseudo};
use std::collections::BTreeMap;

pub(super) fn append_pseudo_rule(
    suffix: &str,
    base: Option<&Pseudo>,
    current: Option<&Pseudo>,
    assets: &BTreeMap<String, String>,
    rules: &mut Vec<(String, String)>,
) {
    let Some(current) = current else {
        if base.is_some() {
            rules.push((suffix.to_string(), "content:none;".into()));
        }
        return;
    };
    let base_styles = base
        .map(|pseudo| &pseudo.style)
        .cloned()
        .unwrap_or_default();
    let mut changed = changed_styles(&base_styles, &current.style);
    super::super::style_delta::append_reversions(
        &mut changed,
        &super::super::style_delta::declared(&base_styles),
        &current.style,
    );
    // `changed` still carries `content`, so a box whose content varies across viewports
    // declares it twice here — once from the field below and once from the map. Both spell it
    // the same way, so the duplicate is inert. Filtering `content` out of `changed`, as
    // `css_pseudo::declarations` does, would leave the field below as the only source: correct
    // only because that field is now localised.
    let content = if base.is_none_or(|pseudo| pseudo.content != current.content) {
        super::super::css_pseudo::content_declaration(&current.content, assets)
    } else {
        String::new()
    };
    if !content.is_empty() || !changed.is_empty() {
        rules.push((
            suffix.to_string(),
            format!("{content}{}", declarations(&changed, assets)),
        ));
    }
}
