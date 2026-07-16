mod animations;
mod assets;
mod css;
mod jsx;
mod jsx_attrs;
mod jsx_states;
mod names;
mod roots;
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
    let styles = css::build(specification, &assets);
    let components = tree::components(specification, &styles.classes);
    let (html_class, body_class, root_class) = roots::classes(specification, &components);
    fs::write(source.join("styles.css"), styles.css)?;
    fs::write(
        source.join("App.jsx"),
        jsx::app(specification, &components, &assets),
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
            &styles.interaction_classes,
            &assets,
        ),
    )?;
    fs::write(
        source.join("main.jsx"),
        format!(
            "import React from 'react';\nimport {{createRoot}} from 'react-dom/client';\nimport './styles.css';\nimport App from './App.jsx';\ndocument.documentElement.className={};\ndocument.body.className={};\nconst root=document.getElementById('root');\nroot.className={};\ncreateRoot(root).render(<App />);\n",
            serde_json::to_string(&html_class)?,
            serde_json::to_string(&body_class)?,
            serde_json::to_string(&root_class)?
        ),
    )?;
    let title = specification
        .states
        .first()
        .map(|state| state.title.as_str())
        .unwrap_or("Recreate");
    fs::write(
        root.join("index.html"),
        format!(
            "<!doctype html><html><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title></head><body><div id=\"root\"></div><script type=\"module\" src=\"/src/main.jsx\"></script></body></html>",
            escape_html(title)
        ),
    )?;
    fs::write(
        root.join("package.json"),
        r#"{"private":true,"scripts":{"dev":"vite","build":"vite build"},"dependencies":{"vite":"^8.1.0","react":"^19.2.0","react-dom":"^19.2.0"}}"#,
    )?;
    Ok(())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
