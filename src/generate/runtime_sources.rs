use anyhow::Result;
use std::{fs, path::Path};

const MODULES: [(&str, &str); 6] = [
    (
        "interaction.mjs",
        include_str!("../../runtime/interaction.mjs"),
    ),
    ("sequence.mjs", include_str!("../../runtime/sequence.mjs")),
    ("textarea.mjs", include_str!("../../runtime/textarea.mjs")),
    ("anchor.mjs", include_str!("../../runtime/anchor.mjs")),
    ("style.mjs", include_str!("../../runtime/style.mjs")),
    ("shadow.mjs", include_str!("../../runtime/shadow.mjs")),
];

pub fn write(source: &Path) -> Result<()> {
    let runtime = source.join("runtime");
    fs::create_dir_all(&runtime)?;
    for (name, contents) in MODULES {
        fs::write(runtime.join(name), contents)?;
    }
    Ok(())
}

/// The entry module adopts the generated stylesheet, undoes the two markers the emitted
/// document carries for its own benefit, and mounts the app. It deliberately does not
/// touch the document root's classes: those are serialised into `index.html`, so anything
/// assigned here would race the markup and silently win.
///
/// The generated sheet is handed to the style owner rather than adopted here, because a
/// sheet adopted directly reaches the document and nothing else — and a shadow tree that
/// the app opens is a scope the document's sheets do not enter.
pub fn write_entry(source: &Path, mount_source: &str) -> Result<()> {
    fs::write(
        source.join("main.jsx"),
        format!(
            "import React from 'react';\nimport {{createRoot}} from 'react-dom/client';\nimport generatedCss from './styles.css?inline';\nimport scopedStyles from './generated/scoped-styles.js';\nimport {{adoptRegisteredStyles}} from './runtime/style.mjs';\nimport App from './App.jsx';\nadoptRegisteredStyles([...scopedStyles,generatedCss]);\ndocument.querySelector('script[data-recreate-entry]')?.remove();\nconst capturedBase=document.querySelector('base[data-recreate-base-href]');if(capturedBase){{capturedBase.href=capturedBase.dataset.recreateBaseHref;delete capturedBase.dataset.recreateBaseHref}}\n{mount_source}\n"
        ),
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "runtime_style_tests.rs"]
mod style_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn embeds_the_exact_tested_runtime_modules() {
        for (_, contents) in super::MODULES {
            assert!(!contents.trim().is_empty());
            assert!(contents.contains("export "));
        }
    }

    /// The entry module is the one place a sheet could reach the document without passing the
    /// owner, and a sheet adopted that way never reaches a shadow root the app opens.
    #[test]
    fn routes_every_sheet_through_the_one_owner_of_adoption() {
        let directory = tempfile::tempdir().unwrap();
        super::write_entry(
            directory.path(),
            "createRoot(document.body).render(<App/>);",
        )
        .unwrap();
        let entry = std::fs::read_to_string(directory.path().join("main.jsx")).unwrap();
        assert!(entry.contains("adoptRegisteredStyles([...scopedStyles,generatedCss])"));
        assert!(
            !entry.contains("document.adoptedStyleSheets="),
            "the entry module adopts a sheet the shadow roots will never see"
        );
    }

    /// The shadow component's own wiring into that owner, which no harness here executes: a
    /// root it opens and does not adopt into renders bare, and every emitted file looks the
    /// same either way.
    #[test]
    fn adopts_the_registered_styles_into_every_root_it_opens() {
        let shadow = super::MODULES
            .iter()
            .find(|(name, _)| *name == "shadow.mjs")
            .unwrap()
            .1;
        assert!(shadow.contains("import {adoptInto} from './style.mjs'"));
        assert!(shadow.contains("adoptInto(root)"));
    }
}
