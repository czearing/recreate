use super::{
    generated_source::{generated_class, jsx_classes, jsx_files},
    source_style_compact::compact_unique_generated,
    source_style_support::{brace_delta, css_classes, format_css},
};
use anyhow::Result;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

pub fn split(source: &Path) -> Result<()> {
    let files = jsx_files(source)?;
    let component_root = source.join("components");
    let mut owners = HashMap::<String, HashSet<PathBuf>>::new();
    let mut component_owners = HashMap::<String, PathBuf>::new();
    for file in &files {
        for class_name in jsx_classes(&fs::read_to_string(file)?) {
            if file.starts_with(&component_root) {
                component_owners
                    .entry(class_name.clone())
                    .or_insert_with(|| file.clone());
            }
            owners.entry(class_name).or_default().insert(file.clone());
        }
    }
    let css_file = source.join("styles.css");
    let mut shared = String::new();
    let mut scoped = BTreeMap::<PathBuf, String>::new();
    for line in fs::read_to_string(&css_file)?.lines() {
        if line.trim_start().starts_with('@') {
            shared.push_str(line);
            shared.push('\n');
            continue;
        }
        let all_classes = css_classes(line);
        let mut generated = all_classes
            .iter()
            .filter(|class_name| generated_class(class_name))
            .peekable();
        if generated.peek().is_some() && generated.all(|name| !owners.contains_key(*name)) {
            continue;
        }
        let classes = all_classes
            .into_iter()
            .filter(|class_name| owners.contains_key(*class_name))
            .collect::<Vec<_>>();
        let owner = (!classes.is_empty() && brace_delta(line) == 0)
            .then(|| resolve_owner(&classes, &owners, &component_owners))
            .flatten();
        if let Some(owner) = owner {
            scoped
                .entry(owner)
                .or_default()
                .push_str(&format!("{line}\n"));
        } else {
            shared.push_str(line);
            shared.push('\n');
        }
    }
    write_shared(source, &css_file, &compact_unique_generated(&shared))?;
    write_scoped(source, scoped)?;
    Ok(())
}

fn resolve_owner(
    classes: &[&str],
    owners: &HashMap<String, HashSet<PathBuf>>,
    component_owners: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    let mut resolved = None;
    for class_name in classes {
        let current = if let Some(owner) = component_owners.get(*class_name) {
            owner.clone()
        } else {
            let class_owners = owners.get(*class_name)?;
            if class_owners.len() != 1 {
                return None;
            }
            class_owners.iter().next()?.clone()
        };
        if resolved.as_ref().is_some_and(|owner| owner != &current) {
            return None;
        }
        resolved = Some(current);
    }
    resolved
}

