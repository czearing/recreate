//! Assembling the reach harness, in one place.
//!
//! Every test that drives the shipped script substitutes the same placeholder set, and the
//! set grows as the script learns to distinguish another platform value. Assembled at each
//! call site, a newly added placeholder is silently missed by the older callers: the token
//! reaches Node as a literal string, and the failure surfaces as a wrong URL far from the
//! call that forgot it. One owner makes the set impossible to under-substitute.
//!
//! The location is fixed and the document base is a parameter, because that is the shape of
//! the distinction being modelled: a page's location is a given, and `<base href>` is what
//! moves the base away from it.

use crate::node_eval;
use serde_json::Value;

const HARNESS: &str = include_str!("asset_attributes_reach_harness.js");

pub(super) const LOCATION: &str = "http://rig.test:59700/page.html";
pub(super) const ORIGIN: &str = "http://rig.test:59700/";

/// Walks `tree` exactly as the capture does, resolving against `base` as the document base
/// URL, and returns the harness's report.
pub(super) fn walk(tree: &Value, css_rules: &Value, base: &str) -> Value {
    node_eval::json(
        &HARNESS
            .replace("__ASSET_ATTRIBUTES__", &super::js_source())
            .replace("__TREE__", &tree.to_string())
            .replace("__CSS_RULES__", &css_rules.to_string())
            .replace("__DOCUMENT_BASE__", base)
            .replace("__BASE__", LOCATION),
    )
}

/// The URLs the walk queued for download.
pub(super) fn assets(result: &Value) -> Vec<String> {
    result["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect()
}
