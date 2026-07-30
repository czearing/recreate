use anyhow::Result;
use std::{fs, path::Path};

pub fn split(
    source: &Path,
    app: &mut String,
    states: &mut String,
    interaction_count: usize,
) -> Result<()> {
    split_states(source, states, interaction_count)?;
    split_views(source, app)?;
    split_app_data(source, app)?;
    format_app(app);
    Ok(())
}

fn split_app_data(source: &Path, app: &mut String) -> Result<()> {
    let names = [
        "transitionGraph",
        "transitionEdges",
        "controlStyles",
        "baselineSelectedTokens",
        "baselineSelectedState",
        "baselinePressedTokens",
        "returnStorageKey",
        "viewportWidths",
        "focusedTargets",
        "closableStates",
        "statefulStates",
        "replacementStates",
        "capturedScrolls",
        "initialScrolls",
        "carouselPrevious",
        "carouselNext",
        "carouselState",
        "attributeSequences",
        "responsiveAttributePaths",
        "responsiveAttributeValues",
        "responsiveAttributes",
    ];
    let mut data = Vec::new();
    let mut kept = Vec::new();
    for line in app.lines() {
        if names
            .iter()
            .find(|name| line.trim_start().starts_with(&format!("const {name}=")))
            .is_some()
        {
            data.push(line.replacen("const ", "export const ", 1));
        } else {
            kept.push(line);
        }
    }
    if data.is_empty() {
        return Ok(());
    }
    let directory = source.join("generated");
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join("state-data.js"),
        format!("{}\n", data.join("\n")),
    )?;
    let import = format!(
        "import {{{}}} from './generated/state-data.js';",
        names.join(",")
    );
    let insert = kept
        .iter()
        .position(|line| !line.starts_with("import "))
        .unwrap_or_default();
    kept.insert(insert, &import);
    *app = kept.join("\n");
    Ok(())
}

fn format_app(app: &mut String) {
    *app = app
        .replace(
            "export default function App(){",
            "export default function App() {\n  ",
        )
        .replace(";const", ";\n  const")
        .replace(";useLayoutEffect", ";\n  useLayoutEffect")
        .replace(";useEffect", ";\n  useEffect")
        .replace(";return replacementStates", ";\n  return replacementStates");
}

fn split_states(source: &Path, states: &mut String, count: usize) -> Result<()> {
    let imports = component_imports(states, "../components");
    let mut starts = (1..=count)
        .filter_map(|index| {
            let view = states.find(&format!("function Interaction{index}View0"));
            let empty = states.find(&format!("export function Interaction{index}("));
            view.into_iter()
                .chain(empty)
                .min()
                .map(|start| (index, start))
        })
        .collect::<Vec<_>>();
    starts.sort_by_key(|(_, start)| *start);
    let Some((_, first)) = starts.first().copied() else {
        return Ok(());
    };
    let runtime = remove_component_imports(&states[..first]);
    let directory = source.join("states");
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join("runtime.jsx"),
        format!(
            "{runtime}\nexport {{keyActivate,selectViewport,ExistingSurface,ReplacementSurface,InsertedSurface,AnchoredSurface}};\n"
        ),
    )?;
    let mut index_source = String::new();
    for (position, (index, start)) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .map(|(_, start)| *start)
            .unwrap_or(states.len());
        let segment = states[*start..end].trim();
        if segment.lines().count() >= 900 && segment.contains("View0") {
            super::source_view_split::split_interaction_views(
                &directory, *index, segment, &imports,
            )?;
        } else {
            fs::write(
                directory.join(format!("Interaction{index}.jsx")),
                format!(
                    "import React from 'react';\nimport {{keyActivate,selectViewport,ExistingSurface,ReplacementSurface,InsertedSurface,AnchoredSurface}} from './runtime.jsx';\n{imports}\n\n{segment}\n"
                ),
            )?;
        }
        index_source.push_str(&format!(
            "export {{Interaction{index}}} from './states/Interaction{index}.jsx';\n"
        ));
    }

    *states = index_source;
    Ok(())
}

fn split_views(source: &Path, app: &mut String) -> Result<()> {
    let Some(views_end) = app.find("const baselineViews=") else {
        return Ok(());
    };
    let mut starts = Vec::new();
    let mut index = 0;
    loop {
        let needle = format!("function Baseline{index}(");
        let Some(start) = app.find(&needle) else {
            break;
        };
        starts.push((index, start));
        index += 1;
    }
    let Some((_, first)) = starts.first().copied() else {
        return Ok(());
    };
    let imports = component_imports(app, "../components");
    let directory = source.join("views");
    fs::create_dir_all(&directory)?;
    let mut view_imports = String::new();
    for (position, (index, start)) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .map(|(_, start)| *start)
            .unwrap_or(views_end);
        let segment = app[*start..end].trim().replacen(
            &format!("function Baseline{index}("),
            &format!("export default function Baseline{index}("),
            1,
        );
        fs::write(
            directory.join(format!("Baseline{index}.jsx")),
            format!(
                "import React from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{moveCarousel}} from '../runtime/carousel.mjs';\n{imports}\n\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\n\n{segment}\n"
            ),
        )?;
        view_imports.push_str(&format!(
            "import Baseline{index} from './views/Baseline{index}.jsx';\n"
        ));
    }
    let prefix = remove_component_imports(&app[..first]);
    let suffix = &app[views_end..];
    *app = format!("{view_imports}{prefix}{suffix}");
    Ok(())
}

fn component_imports(source: &str, relative: &str) -> String {
    source
        .lines()
        .filter(|line| line.starts_with("import ") && line.contains("./components/"))
        .map(|line| line.replace("./components", relative))
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_component_imports(source: &str) -> String {
    source
        .lines()
        .filter(|line| !(line.starts_with("import ") && line.contains("./components/")))
        .collect::<Vec<_>>()
        .join("\n")
}
