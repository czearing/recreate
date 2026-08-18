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
    let scoped_compounds = output.scoped_compounds;
    timing("responsive");
    let mut interaction_classes =
        super::css_interactions::append(specification, assets, &classes, &mut css, &timing);
    let authored = super::animation_keyframes::authored_names(&css);
    animations::append(
        &base.animations,
        &authored,
        &super::before_change::BeforeChange::new(&base.css_rules, &base.nodes)
            .with_entry_motion(&base.nodes, &base.animations),
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
    super::selector_marker::apply(&scoped_compounds, &base.nodes, prefix, &mut classes);
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
/// statement at-rule carries placement rather than definition — `@charset` must be the first
/// byte, `@namespace` must precede every style rule, and `@import` names a sheet the walk
/// itself enters via `CSSImportRule.styleSheet`, so re-emitting it refetches and double-applies.
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

/// The parts of a rule worth keeping, with every grouping at-rule rebuilt around whichever
/// of its members survive.
///
/// `global_rule` answers a question about one rule's kind, which is the whole answer only at
/// the top level. A grouping at-rule is neither a definition nor a style rule: what it
/// contributes depends on what it holds, and CSS Conditional Rules 3 allows `@font-face` and
/// `@keyframes` inside every conditional group. Asking `keep` of the group therefore answers
/// the wrong question — its members must be asked, one at a time, at whatever depth they sit.
///
/// This is the only walk that knows a definition can be nested, so every stage needing that
/// knowledge arrives through here rather than re-deriving it — `keep` is stateful so a
/// caller can *learn* what a stylesheet holds as it answers, rather than write a second
/// descent that will drift. This owns where to look; the caller owns only what counts.
///
/// The prelude is rebuilt rather than dropped because a condition is meaning: a definition
/// lifted out of `@media (prefers-reduced-motion: no-preference)` animates a page the author
/// kept still, which is worse than omitting it. A group nothing survives publishes nothing.
pub(super) fn retain(rule: &str, keep: &mut dyn FnMut(&str) -> bool) -> Option<String> {
    let body_start = rule.find('{')?;
    let prelude = &rule[..body_start];
    if !rule.ends_with('}') || !prelude.trim_start().starts_with('@') || global_rule(rule) {
        return keep(rule).then(|| rule.to_string());
    }
    let members = super::css_rule_split::top_level(&rule[body_start + 1..rule.len() - 1])
        .iter()
        .filter_map(|member| retain(member, &mut *keep))
        .collect::<Vec<_>>();
    (!members.is_empty()).then(|| format!("{prelude}{{{}}}", members.join("\n")))
}

#[cfg(test)]
#[path = "css_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "css_tests_part_2.rs"]
mod tests_part_2;
