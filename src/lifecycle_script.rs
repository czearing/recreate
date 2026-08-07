const RECORDER: &str = concat!("\n", include_str!("lifecycle_script.js"));

pub fn source() -> String {
    RECORDER.replace(
        "__LIFECYCLE_SETTLE__",
        crate::lifecycle_settle_script::SOURCE,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn records_rotating_text_content() {
        let source = super::source();
        assert!(source.contains("attribute: 'textContent'"));
        assert!(source.contains("const lastText = new Map()"));
        assert!(source.contains("const trackableText = element"));
        assert!(source.contains("if (recordText(mutation.target, now)) continue"));
        assert!(source.contains("mutation.type === 'characterData'"));
        assert!(source.contains("for (const child of node.querySelectorAll('*')) recordText"));
        assert!(source.contains("for (const node of mutation.addedNodes)"));
    }

    #[test]
    fn records_full_recurring_visual_trajectory() {
        let source = super::source();
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
            assert!(source.contains(property), "missing {property}");
        }
    }

    /// The recorder must carry a settle decision rather than run to a constant, and its
    /// keyframe offsets must be resolved against however long it actually ran.
    #[test]
    fn the_recording_window_is_measured_and_only_ceilinged() {
        let source = super::source();
        assert!(!source.contains("__LIFECYCLE_SETTLE__"));
        assert!(source.contains("lifecycleSettled(now - start, now - lastChange, busy)"));
        assert!(source.contains("const LIFECYCLE_CEILING_MS = 12000"));
        assert!(!source.contains("(now - start) / 12000"));
        assert!(!source.contains("now - start < 12000"));
    }
}
