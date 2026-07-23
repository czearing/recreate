use super::responsive;
use super::{animations, interactions, startup_overlays, state_styles};
use super::{
    css_layout,
    css_values::{hash, responsive_signatures_for},
};
use crate::model::Specification;
#[cfg(test)]
use crate::model::Styles;
use std::collections::{BTreeMap, HashMap, HashSet};

pub(super) use super::css_values::declarations;

pub struct CssOutput {
    pub css: String,
    pub classes: BTreeMap<String, String>,
    pub interaction_classes: Vec<BTreeMap<String, String>>,
}

#[derive(Default)]
struct ScopeCache {
    signature_classes: HashMap<String, String>,
    emitted: HashSet<String>,
}

fn child_nodes(nodes: &[crate::model::Node]) -> HashMap<&str, Vec<&crate::model::Node>> {
    let mut children = HashMap::new();
    for node in nodes {
        if let Some(parent) = node.parent.as_deref() {
            children.entry(parent).or_insert_with(Vec::new).push(node);
        }
    }
    children
}

fn multiline_text_box(node: &crate::model::Node) -> bool {
    node.style
        .get("line-height")
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|line_height| node.rect.height > line_height * 1.5)
}

fn important_interaction_paint(css: &str) -> String {
    css.split_inclusive(';')
        .map(|declaration| {
            let property = declaration
                .split_once(':')
                .map(|(property, _)| property)
                .unwrap_or_default();
            if (matches!(
                property,
                "background-color"
                    | "border"
                    | "color"
                    | "fill"
                    | "stroke"
                    | "-webkit-text-fill-color"
            ) || property.starts_with("border-"))
                && !declaration.contains("!important")
            {
                format!("{}!important;", declaration.trim_end_matches(';'))
            } else {
                declaration.to_string()
            }
        })
        .collect()
}

fn visual_flex_direction(
    node: &crate::model::Node,
    children: &[&crate::model::Node],
) -> Option<&'static str> {
    if node
        .style
        .get("display")
        .is_none_or(|value| value != "flex")
    {
        return None;
    }
    let direction = node.style.get("flex-direction")?.as_str();
    let first = children
        .iter()
        .find(|child| child.rect.width > 0.0 && child.rect.height > 0.0)?;
    let last = children
        .iter()
        .rev()
        .find(|child| child.rect.width > 0.0 && child.rect.height > 0.0)?;
    match direction {
        "row" if first.rect.x > last.rect.x + 1.0 => Some("row-reverse"),
        "column" if first.rect.y > last.rect.y + 1.0 => Some("column-reverse"),
        _ => None,
    }
}

fn visual_float(
    node: &crate::model::Node,
    parent: Option<&crate::model::Node>,
) -> Option<&'static str> {
    let parent = parent?;
    let missing_float = node.style.get("float").is_none_or(|value| value == "none");
    let right_edge = parent.rect.x + parent.rect.width;
    (missing_float
        && parent
            .style
            .get("display")
            .is_some_and(|value| value == "block")
        && node
            .style
            .get("display")
            .is_some_and(|value| value == "block")
        && node
            .style
            .get("position")
            .is_some_and(|value| value == "static")
        && node.rect.width <= 0.5
        && (node.rect.x - right_edge).abs() <= 1.0)
        .then_some("right")
}

pub fn build(specification: &Specification, assets: &BTreeMap<String, String>) -> CssOutput {
    build_scoped(specification, assets, "r", true, None, None, None)
}

