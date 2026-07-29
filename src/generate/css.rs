use super::{animations, startup_overlays, state_styles};
use crate::model::Specification;
#[cfg(test)]
use crate::model::Styles;
use std::collections::{BTreeMap, HashMap, HashSet};

#[cfg(test)]
use super::css_custom_properties::append as append_custom_property_fallbacks;
#[cfg(test)]
use super::css_paths::topology as topology_changed_paths;
#[cfg(test)]
use super::css_state_helpers::{fluid_height_paths, with_baseline_css};
pub(super) use super::css_values::declarations;
#[cfg(test)]
use super::css_visual::{important_interaction_paint, inferred_float as visual_float};

pub struct CssOutput {
    pub css: String,
    pub classes: BTreeMap<String, String>,
    pub interaction_classes: Vec<BTreeMap<String, String>>,
}

#[derive(Default)]
pub(super) struct ScopeCache {
    pub signature_classes: HashMap<String, String>,
    pub emitted: HashSet<String>,
}

pub fn build(specification: &Specification, assets: &BTreeMap<String, String>) -> CssOutput {
    build_scoped(specification, assets, "r", true, None, None, None)
}

pub(super) fn build_scoped(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
    prefix: &str,
    include_interactions: bool,
    reuse: Option<(&[crate::model::PageState], &BTreeMap<String, String>)>,
    cache: Option<&mut ScopeCache>,
    path_override: Option<&HashSet<String>>,
) -> CssOutput {
    let started = std::time::Instant::now();
    let timing = |phase: &str| {
        if std::env::var_os("RECREATE_TIMING").is_some() && include_interactions {
            eprintln!("css_{phase}={:.3}s", started.elapsed().as_secs_f64());
        }
    };
    let Some(base) = specification.states.first() else {
        return CssOutput {
            css: String::new(),
            classes: BTreeMap::new(),
            interaction_classes: Vec::new(),
        };
    };
    let output = super::css_base::build(super::css_base::Request {
        specification,
        assets,
        prefix,
        include_interactions,
        reuse,
        cache,
        path_override,
        timing: &timing,
    });
    let mut css = output.css;
    let mut classes = output.classes;
    timing("responsive");
    let mut interaction_classes =
        super::css_interactions::append(specification, assets, &classes, &mut css, &timing);
    animations::append(&base.animations, &mut classes, &mut css);
    startup_overlays::append(&specification.states, &mut css);
    let inherited = specification
        .interactions
        .iter()
        .zip(&interaction_classes)
        .map(|(interaction, classes)| {
            (
                interaction
                    .states
                    .first()
                    .map(|state| state.state_styles.as_slice())
                    .unwrap_or_default(),
                classes,
            )
        })
        .collect::<Vec<_>>();
    state_styles::append_inherited(&base.state_styles, &classes, &inherited, assets, &mut css);
    super::css_custom_properties::append_for_spec(specification, &base.css_rules, &mut css);
    super::custom_properties::append_responsive(&specification.states, &classes, &mut css);
    timing("states");
    if !include_interactions {
        interaction_classes.clear();
    }
    CssOutput {
        css,
        classes,
        interaction_classes,
    }
}

pub(super) fn global_rule(rule: &str) -> bool {
    let rule = rule.trim_start();
    rule.starts_with("@font-face")
        || rule.starts_with("@keyframes")
        || rule.starts_with("@-webkit-keyframes")
}

pub(super) fn rewrite_rule_assets(rule: &str, assets: &BTreeMap<String, String>) -> String {
    let mut replacements: Vec<_> = assets.iter().collect();
    replacements.sort_by_key(|(url, _)| std::cmp::Reverse(url.len()));
    replacements
        .into_iter()
        .fold(rule.to_string(), |text, (url, local)| {
            let text = text.replace(url, local);
            url.strip_prefix("https:")
                .map_or(text.clone(), |relative| text.replace(relative, local))
        })
}

#[cfg(test)]
#[path = "css_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "css_tests_part_2.rs"]
mod tests_part_2;
