/// Recovers the source application's own component name from a captured class
/// attribute.
///
/// Only scoped-class conventions carry component identity. CSS Modules compile
/// to `[name]__[local]___[hash]`, so the module name survives in the DOM.
/// styled-components (`sc-jrsJCI`), Emotion (`css-1a2b3c4`) and utility
/// frameworks (`bg-blue-500`) carry none, and this returns `None` for them so
/// the caller falls back to a structural name instead of inventing one.
///
/// Nothing here matches any particular site's vocabulary: the only inputs are
/// the delimiter conventions the bundlers themselves document.
pub fn from_class(class: &str) -> Option<String> {
    class.split_whitespace().filter_map(module_name).next_back()
}

fn module_name(token: &str) -> Option<String> {
    let prefix = token.split("__").next().filter(|prefix| *prefix != token)?;
    let segments = prefix
        .split(['-', '_'])
        .filter(|segment| !noise(segment))
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return None;
    }
    let name = segments
        .iter()
        .rfind(|segment| pascal_case(segment))
        .map(|segment| (*segment).to_string())
        .unwrap_or_else(|| segments.iter().map(|segment| capitalize(segment)).collect());
    (name.chars().count() >= 2 && !hash_like(&name)).then_some(name)
}

/// Path and build noise that appears in scoped names but never identifies a
/// component: bundler markers, directory names, and opaque hashes.
fn noise(segment: &str) -> bool {
    segment.is_empty()
        || matches!(
            segment.to_ascii_lowercase().as_str(),
            "module" | "modules" | "src" | "lib" | "packages" | "styles" | "style" | "index" | "css"
        )
        || hash_like(segment)
}

fn hash_like(segment: &str) -> bool {
    let has_digit = segment.chars().any(|character| character.is_ascii_digit());
    let has_letter = segment.chars().any(char::is_alphabetic);
    (has_digit && has_letter && segment.len() >= 5)
        || segment.chars().all(|character| character.is_ascii_digit())
}

fn pascal_case(segment: &str) -> bool {
    segment
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
        && segment.chars().any(|character| character.is_lowercase())
}

fn capitalize(segment: &str) -> String {
    let mut characters = segment.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + characters.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::from_class;

    #[test]
    fn reads_css_modules_names_across_bundler_conventions() {
        assert_eq!(
            from_class("------packages-new-office-ux-src-NotebookCard-NotebookCard-module__createCard-fs8I4M"),
            Some("NotebookCard".into())
        );
        assert_eq!(
            from_class("Button_primary__jhu84"),
            Some("Button".into())
        );
        assert_eq!(
            from_class("SegmentedControl-module__root___2Kj3f"),
            Some("SegmentedControl".into())
        );
    }

    #[test]
    fn falls_back_to_the_whole_block_when_no_segment_is_pascal_case() {
        assert_eq!(
            from_class("notebook-card__title"),
            Some("NotebookCard".into())
        );
    }

    #[test]
    fn rejects_class_conventions_that_carry_no_component_identity() {
        assert_eq!(from_class("sc-jrsJCI"), None);
        assert_eq!(from_class("css-1a2b3c4"), None);
        assert_eq!(from_class("bg-blue-500 flex mt-3"), None);
        assert_eq!(from_class("_button_8v9ez_1"), None);
        assert_eq!(from_class(""), None);
    }

    #[test]
    fn prefers_the_last_scoped_name_so_application_styles_outrank_library_styles() {
        assert_eq!(
            from_class("Card-module__root-a1 NotebookCard-module__createCard-b2"),
            Some("NotebookCard".into())
        );
    }
}
