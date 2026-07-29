use anyhow::Result;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub fn format_css(source: &str) -> String {
    let mut output = String::new();
    let mut indent = 0_usize;
    let mut quote = None;
    let mut comment = false;
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if comment {
            output.push(character);
            if character == '*' && characters.peek() == Some(&'/') {
                output.push('/');
                characters.next();
                comment = false;
            }
        } else if quote.is_none() && character == '/' && characters.peek() == Some(&'*') {
            output.push_str("/*");
            characters.next();
            comment = true;
        } else if let Some(active) = quote {
            output.push(character);
            if character == active && !output.ends_with(&format!("\\{active}")) {
                quote = None;
            }
        } else if matches!(character, '"' | '\'') {
            quote = Some(character);
            output.push(character);
        } else if character == '{' {
            trim_end(&mut output);
            output.push_str(" {\n");
            indent += 1;
            output.push_str(&"  ".repeat(indent));
        } else if character == '}' {
            indent = indent.saturating_sub(1);
            trim_end(&mut output);
            output.push('\n');
            output.push_str(&"  ".repeat(indent));
            output.push_str("}\n");
            output.push_str(&"  ".repeat(indent));
        } else if character == ';' {
            trim_end(&mut output);
            output.push_str(";\n");
            output.push_str(&"  ".repeat(indent));
        } else if character.is_whitespace() {
            if !output.ends_with([' ', '\n']) {
                output.push(' ');
            }
        } else {
            output.push(character);
        }
    }
    format!("{}\n", output.trim())
}

fn trim_end(output: &mut String) {
    output.truncate(output.trim_end().len());
}

pub fn jsx_files(source: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![source.join("App.jsx"), source.join("states.jsx")];
    for directory in ["components", "states", "views"] {
        let directory = source.join(directory);
        if directory.exists() {
            collect_jsx(&directory, &mut files)?;
        }
    }
    Ok(files)
}

fn collect_jsx(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)?.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_jsx(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "jsx") {
            files.push(path);
        }
    }
    Ok(())
}

pub fn jsx_classes(source: &str) -> HashSet<String> {
    let mut classes = HashSet::new();
    for marker in ["className={\"", "className=\""] {
        let mut remaining = source;
        while let Some(index) = remaining.find(marker) {
            remaining = &remaining[index + marker.len()..];
            let Some(end) = remaining.find('"') else {
                break;
            };
            classes.extend(remaining[..end].split_whitespace().map(str::to_string));
            remaining = &remaining[end + 1..];
        }
    }
    for value in source.split('"').skip(1).step_by(2) {
        classes.extend(
            value
                .split_whitespace()
                .filter(|value| generated_class(value))
                .map(str::to_string),
        );
    }
    classes
}

fn generated_class(value: &str) -> bool {
    matches!(value.as_bytes().first(), Some(b'r' | b's'))
        && value.len() == 11
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

pub fn css_classes(source: &str) -> Vec<&str> {
    let start = if source.starts_with('.') {
        0
    } else if let Some(start) = source.rfind("{.") {
        start + 1
    } else {
        return Vec::new();
    };
    let Some(open) = source[start..].find('{').map(|open| start + open) else {
        return Vec::new();
    };
    let mut classes = Vec::new();
    let mut remaining = &source[start..open];
    while let Some(index) = remaining.find('.') {
        remaining = &remaining[index + 1..];
        let length = remaining
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
            .map(char::len_utf8)
            .sum();
        if length == 0 {
            continue;
        }
        classes.push(&remaining[..length]);
        remaining = &remaining[length..];
    }
    classes
}

pub fn brace_delta(source: &str) -> i32 {
    source.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_generated_jsx_classes() {
        let classes =
            super::jsx_classes(r#"<div className={"r123 extra"} /><span className="s456" />"#);
        assert!(classes.contains("r123"));
        assert!(classes.contains("s456"));
    }

    #[test]
    fn formats_css_declarations() {
        assert_eq!(
            super::format_css(".a{color:red;background:white;}"),
            ".a {\n  color:red;\n  background:white;\n}\n"
        );
    }
}
