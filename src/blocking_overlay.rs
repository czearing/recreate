//! The single owner of "is this element a full-page blocking overlay?".
//!
//! Splash screens, consent walls and route-transition curtains all present the same shape:
//! a positioned element that covers the viewport, paints above the page, and still takes
//! pointer input. Capture has to recognise it three times — to wait for it to appear, to
//! wait for it to leave, and to decide which nodes belong to the startup layer — and the
//! rule had been written out separately at each site.
//!
//! The rule now exists once, as this JavaScript expression, and is asked once per element
//! while the page is still open. Rust used to carry a second rendering of the same clauses
//! over the captured node, which looks like a safe duplicate and is not: `Node::style` is
//! the authored diff, so a property equal to its revert baseline is absent. Every negative
//! clause then read absence as evidence — `None != Some("hidden")` is true — and a subtree
//! hidden by an ancestor was reported as a curtain. The verdict is recorded as
//! [`crate::model::Node::blocking_overlay`] instead, for the reason `disabled` and `rtl` are.

/// The share of the viewport an element must cover before it can hide the page behind it.
const AREA_RATIO: f64 = 0.9;
/// The stacking level above which an element paints over ordinary page content.
const MIN_Z_INDEX: i32 = 50;
/// The `position` values that take an element out of flow so it can cover the page.
const POSITIONS: [&str; 2] = ["absolute", "fixed"];

/// The rule as a JavaScript expression, for use inside an injected page script.
///
/// Evaluates to a function of one element. Callers name it themselves so the same source
/// can back a `find` predicate, a per-node record and a standalone test.
///
/// Whether the element is drawn at all is asked of the engine rather than spelled out.
/// `display`, `visibility`, `opacity` and `content-visibility` each hide an element for a
/// different reason and resolve by a different rule — `visibility` inherits, `opacity`
/// composites the subtree without inheriting, `content-visibility: auto` is not a value at
/// all — so no per-element style read covers them, and enumerating them is what let an
/// `opacity: 0` ancestor slip past even against live computed style. `checkVisibility` is
/// specified to answer for the element and every one of its ancestors at once.
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
         element.checkVisibility({{\
         opacityProperty: true, visibilityProperty: true, contentVisibilityAuto: true\
         }});\
         }})"
    )
}

#[cfg(test)]
#[path = "blocking_overlay_tests.rs"]
mod tests;
