#[cfg(test)]
mod animation_order_tests;
mod animation_timing;
mod animations;
mod assets;
mod assets_remote;
mod attribute_sequences;
#[cfg(test)]
mod authenticated_interaction_runtime_tests;
mod authored_css;
mod css;
mod css_layout;
mod css_values;
#[cfg(test)]
mod interaction_geometry_support;
mod interaction_labels;
#[cfg(test)]
mod interaction_runtime_support;
#[cfg(test)]
mod interaction_runtime_tests;
mod interaction_scroll;
mod interactions;
mod jsx;
mod jsx_attrs;
mod jsx_states;
mod jsx_variants;
#[cfg(test)]
#[path = "mount_tests.rs"]
mod mount_tests;
mod names;
mod responsive;
mod responsive_geometry;
mod responsive_height;
#[cfg(test)]
mod responsive_runtime_support;
#[cfg(test)]
mod responsive_runtime_tests;
mod roots;
mod startup_overlays;
mod state_style_maps;
mod state_styles;
mod structural_css;
#[cfg(test)]
mod structural_runtime_support;
#[cfg(test)]
mod structural_runtime_tests;
#[cfg(test)]
mod structural_tests;
mod structural_tree;
#[cfg(test)]
mod tests;
mod tree;

use crate::model::{BrowserCookie, Specification};
use anyhow::Result;
use std::{fs, path::Path};

pub async fn from_file(spec: &Path, out: &Path) -> Result<()> {
    let specification: Specification = serde_json::from_slice(&fs::read(spec)?)?;
    write_project(&specification, out, &[]).await
}

pub async fn write_project(
    specification: &Specification,
    out: &Path,
    cookies: &[BrowserCookie],
) -> Result<()> {
    let root = out.join("react");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    let source = root.join("src");
    fs::create_dir_all(source.join("components"))?;
    let assets = assets::download(specification, &root, cookies).await?;
    let mut styles = css::build(specification, &assets);
    styles.css.push_str(interactions::FOCUS_CSS);
    styles.css.push_str(interactions::REDUCED_MOTION_CSS);
    let components = tree::components(specification, &styles.classes);
    let mut structural_classes = std::collections::HashSet::new();
    let mut state_classes = structural_css::class_maps(
        &specification.states,
        &styles.classes,
        &assets,
        &mut styles.css,
        &mut structural_classes,
    );
    for (state, classes) in specification.states.iter().zip(&mut state_classes) {
        animations::append_startup(&state.animations, classes, &mut styles.css);
    }
    let interaction_state_classes = specification
        .interactions
        .iter()
        .zip(&styles.interaction_classes)
        .map(|(interaction, classes)| {
            structural_css::class_maps(
                &interaction.states,
                classes,
                &assets,
                &mut styles.css,
                &mut structural_classes,
            )
        })
        .collect::<Vec<_>>();
    state_style_maps::append(
        specification,
        &state_classes,
        &interaction_state_classes,
        &assets,
        &mut styles.css,
    );
    let (html_class, body_class, root_class) = roots::classes(specification, &components);
    let has_root = specification.states.first().is_some_and(|state| {
        state.nodes.iter().any(|node| {
            node.attributes
                .get("id")
                .is_some_and(|value| value == "root")
        })
    });
    let (mount_source, mount_markup) = mount(has_root, &root_class)?;
    fs::write(source.join("styles.css"), styles.css)?;
    fs::write(
        source.join("App.jsx"),
        jsx::app(specification, &components, &state_classes, &assets),
    )?;
    let mut component_index = String::new();
    for component in &components.items {
        let directory = source.join("components").join(&component.name);
        fs::create_dir_all(&directory)?;
        fs::write(
            directory.join(format!("{}.jsx", component.name)),
            jsx::component(component, &components, &assets),
        )?;
        component_index.push_str(&format!(
            "export {{default as {0}}} from './{0}/{0}.jsx';\n",
            component.name
        ));
    }
    fs::write(source.join("components").join("index.js"), component_index)?;
    fs::write(
        source.join("states.jsx"),
        jsx_states::interaction_states(
            specification,
            &components,
            &interaction_state_classes,
            &assets,
        ),
    )?;
    fs::write(
        source.join("main.jsx"),
        format!(
            "import React from 'react';\nimport {{createRoot}} from 'react-dom/client';\nimport './styles.css';\nimport App from './App.jsx';\ndocument.documentElement.className={};\ndocument.body.className={};\n{mount_source}\n",
            serde_json::to_string(&html_class)?,
            serde_json::to_string(&body_class)?,
        ),
    )?;
    let title = specification
        .states
        .first()
        .map(|state| state.title.as_str())
        .unwrap_or("Recreate");
    fs::write(root.join("index.html"), document(title, mount_markup))?;
    fs::write(
        root.join("package.json"),
        r#"{"private":true,"scripts":{"dev":"vite","build":"vite build"},"dependencies":{"vite":"^8.1.0","react":"^19.2.0","react-dom":"^19.2.0"}}"#,
    )?;
    Ok(())
}

fn mount(has_root: bool, root_class: &str) -> Result<(String, &'static str)> {
    if !has_root {
        return Ok(("createRoot(document.body).render(<App />);".into(), ""));
    }
    Ok((
        format!(
            "const root=document.getElementById('root');\nroot.className={};\ncreateRoot(root).render(<App />);",
            serde_json::to_string(root_class)?
        ),
        "<div id=\"root\"></div>",
    ))
}

fn document(title: &str, mount_markup: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"icon\" href=\"data:,\"><title>{}</title></head><body>{mount_markup}<script type=\"module\" src=\"/src/main.jsx\"></script></body></html>",
        escape_html(title),
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
