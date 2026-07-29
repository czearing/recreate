pub const SOURCE: &str = concat!("\n", include_str!("lifecycle_script.js"));

#[cfg(test)]
mod tests {
    #[test]
    fn records_rotating_text_content() {
        assert!(super::SOURCE.contains("attribute: 'textContent'"));
        assert!(super::SOURCE.contains("const lastText = new Map()"));
        assert!(super::SOURCE.contains("const trackableText = element"));
        assert!(super::SOURCE.contains("if (recordText(mutation.target, now)) continue"));
        assert!(
            super::SOURCE.contains("for (const child of node.querySelectorAll('*')) recordText")
        );
        assert!(super::SOURCE.contains("mutation.type === 'characterData'"));
        assert!(super::SOURCE.contains("for (const node of mutation.addedNodes)"));
    }

    #[test]
    fn records_full_recurring_visual_trajectory() {
        for property in [
            "backgroundColor",
            "clipPath",
            "filter",
            "height",
            "maskImage",
            "scale",
            "transformOrigin",
            "width",
        ] {
            assert!(super::SOURCE.contains(property), "missing {property}");
        }
        assert!(super::SOURCE.contains("now - start < 12000"));
    }
}
