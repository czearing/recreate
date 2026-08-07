//! The single owner of "is this element a full-page blocking overlay?".
//!
//! Splash screens, consent walls and route-transition curtains all present the same shape:
//! a positioned element that covers the viewport, paints above the page, and still takes
//! pointer input. Capture has to recognise it three times — to wait for it to appear, to
//! wait for it to leave, and to decide which nodes belong to the startup layer — and the
//! rule had been written out separately at each site. The copies drifted: the one selecting
//! startup nodes had lost the pointer-events, display and visibility clauses, so it selected
//! a different set of elements than the check that decided an overlay was there at all.
//!
//! The thresholds are named once here. Capture runs in two places, in-page JavaScript and
//! Rust over an already-captured node, so the rule is rendered twice from those same
//! constants and a test holds both renderings against one fixture.

use crate::model::{Node, Viewport};

/// The share of the viewport an element must cover before it can hide the page behind it.
const AREA_RATIO: f64 = 0.9;
/// The stacking level above which an element paints over ordinary page content.
const MIN_Z_INDEX: i32 = 50;
/// The `position` values that take an element out of flow so it can cover the page.
const POSITIONS: [&str; 2] = ["absolute", "fixed"];

/// The rule as a JavaScript expression, for use inside an injected page script.
///
/// Evaluates to a function of one element. Callers name it themselves so the same source
/// can back a `find` predicate and a standalone test.
pub fn js_predicate() -> String {
    let positions = POSITIONS.map(|value| format!("'{value}'")).join(",");
    format!(
        "(element => {{\
         const rect = element.getBoundingClientRect();\
         const style = getComputedStyle(element);\
         const z = Number(style.zIndex);\
         return rect.width * rect.height >= innerWidth * innerHeight * {AREA_RATIO} &&\
         [{positions}].includes(style.position) &&\
         Number.isFinite(z) && z >= {MIN_Z_INDEX} &&\
         style.pointerEvents !== 'none' &&\
         style.display !== 'none' &&\
         style.visibility !== 'hidden';\
         }})"
    )
}

/// The rule against a node already read out of the page.
pub fn is_blocking_overlay(node: &Node, viewport: &Viewport) -> bool {
    let viewport_area = f64::from(viewport.width) * f64::from(viewport.height);
    let style = |name: &str| node.style.get(name).map(String::as_str);
    let z_index = node
        .style
        .get("z-index")
        .and_then(|value| value.parse::<i32>().ok());
    node.rect.width * node.rect.height >= viewport_area * AREA_RATIO
        && matches!(style("position"), Some("absolute" | "fixed"))
        && z_index.is_some_and(|value| value >= MIN_Z_INDEX)
        && style("pointer-events") != Some("none")
        && style("display") != Some("none")
        && style("visibility") != Some("hidden")
}

#[cfg(test)]
#[path = "blocking_overlay_tests.rs"]
mod tests;
