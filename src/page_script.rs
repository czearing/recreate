use crate::{asset_script, capture::authored_sheets::AuthoredSheet, style_baseline};
const CAPTURE: &str = include_str!("page_capture.js");

/// The stylesheet text the page is not allowed to read for itself, injected as data so the
/// capture script keeps its single top-level form. Wrapping the script in another function
/// instead would change what it evaluates to, and the failure is silent.
pub fn source_with_sheets(sheets: &[AuthoredSheet]) -> String {
    with_sheets(&source_template(), sheets)
}

fn with_sheets(template: &str, sheets: &[AuthoredSheet]) -> String {
    template.replace(
        "__AUTHORED_SHEETS__",
        &serde_json::to_string(sheets).unwrap_or_else(|_| "[]".into()),
    )
}

fn source_template() -> String {
    template_with_assets(&asset_script::with_downloads())
}

/// One template, so a capture stage added here cannot be missing from the asset-free form.
fn template_with_assets(assets: &str) -> String {
    CAPTURE
        .replace("__STYLE_BASELINE__", &style_baseline::source())
        .replace(
            "__ASSET_ATTRIBUTES__",
            &crate::asset_attributes::js_source(),
        )
        .replace("__STATE_STYLE_CAPTURE__", crate::state_style_script::SOURCE)
        .replace(
            "__BLOCKING_OVERLAY__",
            &crate::blocking_overlay::js_predicate(),
        )
        .replace("__RULE_ACTIVATION__", crate::rule_activation_script::SOURCE)
        .replace(
            "__ATTRIBUTE_SEQUENCE_CAPTURE__",
            &crate::attribute_sequence_script::source(),
        )
        .replace("__ASSET_CAPTURE__", assets)
}

pub fn source_without_assets() -> String {
    with_sheets(
        &template_with_assets(&asset_script::without_downloads()),
        &[],
    )
}

#[cfg(test)]
#[path = "page_script_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "page_script_producer_tests.rs"]
mod producer_tests;
