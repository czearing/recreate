use crate::style_contract;

const VISUAL_PROPERTIES: &str = concat!(
    "'display','visibility','position','width','height','overflow','overflow-x','overflow-y',",
    "'color','background-color','background-image','border','border-radius','box-shadow',",
    "'outline-color','outline-style','outline-width','outline-offset','opacity','transform',",
    "'font-family','font-size','font-weight','line-height','text-decoration','cursor',",
    "'pointer-events','fill','stroke'"
);
const FULL_SELECTION: &str = "const selected = new Set(document.querySelectorAll('*'));";
const VISUAL_SELECTION: &str = r#"
  const selected = new Set([
    document.documentElement, document.body,
    ...document.querySelectorAll(
      'a[href],button,input:not([type="hidden"]),select,textarea,summary,'+
      '[role],[aria-label],[aria-haspopup],[aria-expanded],[aria-pressed],'+
      '[aria-selected],[tabindex]:not([tabindex="-1"]),img,svg,canvas,video,audio'
    )
  ]);
  for (const element of [...selected]) {
    for (let parent = element.parentElement; parent; parent = parent.parentElement) {
      selected.add(parent);
    }
  }
"#;

const SOURCE: &str = concat!("\n", include_str!("interaction_script.js"));

pub fn source() -> String {
    render(style_contract::PROPERTIES, FULL_SELECTION)
}

pub fn visual_source() -> String {
    render(VISUAL_PROPERTIES, VISUAL_SELECTION)
}

fn render(properties: &str, selection: &str) -> String {
    SOURCE
        .replace("__STYLE_PROPERTIES__", properties)
        .replace("__SELECTION__", selection)
        .replace(
            "__DIRECTIONAL_BORDERS__",
            style_contract::DIRECTIONAL_BORDERS,
        )
        .replace("__ASSET_CAPTURE__", "const assetData = {};")
}

#[cfg(test)]
mod tests {
    #[test]
    fn captures_complete_interaction_documents() {
        assert!(super::source().contains("new Set(document.querySelectorAll('*'))"));
        assert!(!super::SOURCE.contains("pin|delete|duplicate"));
        assert!(super::SOURCE.contains("scroll_left: element.scrollLeft"));
        assert!(super::visual_source().contains("'outline-style'"));
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

    #[test]
    fn canonicalizes_control_values_as_text_evidence() {
        assert!(super::SOURCE.contains("element.matches('textarea,input')"));
        assert!(super::SOURCE.contains("document.createTextNode(element.value)"));
    }
}
