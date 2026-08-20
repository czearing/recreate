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
        let authored = super::animation_keyframes::authored_names(&styles.css);
        let starting = super::before_change::BeforeChange::new(&state.css_rules, &state.nodes)
            .with_entry_motion(&state.nodes, &state.animations);
        animations::append_startup(
            &state.animations,
            &authored,
            &starting,
            classes,
            &mut styles.css,
        );
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
    let root_class = roots::root_class(specification, &components);
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
            super::root_reset::root_reset(specification, &assets),
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
    let relocation_css = format!(
        "{}{}",
        styles.css,
        super::relocation_binding::rules(
            &std::iter::once((&specification.states[0], &styles.classes))
                .chain(
                    specification
                        .interactions
                        .iter()
                        .filter_map(|interaction| interaction.states.first())
                        .zip(&styles.interaction_classes)
                )
                .collect::<Vec<_>>(),
            &styles.css,
        )
    );
    super::source_svg_assets::extract(
        &mut [&mut app_source, &mut state_source],
        &root.join("public").join("assets"),
        &relocation_css,
    )?;
    let exported = components
        .items
        .iter()
        .map(|component| component.name.clone())
        .collect();
    if let Some(block_source) =
        source_dedupe::extract_repeated_blocks(&mut [&mut app_source, &mut state_source], &exported)
    {
        source_generated_blocks::write(&source.join("components"), &block_source)?;
    }
    let generated_items =
        super::source_item_dedupe::extract(&mut [&mut app_source, &mut state_source], &exported);
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
        let component_source = jsx::component(component, &components);
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
    source_imports::prune_tree(&source)?;
    timing("sources");
    runtime_sources::write_entry(&source, &mount_source)?;
    fs::write(
        root.join("index.html"),
        document::render(
            specification.states.first(),
            mount_markup,
            &styles.classes,
            &assets,
        ),
    )?;
    fs::write(
        root.join("package.json"),
        r#"{"private":true,"scripts":{"dev":"vite","build":"vite build"},"dependencies":{"vite":"^8.1.0","react":"^19.2.0","react-dom":"^19.2.0"}}"#,
    )?;
    Ok(())
}
