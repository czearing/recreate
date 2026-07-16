use super::animations;
use crate::model::{Specification, Styles};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

pub struct CssOutput {
    pub css: String,
    pub classes: BTreeMap<String, String>,
    pub interaction_classes: Vec<BTreeMap<String, String>>,
}

pub fn build(specification: &Specification, assets: &BTreeMap<String, String>) -> CssOutput {
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
    for node in &base.nodes {
        if node.tag == "#text" {
            continue;
        }
        let signature = responsive_signature(specification, &node.path);
        let class = signature_classes
            .entry(signature.clone())
            .or_insert_with(|| format!("r{}", &hash(&signature)[..10]))
            .clone();
        if !css.contains(&format!(".{class}{{")) {
            css.push_str(&format!(
                ".{class}{{{}}}\n",
                declarations(&node.style, assets)
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
    append_responsive(specification, assets, &classes, &mut css);
    animations::append(&base.animations, &mut classes, &mut css);
    let mut interaction_classes = Vec::new();
    for interaction in &specification.interactions {
        let interaction_spec = Specification {
            schema_version: specification.schema_version,
            requested_url: specification.requested_url.clone(),
            captured_url: specification.captured_url.clone(),
            states: interaction.states.clone(),
            interactions: Vec::new(),
        };
        let output = build(&interaction_spec, assets);
        css.push_str(&output.css);
        interaction_classes.push(output.classes);
    }
    CssOutput {
        css,
        classes,
        interaction_classes,
    }
}

fn append_responsive(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
    classes: &BTreeMap<String, String>,
    css: &mut String,
) {
    let Some(base) = specification.states.first() else {
        return;
    };
    let base_styles: BTreeMap<_, _> = base
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), &node.style))
        .collect();
    for state in specification.states.iter().skip(1) {
        let mut rules = String::new();
        for node in &state.nodes {
            let (Some(base_style), Some(class)) =
                (base_styles.get(node.path.as_str()), classes.get(&node.path))
            else {
                continue;
            };
            let changed: Styles = node
                .style
                .iter()
                .filter(|(key, value)| base_style.get(*key) != Some(*value))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            if !changed.is_empty() {
                rules.push_str(&format!(".{class}{{{}}}", declarations(&changed, assets)));
            }
        }
        if !rules.is_empty() {
            css.push_str(&format!(
                "@media(max-width:{}px){{{rules}}}\n",
                state.viewport.width
            ));
        }
    }
}

fn declarations(styles: &Styles, assets: &BTreeMap<String, String>) -> String {
    styles
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| {
            let value = assets
                .iter()
                .fold(value.clone(), |text, (url, local)| text.replace(url, local));
            format!("{key}:{value};")
        })
        .collect()
}

fn style_signature(styles: &Styles) -> String {
    styles
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn responsive_signature(specification: &Specification, path: &str) -> String {
    specification
        .states
        .iter()
        .filter_map(|state| state.nodes.iter().find(|node| node.path == path))
        .map(|node| {
            format!(
                "{}|{}|{}",
                style_signature(&node.style),
                pseudo_signature(node.before.as_ref()),
                pseudo_signature(node.after.as_ref())
            )
        })
        .collect::<Vec<_>>()
        .join("||")
}

fn pseudo_signature(pseudo: Option<&crate::model::Pseudo>) -> String {
    pseudo
        .map(|pseudo| format!("{}:{}", pseudo.content, style_signature(&pseudo.style)))
        .unwrap_or_default()
}

fn hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
