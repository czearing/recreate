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
    animations::append(
        &base.animations,
        &super::animation_keyframes::authored_names(&base.css_rules),
        &mut classes,
        &mut css,
    );
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
    let declared = super::custom_properties::declared_names(&specification.states);
    super::css_custom_properties::append_for_spec(
        specification,
        &base.css_rules,
        &declared,
        &mut css,
    );
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

/// The at-rules whose block holds other rules rather than a definition — the CSSOM
/// `CSSGroupingRule` interface, enumerated in one place. Everything one of these
/// contributes reaches an element through a selector.
const GROUPING_AT_RULES: &[&str] = &[
    "media",
    "supports",
    "container",
    "layer",
    "scope",
    "starting-style",
];

/// Whether an authored rule must be re-emitted verbatim because no baked computed style
/// can stand in for it.
///
/// The tool bakes each element's computed style into a hashed class, so any rule that
/// reaches an element through a selector is already represented and re-emitting it would
/// apply it a second time. That covers style rules and the grouping at-rules whose bodies
/// are style rules. What a computed style cannot carry is an at-rule that *defines* a
/// name — a font, a set of keyframes, a custom property registration, a counter style —
/// because the computed style holds only the name and the definition lives nowhere else.
///
/// Stated as a rejection rather than a list of names, so a definition at-rule this tool
/// has never seen survives instead of vanishing. The three rejected shapes are the only
/// ones re-emission can harm: a style rule and a grouping rule double-apply, and a
/// statement at-rule carries placement rather than definition — `@charset` must be the
/// first byte, `@namespace` must precede every style rule, and `@import` names a sheet the
/// capture already walked and baked, so re-emitting it refetches and double-applies.
pub(super) fn global_rule(rule: &str) -> bool {
    // Layer membership is a wrapper the capture rebuilds around every rule it records, so
    // the decision is about the rule inside it. The wrapper itself stays on the emitted
    // text, because it is the rule's cascade position.
    let (_, rule) = super::css_layers::peel(rule);
    let Some(prelude) = rule.strip_prefix('@') else {
        return false;
    };
    // A statement at-rule has no block. Testing the closing brace rather than the opening
    // one keeps `@import url("data:text/css,a{}")` a statement.
    if !rule.ends_with('}') {
        return false;
    }
    let name = prelude
        .split(|character: char| character.is_whitespace() || character == '(' || character == '{')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    !GROUPING_AT_RULES.contains(&name.as_str())
}

#[cfg(test)]
#[path = "css_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "css_tests_part_2.rs"]
mod tests_part_2;
