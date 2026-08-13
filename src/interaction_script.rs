use crate::style_baseline;

const FULL_SELECTION: &str = "const selected = new Set(document.querySelectorAll('*'));";

const SOURCE: &str = concat!("\n", include_str!("interaction_script.js"));

pub fn source() -> String {
    render(FULL_SELECTION)
}

fn render(selection: &str) -> String {
    SOURCE
        .replace("__STYLE_BASELINE__", &style_baseline::source())
        .replace(
            "__ASSET_ATTRIBUTES__",
            &crate::asset_attributes::js_source(),
        )
        .replace("__SELECTION__", selection)
        .replace(
            "__ASSET_CAPTURE__",
            &crate::asset_script::without_downloads(),
        )
}

#[cfg(test)]
mod tests {
    #[test]
    fn captures_complete_interaction_documents() {
        assert!(super::source().contains("new Set(document.querySelectorAll('*'))"));
        assert!(!super::SOURCE.contains("pin|delete|duplicate"));
        assert!(super::SOURCE.contains("scroll_left: element.scrollLeft"));
    }

    #[test]
    fn excludes_non_rendered_runtime_nodes() {
        assert!(
            super::SOURCE
                .contains("script,noscript,[data-recreate-startup],.recreateAnchoredSurface")
        );
    }

    #[test]
    fn selection_does_not_depend_on_generated_animations() {
        assert!(!super::SOURCE.contains("document.getAnimations"));
    }

    #[test]
    fn captures_selected_surface_assets() {
        let source = super::source();
        assert!(source.contains("const assetData = {}"));
        assert!(source.contains("asset_data: assetData"));
    }
}
