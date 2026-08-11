use super::css::{ScopeCache, global_rule, retain};
use super::css_values::{hash, responsive_signatures_for};
use crate::model::{PageState, Specification};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct Output {
    pub css: String,
    pub classes: BTreeMap<String, String>,
}

pub struct Request<'a, T: Fn(&str)> {
    pub specification: &'a Specification,
    pub assets: &'a BTreeMap<String, String>,
    pub prefix: &'a str,
    pub include_interactions: bool,
    pub reuse: Option<(&'a [PageState], &'a BTreeMap<String, String>)>,
    pub cache: Option<&'a mut ScopeCache>,
    pub path_override: Option<&'a HashSet<String>>,
    pub timing: &'a T,
}

pub fn build<T: Fn(&str)>(request: Request<'_, T>) -> Output {
    let Request {
        specification,
        assets,
        prefix,
        include_interactions,
        reuse,
        cache,
        path_override,
        timing,
    } = request;
    let base = &specification.states[0];
    let mut css = String::new();
    if include_interactions {
        for rule in &base.css_rules {
            if let Some(kept) = retain(rule, &global_rule) {
                css.push_str(&super::asset_urls::rewrite(&kept, assets));
                css.push('\n');
            }
        }
    }
    let mut local_cache = ScopeCache::default();
    let cache = cache.unwrap_or(&mut local_cache);
    let mut classes = BTreeMap::new();
    let changed_paths = path_override.cloned().or_else(|| {
        reuse.map(|(baselines, _)| super::css_paths::changed(specification, baselines))
    });
    let contextual_widths = reuse
        .and_then(|(baselines, _)| {
            baselines
                .iter()
                .find(|baseline| baseline.viewport.width == base.viewport.width)
        })
        .map(|baseline| super::css_paths::contextual_widths(base, baseline))
        .unwrap_or_default();
    let authored_rules = super::authored_css::Index::new(&base.css_rules);
    let fluid_heights = super::css_state_helpers::fluid_height_paths(specification);
    if std::env::var_os("RECREATE_TIMING").is_some()
        && let Some(paths) = &changed_paths
    {
        eprintln!("css_{prefix}_changed_paths={}", paths.len());
    }
    let signatures = responsive_signatures_for(specification, changed_paths.as_ref());
    let nodes: HashMap<_, _> = base
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    for node in &base.nodes {
        if node.tag == "#text" {
            continue;
        }
        if changed_paths
            .as_ref()
            .is_some_and(|paths| !paths.contains(&node.path))
            && let Some(class) = reuse.and_then(|(_, classes)| classes.get(&node.path))
        {
            classes.insert(node.path.clone(), class.clone());
            continue;
        }
        let parent = node
            .parent
            .as_deref()
            .and_then(|parent| nodes.get(parent).copied());
        let float = super::css_visual::inferred_float(node, parent);
        let width = contextual_widths
            .contains(&node.path)
            .then_some(node.rect.width);
        let signature = format!(
            "{}|layout:{}|visual-float:{}|contextual-width:{}",
            signatures
                .get(&node.path)
                .map(String::as_str)
                .unwrap_or_default(),
            super::css_layout::role(node, parent, &base.viewport),
            float.unwrap_or_default(),
            width.map(|value| value.to_string()).unwrap_or_default()
        );
        let class = cache
            .signature_classes
            .entry(signature.clone())
            .or_insert_with(|| format!("{prefix}{}", &hash(&signature)[..10]))
            .clone();
        if cache.emitted.insert(class.clone()) {
            let mut declarations = super::responsive::base_declarations_indexed(
                node,
                parent,
                &base.viewport,
                assets,
                &authored_rules,
                fluid_heights.contains(&node.path),
            );
            super::css_base_style::append_indexed(
                node,
                float,
                width,
                &authored_rules,
                &mut declarations,
            );
            if !include_interactions {
                declarations = super::css_visual::important_interaction_paint(&declarations);
            }
            if !declarations.is_empty() {
                css.push_str(&format!(".{class}{{{declarations}}}\n"));
            }
            super::css_pseudo::append(node, &class, assets, &mut css);
        }
        classes.insert(node.path.clone(), class);
    }
    timing("base");
    super::responsive::append_filtered(
        specification,
        assets,
        &classes,
        &mut css,
        changed_paths.as_ref(),
        &fluid_heights,
    );
    append_authored_media(base, &classes, &mut css);
    Output { css, classes }
}

fn append_authored_media(base: &PageState, classes: &BTreeMap<String, String>, css: &mut String) {
    let mut emitted = HashSet::new();
    for node in &base.nodes {
        let Some(class) = classes.get(&node.path) else {
            continue;
        };
        for rule in super::authored_media::rules(node, class, &base.css_rules) {
            if emitted.insert(rule.clone()) {
                css.push_str(&rule);
                css.push('\n');
            }
        }
    }
}
