use super::source_dedupe_support::jsx_blocks;
use anyhow::Result;
use std::{fs, path::Path};

pub fn split_interaction_views(
    directory: &Path,
    index: usize,
    source: &str,
    imports: &str,
) -> Result<()> {
    let footer_start = source
        .find(&format!("const interaction{index}Views="))
        .expect("interaction view list should exist");
    let view_source = &source[..footer_start];
    let footer = &source[footer_start..];
    let starts = view_starts(view_source, index);
    let views = directory.join(format!("Interaction{index}"));
    fs::create_dir_all(&views)?;
    let mut view_imports = String::new();
    for (position, (view, start)) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .map(|(_, start)| *start)
            .unwrap_or(view_source.len());
        let mut function = view_source[*start..end].trim().replacen(
            &format!("function Interaction{index}View{view}"),
            &format!("export default function Interaction{index}View{view}"),
            1,
        );
        let surface_imports = split_view_surfaces(&views, index, *view, &mut function, imports)?;
        fs::write(
            views.join(format!("Interaction{index}View{view}.jsx")),
            view_module(&function, &surface_imports, imports),
        )?;
        view_imports.push_str(&format!(
            "import Interaction{index}View{view} from './Interaction{index}/Interaction{index}View{view}.jsx';\n"
        ));
    }
    fs::write(
        directory.join(format!("Interaction{index}.jsx")),
        format!(
            "import React from 'react';\nimport {{selectViewport}} from './runtime.jsx';\n{view_imports}\n{footer}\n"
        ),
    )?;
    Ok(())
}

fn view_starts(source: &str, index: usize) -> Vec<(usize, usize)> {
    let mut starts = Vec::new();
    let mut view = 0;
    while let Some(start) = source.find(&format!("function Interaction{index}View{view}")) {
        starts.push((view, start));
        view += 1;
    }
    starts
}

fn view_module(function: &str, surface_imports: &str, imports: &str) -> String {
    format!(
        "import React from 'react';\nimport {{keyActivate,ExistingSurface,ReplacementSurface,InsertedSurface,AnchoredSurface}} from '../runtime.jsx';\n{surface_imports}{}\n\n{function}\n",
        imports.replace("../components", "../../components")
    )
}

fn split_view_surfaces(
    directory: &Path,
    interaction: usize,
    view: usize,
    source: &mut String,
    imports: &str,
) -> Result<String> {
    let mut blocks = replacement_surfaces(source);
    let mut module_imports = String::new();
    for (surface, (start, end, mut block)) in blocks.drain(..).enumerate().rev() {
        let section_imports =
            split_collection_sections(directory, interaction, view, surface, &mut block, imports)?;
        let name = format!("Interaction{interaction}View{view}Surface{surface}");
        fs::write(
            directory.join(format!("{name}.jsx")),
            format!(
                "import React from 'react';\nimport {{ReplacementSurface}} from '../runtime.jsx';\n{section_imports}{}\n\nexport default function {name}() {{\n  return (\n    {block}\n  );\n}}\n",
                imports.replace("../components", "../../components")
            ),
        )?;
        source.replace_range(start..end, &format!("<{name} />"));
        module_imports.push_str(&format!("import {name} from './{name}.jsx';\n"));
    }
    Ok(module_imports)
}

fn replacement_surfaces(source: &str) -> Vec<(usize, usize, String)> {
    let mut blocks = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = source[offset..].find("<ReplacementSurface") {
        let start = offset + relative_start;
        let Some(relative_end) = source[start..].find("</ReplacementSurface>") else {
            break;
        };
        let end = start + relative_end + "</ReplacementSurface>".len();
        blocks.push((start, end, source[start..end].to_string()));
        offset = end;
    }
    blocks
}

fn split_collection_sections(
    directory: &Path,
    interaction: usize,
    view: usize,
    surface: usize,
    source: &mut String,
    imports: &str,
) -> Result<String> {
    if source.lines().count() < 900 {
        return Ok(String::new());
    }
    let children = jsx_blocks(source)
        .into_iter()
        .filter(|(start, _, block)| {
            *start > 0
                && source
                    .as_bytes()
                    .get(*start)
                    .is_some_and(|byte| *byte == b'<')
                && !block.starts_with("<ReplacementSurface")
        })
        .collect::<Vec<_>>();
    let groups = section_groups(&children, 500);
    let mut replacements = Vec::new();
    let mut section_imports = String::new();
    for (section, (start, end)) in groups.into_iter().enumerate() {
        let suffix = ["Primary", "Continuation", "Remainder"]
            .get(section)
            .map_or_else(|| format!("Section{}", section + 1), |name| (*name).into());
        let name = format!("Interaction{interaction}View{view}Collection{suffix}Surface{surface}");
        write_section(directory, &name, &source[start..end], imports)?;
        replacements.push((start, end, format!("<{name} />")));
        section_imports.push_str(&format!("import {name} from './{name}.jsx';\n"));
    }
    super::source_dedupe_support::replace_ranges(source, &mut replacements);
    Ok(section_imports)
}

fn section_groups(children: &[(usize, usize, String)], target_lines: usize) -> Vec<(usize, usize)> {
    let mut groups = Vec::new();
    let mut current = None;
    let mut lines = 0;
    for (start, end, block) in children {
        if lines >= target_lines {
            groups.push((current.take().expect("group start"), *start));
            lines = 0;
        }
        current.get_or_insert(*start);
        lines += block.lines().count();
        if *end == children.last().map_or(*end, |child| child.1) {
            groups.push((current.take().expect("group start"), *end));
        }
    }
    groups
}

fn write_section(directory: &Path, name: &str, source: &str, imports: &str) -> Result<()> {
    fs::write(
        directory.join(format!("{name}.jsx")),
        format!(
            "import React from 'react';\n{}\n\nexport default function {name}() {{\n  return (\n    <>\n{source}\n    </>\n  );\n}}\n",
            imports.replace("../components", "../../components")
        ),
    )?;
    Ok(())
}
