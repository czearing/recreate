use crate::{asset_script, style_contract};
const CAPTURE: &str = include_str!("page_capture.js");

pub fn source() -> String {
    CAPTURE
        .replace("__STYLE_PROPERTIES__", style_contract::PROPERTIES)
        .replace(
            "__DIRECTIONAL_BORDERS__",
            style_contract::DIRECTIONAL_BORDERS,
        )
        .replace("__STATE_STYLE_CAPTURE__", crate::state_style_script::SOURCE)
        .replace(
            "__ATTRIBUTE_SEQUENCE_CAPTURE__",
            crate::attribute_sequence_script::SOURCE,
        )
        .replace("__ASSET_CAPTURE__", asset_script::SOURCE)
}

pub fn source_without_assets() -> String {
    CAPTURE
        .replace("__STYLE_PROPERTIES__", style_contract::PROPERTIES)
        .replace(
            "__DIRECTIONAL_BORDERS__",
            style_contract::DIRECTIONAL_BORDERS,
        )
        .replace("__STATE_STYLE_CAPTURE__", crate::state_style_script::SOURCE)
        .replace(
            "__ATTRIBUTE_SEQUENCE_CAPTURE__",
            crate::attribute_sequence_script::SOURCE,
        )
        .replace("__ASSET_CAPTURE__", "const assetData = {};")
}

#[cfg(test)]
#[path = "page_script_tests.rs"]
mod tests;