fn build_scoped(
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
    let mut css = String::new();
    for rule in &base.css_rules {
        if global_rule(rule) {
            let rule = rewrite_rule_assets(rule, assets);
            css.push_str(&rule);
            css.push('\n');
        }
    }
    let mut local_cache = ScopeCache::default();
    let cache = cache.unwrap_or(&mut local_cache);
    let mut classes = BTreeMap::new();
    let changed_paths = path_override
        .cloned()
        .or_else(|| reuse.map(|(baselines, _)| changed_paths(specification, baselines)));
    let contextual_widths = reuse
        .and_then(|(baselines, _)| {
            baselines
                .iter()
                .find(|baseline| baseline.viewport.width == base.viewport.width)
        })
        .map(|baseline| contextual_width_paths(base, baseline))
        .unwrap_or_default();
    let fluid_heights = fluid_height_paths(specification);
    if std::env::var_os("RECREATE_TIMING").is_some()
        && let Some(paths) = &changed_paths
    {
        eprintln!("css_{prefix}_changed_paths={}", paths.len());
    }
    let responsive_signatures = responsive_signatures_for(specification, changed_paths.as_ref());
    let base_nodes: HashMap<_, _> = base
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let base_children = child_nodes(&base.nodes);
    let text_parents: HashSet<_> = base
        .nodes
        .iter()
        .filter(|node| node.tag == "#text" && !node.text.trim().is_empty())
        .filter_map(|node| node.parent.clone())
        .filter(|path| {
            base_nodes.get(path.as_str()).is_none_or(|node| {
                !matches!(
                    node.tag.as_str(),
                    "button" | "input" | "select" | "textarea"
                )
            })
        })
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
            .and_then(|parent| base_nodes.get(parent).copied());
        let visual_flex = visual_flex_direction(
            node,
            base_children
                .get(node.path.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default(),
        );
        let visual_float = visual_float(node, parent);
        let contextual_width = contextual_widths
            .contains(&node.path)
            .then_some(node.rect.width);
        let signature = format!(
            "{}|layout:{}|visual-flex:{}|visual-float:{}|contextual-width:{}",
            responsive_signatures
                .get(&node.path)
                .map(String::as_str)
                .unwrap_or_default(),
            css_layout::role(node, parent, &base.viewport),
            visual_flex.unwrap_or_default(),
            visual_float.unwrap_or_default(),
            contextual_width
                .map(|width| width.to_string())
                .unwrap_or_default()
        );
        let class = cache
            .signature_classes
            .entry(signature.clone())
            .or_insert_with(|| format!("{prefix}{}", &hash(&signature)[..10]))
            .clone();
        if cache.emitted.insert(class.clone()) {
            let mut base_css = responsive::base_declarations(
                node,
                parent,
                &base.viewport,
                assets,
                &base.css_rules,
                fluid_heights.contains(&node.path),
                text_parents.contains(&node.path),
            );
            if let Some(direction) = visual_flex {
                base_css.push_str(&format!("flex-direction:{direction};"));
            }
            if let Some(value) = visual_float {
                base_css.push_str(&format!("float:{value};"));
            }
            if let Some(width) = contextual_width {
                base_css.push_str(&format!("width:{width}px;"));
            }
            let line_clamp = node
                .style
                .get("-webkit-line-clamp")
                .is_some_and(|value| value != "none" && value != "0");
            let authored_line_clamp = (!line_clamp && multiline_text_box(node))
                .then(|| {
                    super::authored_css::positive_integer_property(
                        node,
                        &base.css_rules,
                        "-webkit-line-clamp",
                    )
                })
                .flatten();
            if line_clamp
                && (node
                    .style
                    .get("-webkit-box-orient")
                    .is_some_and(|value| value == "vertical")
                    || super::authored_css::has_property(
                        node,
                        &base.css_rules,
                        "-webkit-box-orient",
                    ))
                || authored_line_clamp.is_some()
            {
                base_css.push_str("display:-webkit-box;");
                if let Some(lines) = authored_line_clamp {
                    base_css.push_str(&format!(
                        "-webkit-box-orient:vertical;-webkit-line-clamp:{lines};"
                    ));
                }
            }

            if include_interactions {
                css.push_str(&format!(".{class}{{{base_css}}}\n"));
            } else {
                let base_css = important_interaction_paint(&base_css);
                css.push_str(&format!(".{class}{{{base_css}}}\n"));
            }
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
    timing("base");
    responsive::append_filtered(
        specification,
        assets,
        &classes,
        &mut css,
        changed_paths.as_ref(),
        &fluid_heights,
    );
    let mut authored_media = HashSet::new();
    for node in &base.nodes {
        let Some(class) = classes.get(&node.path) else {
            continue;
        };
        for rule in super::authored_media::rules(node, class, &base.css_rules) {
            if authored_media.insert(rule.clone()) {
                css.push_str(&rule);
                css.push('\n');
            }
        }
    }
    timing("responsive");
    let mut interaction_classes = Vec::new();
    let mut interaction_cache = ScopeCache::default();
    for (index, interaction) in specification.interactions.iter().enumerate() {
        if !interactions::rendered(interaction, &specification.states) {
            interaction_classes.push(BTreeMap::new());
            timing(&format!("interaction_{}", index + 1));
            continue;
        }

        let shared_surface = interactions::shared_trigger(interaction, &specification.states);
        let states = if shared_surface {
            interaction
                .states
                .iter()
                .map(|state| {
                    let baseline = specification
                        .states
                        .iter()
                        .find(|baseline| baseline.viewport.width == state.viewport.width)
                        .unwrap_or(&specification.states[0]);
                    let roots = crate::interaction_surface::roots(state, baseline);
                    with_baseline_css(
                        crate::model::PageState {
                            url: state.url.clone(),
                            title: state.title.clone(),
                            viewport: state.viewport.clone(),
                            nodes: state
                                .nodes
                                .iter()
                                .filter(|node| {
                                    roots.iter().any(|root| {
                                        node.path == *root
                                            || node
                                                .path
                                                .strip_prefix(root)
                                                .is_some_and(|suffix| suffix.starts_with('>'))
                                    })
                                })
                                .cloned()
                                .collect(),
                            dom: state
                                .dom
                                .iter()
                                .filter(|(path, _)| {
                                    roots.iter().any(|root| {
                                        *path == root
                                            || path
                                                .strip_prefix(root)
                                                .is_some_and(|suffix| suffix.starts_with('>'))
                                    })
                                })
                                .map(|(path, node)| (path.clone(), node.clone()))
                                .collect(),
                            capture_blockers: state.capture_blockers.clone(),
                            startup_nodes: Vec::new(),
                            startup_delay_ms: 0,
                            startup_duration_ms: 0,
                            animations: Vec::new(),
                            state_styles: state.state_styles.clone(),
                            attribute_sequences: Vec::new(),
                            css_rules: state.css_rules.clone(),
                            asset_urls: Vec::new(),
                            asset_data: Default::default(),
                        },
                        baseline,
                    )
                })
                .collect()
        } else {
            interaction
                .states
                .iter()
                .map(|state| {
                    let baseline = specification
                        .states
                        .iter()
                        .find(|baseline| baseline.viewport.width == state.viewport.width)
                        .unwrap_or(&specification.states[0]);
                    with_baseline_css(state.clone(), baseline)
                })
                .collect()
        };
        let interaction_spec = Specification {
            schema_version: specification.schema_version,
            requested_url: specification.requested_url.clone(),
            captured_url: specification.captured_url.clone(),
            states,
            interactions: Vec::new(),
        };
        let surface_paths = shared_surface.then(|| {
            crate::interaction_surface::paths(&interaction_spec.states, &specification.states)
        });
        let output = build_scoped(
            &interaction_spec,
            assets,
            "s",
            false,
            Some((&specification.states, &classes)),
            Some(&mut interaction_cache),
            surface_paths.as_ref(),
        );
        css.push_str(&output.css);
        interaction_classes.push(output.classes);
        timing(&format!("interaction_{}", index + 1));
    }

    fn changed_paths(
        specification: &Specification,
        baselines: &[crate::model::PageState],
    ) -> HashSet<String> {
        specification
            .states
            .iter()
            .flat_map(|state| {
                let baseline = baselines
                    .iter()
                    .find(|baseline| baseline.viewport.width == state.viewport.width);
                let nodes: HashMap<_, _> = baseline
                    .into_iter()
                    .flat_map(|baseline| &baseline.nodes)
                    .map(|node| (node.path.as_str(), node))
                    .collect();
                let current_children = child_nodes(&state.nodes);
                let baseline_children = baseline
                    .map(|baseline| child_nodes(&baseline.nodes))
                    .unwrap_or_default();
                let contextual = baseline
                    .map(|baseline| contextual_width_paths(state, baseline))
                    .unwrap_or_default();
                state
                    .nodes
                    .iter()
                    .filter(move |node| {
                        contextual.contains(&node.path)
                            || nodes.get(node.path.as_str()).is_none_or(|baseline| {
                                node.style != baseline.style
                                    || node.before != baseline.before
                                    || node.after != baseline.after
                                    || visual_flex_direction(
                                        node,
                                        current_children
                                            .get(node.path.as_str())
                                            .map(Vec::as_slice)
                                            .unwrap_or_default(),
                                    ) != visual_flex_direction(
                                        baseline,
                                        baseline_children
                                            .get(node.path.as_str())
                                            .map(Vec::as_slice)
                                            .unwrap_or_default(),
                                    )
                            })
                    })
                    .map(|node| node.path.clone())
            })
            .collect()
    }
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
    append_custom_property_fallbacks_for_spec(specification, &base.css_rules, &mut css);
    super::custom_properties::append_responsive(&specification.states, &mut css);
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

fn topology_changed_paths(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> HashSet<String> {
    fn children(state: &crate::model::PageState) -> HashMap<&str, HashSet<&str>> {
        let mut children = HashMap::<_, HashSet<_>>::new();
        for node in &state.nodes {
            if let Some(parent) = node.parent.as_deref() {
                children
                    .entry(parent)
                    .or_default()
                    .insert(node.path.as_str());
            }
        }
        children
    }

    let current = children(state);
    let captured = children(baseline);
    let baseline_nodes = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<HashMap<_, _>>();
    let changed_parents = state
        .nodes
        .iter()
        .filter_map(|node| {
            let parent = node.parent.as_deref()?;
            let changed = baseline_nodes
                .get(node.path.as_str())
                .is_none_or(|baseline| {
                    node.rect.width != baseline.rect.width
                        || node.rect.height != baseline.rect.height
                        || node.style != baseline.style
                });
            (changed || current.get(parent) != captured.get(parent)).then_some(parent)
        })
        .collect::<HashSet<_>>();
    state
        .nodes
        .iter()
        .filter(|node| {
            node.parent
                .as_deref()
                .is_some_and(|parent| changed_parents.contains(parent))
        })
        .map(|node| node.path.clone())
        .collect()
}

fn contextual_width_paths(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> HashSet<String> {
    let baseline_nodes = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<HashMap<_, _>>();
    topology_changed_paths(state, baseline)
        .into_iter()
        .filter(|path| {
            let Some(node) = state.nodes.iter().find(|node| node.path == *path) else {
                return false;
            };
            baseline_nodes.get(path.as_str()).is_some_and(|original| {
                let mut generated = original.style.clone();
                super::authored_css::normalize(&mut generated, original, &baseline.css_rules);
                let parent = original
                    .parent
                    .as_deref()
                    .and_then(|parent| baseline_nodes.get(parent).copied());
                super::inherited_styles::normalize(
                    &mut generated,
                    original,
                    parent,
                    &baseline.css_rules,
                );
                super::responsive_geometry::normalize(
                    &mut generated,
                    original,
                    parent,
                    &baseline.viewport,
                    None,
                );
                let relative_width = generated
                    .get("width")
                    .is_some_and(|width| width.ends_with('%'));
                node.rect.width == original.rect.width
                    && node.rect.height == original.rect.height
                    && node.style == original.style
                    && node.rect.x != original.rect.x
                    && relative_width
            })
        })
        .collect()
}

fn with_baseline_css(
    mut state: crate::model::PageState,
    baseline: &crate::model::PageState,
) -> crate::model::PageState {
    let mut rules = baseline.css_rules.clone();
    for rule in std::mem::take(&mut state.css_rules) {
        if !rules.contains(&rule) {
            rules.push(rule);
        }
    }
    state.css_rules = rules;
    state
}

fn fluid_height_paths(specification: &Specification) -> HashSet<String> {
    let mut heights = HashMap::<String, Vec<f64>>::new();
    let mut authored = HashSet::new();
    for state in &specification.states {
        for node in &state.nodes {
            heights
                .entry(node.path.clone())
                .or_default()
                .push(node.rect.height);
            if super::authored_css::has_property(node, &state.css_rules, "height") {
                authored.insert(node.path.clone());
            }
        }
    }
    heights
        .into_iter()
        .filter(|(path, values)| {
            !authored.contains(path)
                && !specification.states.iter().any(|state| {
                    state
                        .nodes
                        .iter()
                        .find(|node| &node.path == path)
                        .is_some_and(|node| {
                            node.style
                                .get("overflow")
                                .is_some_and(|value| value == "hidden")
                                || node
                                    .style
                                    .get("overflow-y")
                                    .is_some_and(|value| value == "hidden")
                                || node.style.contains_key("-webkit-line-clamp")
                        })
                })
                && values
                    .iter()
                    .skip(1)
                    .any(|value| (value - values[0]).abs() > 1.0)
        })
        .map(|(path, _)| path)
        .collect()
}

#[cfg(test)]
fn append_custom_property_fallbacks(rules: &[String], css: &mut String) {
    let references = custom_property_references(css);
    append_custom_property_values(rules, references, css);
}

fn append_custom_property_fallbacks_for_spec(
    specification: &Specification,
    rules: &[String],
    css: &mut String,
) {
    let mut references = custom_property_references(css);
    for state in &specification.states {
        for node in state.nodes.iter().chain(&state.startup_nodes) {
            for value in node.attributes.values() {
                references.extend(custom_property_references(value));
            }
        }
    }
    for interaction in &specification.interactions {
        for state in &interaction.states {
            for node in state.nodes.iter().chain(&state.startup_nodes) {
                for value in node.attributes.values() {
                    references.extend(custom_property_references(value));
                }
            }
        }
    }
    append_custom_property_values(rules, references, css);
}

fn append_custom_property_values(
    rules: &[String],
    references: std::collections::BTreeSet<String>,
    css: &mut String,
) {
    let mut declarations = String::new();
    for name in references {
        let values = rules
            .iter()
            .filter_map(|rule| custom_property_value(rule, &name))
            .collect::<std::collections::BTreeSet<_>>();
        if values.len() == 1 {
            declarations.push_str(&format!("{name}:{};", values.into_iter().next().unwrap()));
        }
    }
    if !declarations.is_empty() {
        css.push_str(&format!(":root{{{declarations}}}\n"));
    }
}

fn custom_property_references(css: &str) -> std::collections::BTreeSet<String> {
    let mut references = std::collections::BTreeSet::new();
    let mut remaining = css;
    while let Some(index) = remaining.find("var(--") {
        remaining = &remaining[index + 4..];
        let end = remaining
            .find([',', ')', ' ', '\t'])
            .unwrap_or(remaining.len());
        references.insert(remaining[..end].to_string());
        remaining = &remaining[end..];
    }
    references
}

fn custom_property_value(rule: &str, name: &str) -> Option<String> {
    let mut remaining = rule;
    while let Some(index) = remaining.find(name) {
        remaining = &remaining[index + name.len()..];
        let candidate = remaining.trim_start();
        if let Some(value) = candidate.strip_prefix(':') {
            let end = value.find([';', '}']).unwrap_or(value.len());
            return Some(value[..end].trim().to_string());
        }
    }
    None
}

pub(super) fn global_rule(rule: &str) -> bool {
    let rule = rule.trim_start();
    rule.starts_with("@font-face")
        || rule.starts_with("@keyframes")
        || rule.starts_with("@-webkit-keyframes")
}

fn rewrite_rule_assets(rule: &str, assets: &BTreeMap<String, String>) -> String {
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
