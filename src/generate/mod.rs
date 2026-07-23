#[cfg(test)]
mod animation_order_tests;
mod animation_timing;
pub(crate) mod animations;
mod assets;
mod assets_remote;
mod attribute_sequences;
#[cfg(test)]
mod authenticated_interaction_runtime_tests;
mod authored_css;
mod authored_media;
mod carousel_inference;
mod css;
mod css_layout;
mod css_values;
mod custom_properties;
mod custom_property_diff;
mod document;
mod inherited_styles;
mod initial_scroll;
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
mod jsx_text_entry;
mod jsx_variants;
#[cfg(test)]
#[path = "mount_tests.rs"]
mod mount_tests;
mod names;
#[cfg(test)]
mod project_test_support;
mod responsive;
mod responsive_attributes;
mod responsive_geometry;
mod responsive_height;
#[cfg(test)]
mod responsive_runtime_support;
#[cfg(test)]
mod responsive_runtime_tests;
mod roots;
mod runtime_sources;
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
    let started = std::time::Instant::now();
    let timing = |phase: &str| {
        if std::env::var_os("RECREATE_TIMING").is_some() {
            eprintln!("generate_{phase}={:.3}s", started.elapsed().as_secs_f64());
        }
    };
    let mut bytes = fs::read(spec)?;
    timing("read");
    let mut specification: Specification = simd_json::serde::from_slice(&mut bytes)?;
    crate::interactions::deduplicate(&mut specification.interactions);
    crate::interaction_surface::normalize(&mut specification);
    timing("parse");
    write_project(&specification, out, &[]).await?;
    timing("write");
    std::mem::forget(specification);
    std::mem::forget(bytes);
    Ok(())
}

pub async fn write_project(
    specification: &Specification,
    out: &Path,
    cookies: &[BrowserCookie],
) -> Result<()> {
    let started = std::time::Instant::now();
    let timing = |phase: &str| {
        if std::env::var_os("RECREATE_TIMING").is_some() {
            eprintln!("project_{phase}={:.3}s", started.elapsed().as_secs_f64());
        }
    };
    let root = out.join("react");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    let source = root.join("src");
    fs::create_dir_all(source.join("components"))?;
    runtime_sources::write(&source)?;
    let assets = assets::download(specification, &root, cookies).await?;
    timing("assets");
    let mut styles = css::build(specification, &assets);
    timing("css");
    styles.css.push_str(interactions::FOCUS_CSS);
    styles.css.push_str(interactions::REDUCED_MOTION_CSS);
    let components = tree::components(specification, &styles.classes);
    timing("components");
    let mut structural_classes = std::collections::HashSet::new();
    let mut state_classes = structural_css::class_maps(
        &specification.states,
        &styles.classes,
        &assets,
        &mut styles.css,
        &mut structural_classes,
        None,
    );
    for (state, classes) in specification.states.iter().zip(&mut state_classes) {
        animations::append_startup(&state.animations, classes, &mut styles.css);
    }
    let interaction_state_classes = specification
        .interactions
        .iter()
        .zip(&styles.interaction_classes)
        .map(|(interaction, classes)| {
            if !interactions::rendered(interaction, &specification.states) {
                return Vec::new();
            }
            let surface_paths = interactions::shared_trigger(interaction, &specification.states)
                .then(|| {
                    crate::interaction_surface::paths(&interaction.states, &specification.states)
                });
            structural_css::class_maps(
                &interaction.states,
                classes,
                &assets,
                &mut styles.css,
                &mut structural_classes,
                surface_paths.as_ref(),
            )
        })
        .collect::<Vec<_>>();
    timing("classes");
    state_style_maps::append(
        specification,
        &state_classes,
        &interaction_state_classes,
        &assets,
        &mut styles.css,
    );
    timing("state_styles");
    let (html_class, body_class, root_class) = roots::classes(specification, &components);
    let baseline = specification.states.first();
    let authored_class = |tag: &str| {
        baseline
            .and_then(|state| state.nodes.iter().find(|node| node.tag == tag))
            .and_then(|node| node.attributes.get("class"))
            .is_some_and(|value| !value.is_empty())
    };
    let mut root_aliases = std::collections::BTreeMap::<String, Vec<&str>>::new();
    if !authored_class("html") {
        root_aliases
            .entry(html_class.clone())
            .or_default()
            .push("html");
    }
    if !authored_class("body") {
        root_aliases
            .entry(body_class.clone())
            .or_default()
            .push("body");
    }
    root_aliases.remove("");
    let root_aliases = root_aliases
        .into_iter()
        .map(|(class_name, elements)| (class_name, elements.join(",")))
        .collect::<Vec<_>>();
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
    timing("sources");
    fs::write(
        source.join("main.jsx"),
        format!(
            "import React from 'react';\nimport {{createRoot}} from 'react-dom/client';\nimport generatedCss from './styles.css?inline';\nimport App from './App.jsx';\nconst rootAliases={};\nconst semanticCss=rootAliases.reduce((css,[className,elements])=>css.replaceAll(`.${{className}}{{`,`.${{className}},${{elements}}{{`),generatedCss);\nconst generatedSheet=new CSSStyleSheet();generatedSheet.replaceSync(semanticCss);document.adoptedStyleSheets=[...document.adoptedStyleSheets,generatedSheet];\ndocument.querySelector('script[data-recreate-entry]')?.remove();\nconst capturedBase=document.querySelector('base[data-recreate-base-href]');if(capturedBase){{capturedBase.href=capturedBase.dataset.recreateBaseHref;delete capturedBase.dataset.recreateBaseHref}}\n{}\n{}\n{mount_source}\n",
            serde_json::to_string(&root_aliases)?,
            if authored_class("html") {
                format!(
                    "document.documentElement.className={};",
                    serde_json::to_string(&html_class)?
                )
            } else {
                "document.documentElement.removeAttribute('class');".into()
            },
            if authored_class("body") {
                format!(
                    "document.body.className={};",
                    serde_json::to_string(&body_class)?
                )
            } else {
                "document.body.removeAttribute('class');".into()
            },
        ),
    )?;
    fs::write(
        root.join("index.html"),
        document::render(specification.states.first(), mount_markup, &styles.classes),
    )?;
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
