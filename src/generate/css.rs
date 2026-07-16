use super::responsive;
use super::{animations, state_styles};
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
                responsive::base_declarations(node, &base.viewport, assets)
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
    responsive::append(specification, assets, &classes, &mut css);
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
    animations::append(&base.animations, &mut classes, &mut css);
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
    CssOutput {
        css,
        classes,
        interaction_classes,
    }
}

pub(super) fn declarations(styles: &Styles, assets: &BTreeMap<String, String>) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_border_contract_is_captured_and_generated() {
        let mut styles = Styles::new();
        for side in ["top", "right", "bottom", "left"] {
            for (property, value) in [
                ("width", "4px"),
                ("style", "solid"),
                ("color", "rgb(216, 168, 78)"),
            ] {
                let name = format!("border-{side}-{property}");
                assert!(
                    crate::style_contract::contains(&name),
                    "missing capture property {name}"
                );
                styles.insert(name, value.into());
            }
        }
        let css = declarations(&styles, &BTreeMap::new());
        for side in ["top", "right", "bottom", "left"] {
            assert!(css.contains(&format!("border-{side}-width:4px;")));
            assert!(css.contains(&format!("border-{side}-style:solid;")));
            assert!(css.contains(&format!("border-{side}-color:rgb(216, 168, 78);")));
        }
    }

    #[test]
    fn grid_item_contract_is_captured_and_generated() {
        let mut styles = Styles::new();
        for (name, value) in [
            ("grid-column-start", "1"),
            ("grid-column-end", "-1"),
            ("grid-row-start", "auto"),
            ("grid-row-end", "auto"),
            ("justify-self", "start"),
        ] {
            assert!(crate::style_contract::contains(name));
            styles.insert(name.into(), value.into());
        }
        let css = declarations(&styles, &BTreeMap::new());
        for (name, value) in styles {
            assert!(css.contains(&format!("{name}:{value};")));
        }
    }
}
