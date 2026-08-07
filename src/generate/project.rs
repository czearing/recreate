use super::{project_mount::mount, *};

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
    fs::write(
        source.join("styles.css"),
        format!(
            "{}{}",
            root_reset(specification),
            source_css::dedupe_exact(&styles.css)
        ),
    )?;
    let mut app_source = jsx::app(specification, &components, &state_classes, &assets);
    let mut state_source = jsx_states::interaction_states(
        specification,
        &components,
        &interaction_state_classes,
        &assets,
    );
    super::source_svg_assets::extract(
        &mut [&mut app_source, &mut state_source],
        &root.join("public").join("assets"),
        &styles.css,
    )?;
    if let Some(block_source) =
        source_dedupe::extract_repeated_blocks(&mut [&mut app_source, &mut state_source])
    {
        source_generated_blocks::write(&source.join("components"), &block_source)?;
    }
    let generated_items = super::source_item_dedupe::extract(
        &mut [&mut app_source, &mut state_source],
        &components
            .items
            .iter()
            .map(|component| component.name.clone())
            .collect(),
    );
    if !generated_items.is_empty() {
        let directory = source.join("components").join("CollectionItems");
        fs::create_dir_all(&directory)?;
        let mut index = String::new();
        for item in generated_items {
            fs::write(directory.join(format!("{}.jsx", item.name)), item.source)?;
            index.push_str(&format!(
                "export {{{}}} from './{}.jsx';\n",
                item.name, item.name
            ));
        }
        fs::write(directory.join("index.js"), index)?;
    }
    source_split::split(
        &source,
        &mut app_source,
        &mut state_source,
        specification.interactions.len(),
    )?;
    fs::write(source.join("App.jsx"), app_source)?;
    let mut component_index = String::new();
    for component in &components.items {
        let directory = source.join("components").join(&component.name);
        fs::create_dir_all(&directory)?;
        let component_source = jsx::component(component, &components, &assets);
        fs::write(
            directory.join(format!("{}.jsx", component.name)),
            component_source,
        )?;
        component_index.push_str(&format!(
            "export {{default as {0}}} from './{0}/{0}.jsx';\n",
            component.name
        ));
    }
    fs::write(source.join("components").join("index.js"), component_index)?;
    fs::write(source.join("states.jsx"), state_source)?;
    source_style_split::split(&source)?;
    timing("sources");
    fs::write(
        source.join("main.jsx"),
        format!(
            "import React from 'react';\nimport {{createRoot}} from 'react-dom/client';\nimport generatedCss from './styles.css?inline';\nimport scopedStyles from './generated/scoped-styles.js';\nimport {{adoptRegisteredStyles}} from './runtime/style.mjs';\nimport App from './App.jsx';\nadoptRegisteredStyles(scopedStyles);\nconst rootAliases={};\nconst semanticCss=rootAliases.reduce((css,[className,elements])=>css.replaceAll(`.${{className}}{{`,`.${{className}},${{elements}}{{`),generatedCss);\nconst generatedSheet=new CSSStyleSheet();generatedSheet.replaceSync(semanticCss);document.adoptedStyleSheets=[...document.adoptedStyleSheets,generatedSheet];\ndocument.querySelector('script[data-recreate-entry]')?.remove();\nconst capturedBase=document.querySelector('base[data-recreate-base-href]');if(capturedBase){{capturedBase.href=capturedBase.dataset.recreateBaseHref;delete capturedBase.dataset.recreateBaseHref}}\n{}\n{}\n{mount_source}\n",
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

/// The user agent gives `body` an 8px margin and the generator emits no rule
/// for the document roots, so a source that reset that margin loses 16px of
/// width in the recreation. Replay the captured root box explicitly.
fn root_reset(specification: &Specification) -> String {
    let Some(state) = specification.states.first() else {
        return String::new();
    };
    let mut css = String::new();
    for tag in ["html", "body"] {
        let Some(node) = state.nodes.iter().find(|node| node.tag == tag) else {
            continue;
        };
        let declarations = ["margin", "padding"]
            .into_iter()
            .flat_map(|box_property| {
                ["top", "right", "bottom", "left"].map(move |side| format!("{box_property}-{side}"))
            })
            .filter_map(|property| {
                node.style
                    .get(&property)
                    .map(|value| format!("{property}:{value};"))
            })
            .collect::<String>();
        if !declarations.is_empty() {
            css.push_str(&format!("{tag}{{{declarations}}}\n"));
        }
    }
    css
}
