use anyhow::Result;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

/// True for the hashed class names this generator mints, which `structural_css` and
/// `css_base` write as a single letter followed by ten hexadecimal digits.
pub fn generated_class(value: &str) -> bool {
    matches!(value.as_bytes().first(), Some(b'r' | b's'))
        && value.len() == 11
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

/// Yields `(offset, body)` for every double quoted string in generated source, where
/// `offset` is the position of the opening quote.
///
/// Every string the generator emits is written by `serde_json::to_string`, which escapes
/// an embedded quote as `\"` and a backslash as `\\`. Reading them back by splitting on
/// the quote character assumes quotes strictly alternate, so a single quote in captured
/// page text would invert the parity of every literal after it in the file. Sharing the
/// writer's escaping rule is what keeps the reader's answer independent of what the page
/// happens to say.
pub fn string_literals(source: &str) -> impl Iterator<Item = (usize, &str)> {
    let bytes = source.as_bytes();
    let mut index = 0;
    std::iter::from_fn(move || {
        while index < bytes.len() && bytes[index] != b'"' {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let quote = index;
        let mut end = quote + 1;
        while end < bytes.len() && bytes[end] != b'"' {
            end += if bytes[end] == b'\\' { 2 } else { 1 };
        }
        index = end.saturating_add(1).min(bytes.len());
        end = end.min(bytes.len());
        while end < bytes.len() && !source.is_char_boundary(end) {
            end += 1;
        }
        Some((quote, &source[quote + 1..end]))
    })
}

/// Every class name the generated source binds to an element: the whole `className` value
/// wherever one is written, and generated class names carried inside serialized props.
pub fn jsx_classes(source: &str) -> HashSet<String> {
    let mut classes = HashSet::new();
    for (quote, value) in string_literals(source) {
        let attribute = source[..quote]
            .trim_end_matches('{')
            .ends_with("className=");
        classes.extend(
            value
                .split_whitespace()
                .filter(|token| attribute || generated_class(token))
                .map(str::to_string),
        );
    }
    classes
}

/// The generated files that bind class names to elements. `main.jsx` is deliberately
/// absent: it names classes only to rewrite the stylesheet, never to render one.
pub fn jsx_files(source: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![source.join("App.jsx"), source.join("states.jsx")];
    for directory in ["components", "states", "views"] {
        let directory = source.join(directory);
        if directory.exists() {
            collect_jsx(&directory, &mut files)?;
        }
    }
    files.sort();
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

#[cfg(test)]
mod tests {
    use super::{jsx_classes, string_literals};

    #[test]
    fn finds_generated_jsx_classes() {
        let classes = jsx_classes(r#"<div className={"r123 extra"} /><span className="s456" />"#);
        assert!(classes.contains("r123"));
        assert!(classes.contains("s456"));
        assert!(classes.contains("extra"));
    }

    #[test]
    fn reads_classes_after_an_escaped_quote_in_serialized_text() {
        let source = r#"<Surface entries={[["a","r0000000001"]]} texts={[["a","say \"hi"]]}/><Surface entries={[["b","r0000000002"]]} texts={[["b","plain"]]}/>"#;
        let classes = jsx_classes(source);
        assert!(classes.contains("r0000000001"), "positive control");
        assert!(
            classes.contains("r0000000002"),
            "a quote in one surface's text must not hide a later surface's class"
        );
    }

    #[test]
    fn class_discovery_does_not_depend_on_escaped_characters_in_text() {
        let template =
            r#"<Surface texts={[["a","TEXT"]]}/><Surface entries={[["b","s0000000004"]]}/>"#;
        let plain = jsx_classes(&template.replace("TEXT", "plain"));
        assert!(plain.contains("s0000000004"));
        for payload in [
            r#"one \" quote"#,
            r#"two \" \" quotes"#,
            r#"a \\ backslash"#,
            r#"trailing backslash \\"#,
        ] {
            assert_eq!(
                jsx_classes(&template.replace("TEXT", payload)),
                plain,
                "the owner set must not depend on {payload}"
            );
        }
    }

    #[test]
    fn reads_classes_after_an_escaped_quote_in_a_state_attribute() {
        let source = r#"<Surface attributes={[["a",[["title","say \"hi"]]]]}/><Surface entries={[["b","s0000000005"]]}/>"#;
        assert!(jsx_classes(source).contains("s0000000005"));
    }

    #[test]
    fn reads_a_class_name_written_after_quoted_page_text() {
        let source = r#"{"he said \"go"}<div className={"r00000000ff"} />"#;
        assert!(jsx_classes(source).contains("r00000000ff"));
    }

    #[test]
    fn treats_an_escaped_quote_as_string_content_not_a_delimiter() {
        let bodies = string_literals(r#"a="one \" two" b="three""#)
            .map(|(_, body)| body)
            .collect::<Vec<_>>();
        assert_eq!(bodies, vec![r#"one \" two"#, "three"]);
    }

    #[test]
    fn keeps_multibyte_text_intact() {
        let bodies = string_literals("\"caf\\u00e9 — dash\" \"next\"")
            .map(|(_, body)| body)
            .collect::<Vec<_>>();
        assert_eq!(bodies, vec!["caf\\u00e9 — dash", "next"]);
    }

    #[test]
    fn returns_jsx_files_in_stable_path_order() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(directory.path().join("components/z")).unwrap();
        std::fs::create_dir_all(directory.path().join("components/a")).unwrap();
        for name in ["App.jsx", "states.jsx"] {
            std::fs::write(directory.path().join(name), "").unwrap();
        }
        std::fs::write(directory.path().join("components/z/Z.jsx"), "").unwrap();
        std::fs::write(directory.path().join("components/a/A.jsx"), "").unwrap();
        let files = super::jsx_files(directory.path()).unwrap();
        assert!(files.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
