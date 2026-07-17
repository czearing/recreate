use super::responsive;
use super::{animations, interaction_scroll, interactions, startup_overlays, state_styles};
use super::{
    css_layout,
    css_values::{hash, responsive_signature, style_signature},
};
use crate::model::Specification;
#[cfg(test)]
use crate::model::Styles;
use std::collections::{BTreeMap, HashMap};

pub(super) use super::css_values::declarations;

pub struct CssOutput {
    pub css: String,
    pub classes: BTreeMap<String, String>,
    pub interaction_classes: Vec<BTreeMap<String, String>>,
}

pub fn build(specification: &Specification, assets: &BTreeMap<String, String>) -> CssOutput {
    build_scoped(specification, assets, "r", true)
}

fn build_scoped(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
    prefix: &str,
    include_interactions: bool,
) -> CssOutput {
    let Some(base) = specification.states.first() else {
        return CssOutput {
            css: String::new(),
            classes: BTreeMap::new(),
            interaction_classes: Vec::new(),
        };
    };
    let mut css = String::from("*{box-sizing:border-box}\n");
    for rule in &base.css_rules {
        if rule.trim_start().starts_with("@font-face") {
            let rule = assets
                .iter()
                .fold(rule.clone(), |text, (url, local)| text.replace(url, local));
            css.push_str(&rule);
            css.push('\n');
        }
    }
    let mut signature_classes = HashMap::new();
    let mut classes = BTreeMap::new();
    let base_nodes: HashMap<_, _> = base
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    for node in &base.nodes {
        if node.tag == "#text" {
            continue;
        }
        let parent = node
            .parent
            .as_deref()
            .and_then(|parent| base_nodes.get(parent).copied());
        let signature = format!(
            "{}|layout:{}",
            responsive_signature(specification, &node.path),
            css_layout::role(node, parent, &base.viewport)
        );
        let class = signature_classes
            .entry(signature.clone())
            .or_insert_with(|| format!("{prefix}{}", &hash(&signature)[..10]))
            .clone();
        if !css.contains(&format!(".{class}{{")) {
            css.push_str(&format!(
                ".{class}{{{}}}\n",
                responsive::base_declarations(node, parent, &base.viewport, assets,)
            ));
        }
        if let Some(before) = &node.before {
            css.push_str(&format!(
                ".{class}::before{{content:{};{}}}\n",
                before.content,
                declarations(&before.style, assets)
            ));
        }
        if let Some(after) = &node.after {
            css.push_str(&format!(
                ".{class}::after{{content:{};{}}}\n",
                after.content,
                declarations(&after.style, assets)
            ));
        }
        classes.insert(node.path.clone(), class);
    }
    for node in &base.startup_nodes {
        if node.tag == "#text" {
            continue;
        }
        let signature = style_signature(&node.style);
        let class = format!("u{}", &hash(&signature)[..10]);
        if !css.contains(&format!(".{class}{{")) {
            css.push_str(&format!(
                ".{class}{{{}}}\n",
                declarations(&node.style, assets)
            ));
        }
        classes.insert(node.path.clone(), class);
    }
    responsive::append(specification, assets, &classes, &mut css);
    let mut interaction_classes = Vec::new();
    for (index, interaction) in specification.interactions.iter().enumerate() {
        let mut states = interaction.states.clone();
        if !interactions::closable(interaction, &specification.states[0]) {
            for state in &mut states {
                let Some(baseline) = specification
                    .states
                    .iter()
                    .find(|baseline| baseline.viewport.width == state.viewport.width)
                else {
                    continue;
                };
                let Some(owner) = interaction_scroll::owner_path(baseline, state) else {
                    continue;
                };
                if let Some(node) = state.nodes.iter_mut().find(|node| node.path == owner) {
                    node.style.insert("overflow-y".into(), "scroll".into());
                    node.style
                        .insert("scrollbar-gutter".into(), "stable".into());
                }
            }
        }
        let interaction_spec = Specification {
            schema_version: specification.schema_version,
            requested_url: specification.requested_url.clone(),
            captured_url: specification.captured_url.clone(),
            states,
            interactions: Vec::new(),
        };
        let output = build_scoped(
            &interaction_spec,
            assets,
            &format!("s{}-", index + 1),
            false,
        );
        css.push_str(&output.css);
        interaction_classes.push(output.classes);
    }
    animations::append(&base.animations, &mut classes, &mut css);
    startup_overlays::append(&specification.states, &mut classes, &mut css);
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
    if !include_interactions {
        interaction_classes.clear();
    }
    CssOutput {
        css,
        classes,
        interaction_classes,
    }
}

#[cfg(test)]
#[path = "css_tests.rs"]
mod tests;