fn write_shared(source: &Path, css_file: &Path, shared: &str) -> Result<()> {
    let styles = source.join("styles");
    fs::create_dir_all(&styles)?;
    let shards = super::source_style_shards::formatted(shared, 150_000, 900);
    if shards.len() <= 1 {
        fs::write(
            css_file,
            format_css(shards.first().map_or("", String::as_str)),
        )?;
        return Ok(());
    }
    let imports = shards
        .iter()
        .enumerate()
        .map(|(index, _)| format!("@import './styles/shared-{index}.css';"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(css_file, format!("{imports}\n"))?;
    for (index, shard) in shards.into_iter().enumerate() {
        fs::write(styles.join(format!("shared-{index}.css")), shard)?;
    }
    Ok(())
}

fn write_scoped(root: &Path, scoped: BTreeMap<PathBuf, String>) -> Result<()> {
    let mut manifest = String::new();
    let mut names = Vec::new();
    for (index, (jsx, css)) in scoped.into_iter().enumerate() {
        let css_file = jsx.with_extension("css");
        let name = css_file
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let stem = css_file
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let shards = super::source_style_shards::formatted(&css, 150_000, 900);
        if shards.len() == 1 {
            fs::write(&css_file, &shards[0])?;
        } else {
            let imports = shards
                .iter()
                .enumerate()
                .map(|(index, _)| format!("@import './{stem}-{index}.css';"))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(&css_file, format!("{imports}\n"))?;
            for (index, shard) in shards.into_iter().enumerate() {
                fs::write(
                    css_file.with_file_name(format!("{stem}-{index}.css")),
                    shard,
                )?;
            }
        }
        let mut source = fs::read_to_string(&jsx)?;
        let depth = jsx
            .parent()
            .and_then(|parent| parent.strip_prefix(root).ok())
            .map_or(0, |relative| relative.components().count());
        let runtime = if depth == 0 {
            "./runtime/style.mjs".to_string()
        } else {
            format!("{}runtime/style.mjs", "../".repeat(depth))
        };
        source.insert_str(
            0,
            &format!(
                "import componentCss from './{name}?inline';\nimport {{registerStyle}} from '{runtime}';\nregisterStyle(componentCss);\n"
            ),
        );
        fs::write(jsx, source)?;
        let relative = css_file
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        manifest.push_str(&format!(
            "import style{index} from '../{relative}?inline';\n"
        ));
        names.push(format!("style{index}"));
    }
    let generated = root.join("generated");
    fs::create_dir_all(&generated)?;
    manifest.push_str(&format!("export default [{}];\n", names.join(",")));
    fs::write(generated.join("scoped-styles.js"), manifest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_small_shared_css_in_the_entry_sheet() {
        let directory = tempfile::tempdir().unwrap();
        let css_file = directory.path().join("styles.css");
        super::write_shared(directory.path(), &css_file, ".a{color:red;}\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(css_file).unwrap(),
            ".a {\n  color:red;\n}\n"
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("styles"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn class_discovery_ignores_declaration_values() {
        assert_eq!(
            super::css_classes(".r123{content:'.s456';background:url(icon.svg)}"),
            vec!["r123"]
        );
    }

    #[test]
    fn keeps_a_rule_whose_class_page_text_would_otherwise_hide() {
        let swept = |text: &str| {
            let directory = tempfile::tempdir().unwrap();
            std::fs::write(
                directory.path().join("App.jsx"),
                format!(
                    r#"<div className={{"r1234567890"}} />{{"{text}"}}<Surface entries={{[["a","s00000000ff"]]}}/>"#
                ),
            )
            .unwrap();
            std::fs::write(directory.path().join("states.jsx"), "").unwrap();
            std::fs::write(
                directory.path().join("styles.css"),
                ".r1234567890{color:red;}\n.s00000000ff{color:blue;}\n.rdeadbeef00{color:green;}\n",
            )
            .unwrap();
            super::split(directory.path()).unwrap();
            let mut css = String::new();
            for entry in std::fs::read_dir(directory.path()).unwrap().flatten() {
                if entry.path().extension().is_some_and(|value| value == "css") {
                    css.push_str(&std::fs::read_to_string(entry.path()).unwrap());
                }
            }
            css
        };
        let plain = swept("plain");
        assert!(plain.contains(".s00000000ff"), "a bound class keeps its rule");
        assert!(
            !plain.contains(".rdeadbeef00"),
            "a class no file binds is genuinely dead and its rule is swept"
        );
        assert_eq!(
            swept(r#"he said \"go"#),
            plain,
            "a quote in page text must not decide which rules survive"
        );
    }

    #[test]
    fn keeps_multi_rule_media_blocks_shared_and_complete() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("App.jsx"),
            r#"<div className={"r1234567890"} />"#,
        )
        .unwrap();
        std::fs::write(directory.path().join("states.jsx"), "").unwrap();
        std::fs::write(
            directory.path().join("styles.css"),
            "@media(max-width:768px){.r1234567890{font-size:53.76px;}.rffffffffff{width:auto;}}\n",
        )
        .unwrap();
        super::split(directory.path()).unwrap();
        let css = std::fs::read_to_string(directory.path().join("styles.css")).unwrap();
        assert!(css.contains("font-size:53.76px"));
        assert!(css.contains(".rffffffffff"));
    }
}
