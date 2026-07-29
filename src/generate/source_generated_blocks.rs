use anyhow::Result;
use std::{fs, path::Path};

pub fn write(directory: &Path, source: &str) -> Result<()> {
    let blocks = directory.join("SharedComponents");
    fs::create_dir_all(&blocks)?;
    let imports = source
        .lines()
        .take_while(|line| line.starts_with("import "))
        .collect::<Vec<_>>()
        .join("\n");
    let starts = component_starts(source);
    let mut index = String::new();
    for (position, (name, start)) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .map(|(_, start)| *start)
            .unwrap_or(source.len());
        let component = compact_button_lists(source[*start..end].trim());
        fs::write(
            blocks.join(format!("{name}.jsx")),
            format!("{imports}\n\n{component}\n"),
        )?;
        index.push_str(&format!("export {{{name}}} from './{name}.jsx';\n"));
    }
    fs::write(blocks.join("index.js"), &index)?;
    fs::write(
        directory.join("SharedComponents.jsx"),
        "export * from './SharedComponents/index.js';\n",
    )?;
    Ok(())
}

fn component_starts(source: &str) -> Vec<(String, usize)> {
    let mut starts = Vec::new();
    let mut offset = 0;
    while let Some(relative) = source[offset..].find("export function ") {
        let start = offset + relative;
        let name_start = start + "export function ".len();
        let name_end = source[name_start..].find('(').unwrap() + name_start;
        starts.push((source[name_start..name_end].to_string(), start));
        offset = name_end;
    }
    starts
}

fn compact_button_lists(source: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut models = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(first) = button(&lines, index) else {
            output.push(lines[index].to_string());
            index += 1;
            continue;
        };
        let start = index;
        let mut options = vec![first];
        index += 3;
        while let Some(option) = button(&lines, index) {
            options.push(option);
            index += 3;
        }
        if options.len() < 8 {
            output.extend(lines[start..index].iter().map(|line| (*line).to_string()));
            continue;
        }
        let indent = &lines[start][..lines[start].len() - lines[start].trim_start().len()];
        let name = option_name(&output, models.len());
        output.extend(mapped_buttons(indent, &name));
        models.push((name, options));
    }
    if models.is_empty() {
        return output.join("\n");
    }
    let model = models
        .into_iter()
        .map(|(name, options)| {
            format!(
                "const {name} = [\n{}\n];",
                options
                    .iter()
                    .map(|(class_name, title, label)| format!(
                        "  [{}, {}, {}],",
                        json(class_name),
                        json(title),
                        json(label)
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let joined = output.join("\n");
    let insertion = joined.find("export function ").unwrap_or_default();
    format!(
        "{}\n{model}\n\n{}",
        &joined[..insertion],
        &joined[insertion..]
    )
}

fn option_name(output: &[String], index: usize) -> String {
    output
        .iter()
        .rev()
        .filter_map(|line| expression_string(line.trim()))
        .find(|label| label.chars().any(char::is_alphabetic))
        .map(|label| {
            let words = label
                .split(|character: char| !character.is_alphanumeric())
                .filter(|word| !word.is_empty())
                .collect::<Vec<_>>();
            let mut name = words
                .first()
                .map_or(String::new(), |word| word.to_lowercase());
            for word in words.iter().skip(1) {
                let mut characters = word.chars();
                if let Some(first) = characters.next() {
                    name.extend(first.to_uppercase());
                    name.extend(characters.flat_map(char::to_lowercase));
                }
            }
            format!("{name}EmojiOptions")
        })
        .unwrap_or_else(|| format!("emojiOptions{}", index + 1))
}

fn button(lines: &[&str], index: usize) -> Option<(String, String, String)> {
    let opening = lines.get(index)?.trim();
    let label = lines.get(index + 1)?.trim();
    let closing = lines.get(index + 2)?.trim();
    if !opening.starts_with("<button ") || closing != "</button>" {
        return None;
    }
    Some((
        property(opening, "className")?,
        property(opening, "title")?,
        expression_string(label)?,
    ))
}

fn property(source: &str, name: &str) -> Option<String> {
    let marker = format!("{name}={{");
    let start = source.find(&marker)? + marker.len();
    let end = source[start..].find('}')? + start;
    serde_json::from_str(&source[start..end]).ok()
}

fn expression_string(source: &str) -> Option<String> {
    let value = source.strip_prefix('{')?.strip_suffix('}')?;
    serde_json::from_str(value).ok()
}

fn json(source: &str) -> String {
    serde_json::to_string(source).expect("string serialization")
}

fn mapped_buttons(indent: &str, name: &str) -> Vec<String> {
    [
        format!("{{{name}.map(([className, title, label]) => ("),
        "  <button key={`${title}-${label}`} className={className} title={title} type={\"button\"}>"
            .into(),
        "    {label}".into(),
        "  </button>".into(),
        "))}".into(),
    ]
    .into_iter()
    .map(|line| format!("{indent}{line}"))
    .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_button_models_with_their_component() {
        let buttons = (0..8)
            .map(|index| {
                format!(
                    "    <button className={{\"item\"}} title={{\"option {index}\"}} type={{\"button\"}}>\n      {{\"😀\"}}\n    </button>"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let source = format!(
            "export function Picker() {{\n  return (\n  <div>\n    {{\"Faces\"}}\n{buttons}\n  </div>\n  );\n}}"
        );
        let compact = super::compact_button_lists(&source);
        assert!(compact.contains("const facesEmojiOptions = ["));
        assert!(compact.find("const ").unwrap() < compact.find("export function").unwrap());
        assert!(compact.contains("facesEmojiOptions.map"));
    }
}
