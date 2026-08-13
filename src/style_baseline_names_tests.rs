//! Which property names the style record keeps, and which it drops as a logical duplicate.
//!
//! The probe records the physical spelling of every logical alias the engine enumerates beside
//! it, because the two carry one value and recording both writes one declaration twice. That is
//! safe only where a physical twin is actually present in the same enumeration. These tests
//! drive the real `styleMap` over an enumeration built to contain both kinds of name at once,
//! so the partition is pinned from the gate rather than from a record assembled past it.

use crate::style_baseline_double::evaluate;

/// Every property name the tests below classify, and the enumeration order the engine uses.
const ENUMERATED: &[&str] = &[
    // Twinless: `start`/`end` name the two ends of a grid line range, not two edges of a flow
    // axis. There is no `grid-column-left`, so nothing else carries the value.
    "grid-column-start",
    "grid-column-end",
    "grid-row-start",
    "grid-row-end",
    // Twinless: SVG paint markers, where `start`/`end` are the ends of a path.
    "marker-start",
    "marker-end",
    // Twinless: scroll-driven animation timeline positions.
    "animation-range-start",
    "animation-range-end",
    // Logical aliases, each enumerated beside the physical longhand that carries its value.
    "padding-inline-start",
    "inline-size",
    "inset-inline-start",
    "margin-block-end",
    "border-inline-start-width",
    "overflow-block",
    "scroll-padding-inline-end",
    // Logical corners. The only logical names with no axis segment, so they are the one shape a
    // rule keyed on `inline`/`block` alone would wrongly keep. All four are asserted, because a
    // rule that derived fewer than the full cross product would still satisfy any two of them.
    "border-start-start-radius",
    "border-start-end-radius",
    "border-end-start-radius",
    "border-end-end-radius",
    // The physical twins themselves, which must always survive.
    "padding-left",
    "width",
    "left",
    "border-bottom-right-radius",
];

/// Runs the real `styleMap` over `ENUMERATED` and answers with the names it kept.
fn kept() -> Vec<String> {
    let names = serde_json::to_string(ENUMERATED).expect("names are strings");
    let value = evaluate(
        &format!(
            "globalThis.enumerated = {names};\n\
             globalThis.declaration = {{\n\
               *[Symbol.iterator](){{ yield* globalThis.enumerated; }},\n\
               getPropertyValue(property){{ return `v:${{property}}`; }}\n\
             }};"
        ),
        "eval(SCRIPT + '\\nObject.keys(styleMap(globalThis.declaration))')",
    );
    serde_json::from_value(value).expect("styleMap answers with an object")
}

/// The defect. A name is dropped on the premise that a physical twin carries its value, so a
/// name with no twin must be kept whatever its segments spell. `start` and `end` are reused by
/// css-grid line placement, by SVG markers and by scroll-driven animation ranges, and in none of
/// the three do they name a flow-relative edge.
#[test]
fn keeps_every_name_that_reuses_a_flow_segment_without_a_physical_twin() {
    let kept = kept();
    for property in [
        "grid-column-start",
        "grid-column-end",
        "grid-row-start",
        "grid-row-end",
        "marker-start",
        "marker-end",
        "animation-range-start",
        "animation-range-end",
    ] {
        assert!(
            kept.iter().any(|name| name == property),
            "{property} was dropped as a logical duplicate, but no physical name carries it: \
             {kept:?}"
        );
    }
}

/// The other half of the partition, which the repair must not trade away. Dropping the prune
/// entirely would satisfy the test above and write every box declaration twice, so a logical
/// alias enumerated beside its physical longhand still has to go.
#[test]
fn still_drops_every_logical_alias_that_has_a_physical_twin() {
    let kept = kept();
    for property in [
        "padding-inline-start",
        "inline-size",
        "inset-inline-start",
        "margin-block-end",
        "border-inline-start-width",
        "overflow-block",
        "scroll-padding-inline-end",
    ] {
        assert!(
            !kept.iter().any(|name| name == property),
            "{property} was recorded beside the physical longhand carrying the same value: \
             {kept:?}"
        );
    }
}

/// The flow-relative corners are the only logical names with no axis segment, so a rule narrowed
/// to `inline`/`block` alone keeps them and writes every rounded corner twice. They are a closed
/// set of four, fixed by the spec, because a corner is named by its block edge then its inline
/// edge and there is no fifth combination of two edges.
#[test]
fn still_drops_every_logical_corner_that_names_two_flow_edges_and_no_axis() {
    let kept = kept();
    for property in [
        "border-start-start-radius",
        "border-start-end-radius",
        "border-end-start-radius",
        "border-end-end-radius",
    ] {
        assert!(
            !kept.iter().any(|name| name == property),
            "{property} was recorded beside `border-*-*-radius`, writing one corner twice: \
             {kept:?}"
        );
    }
}

/// The physical spellings the prune exists to prefer. If one of these were ever dropped the
/// value would be gone from the record entirely, which is the failure the prune trades against.
#[test]
fn keeps_the_physical_longhand_every_logical_alias_resolves_to() {
    let kept = kept();
    for property in [
        "padding-left",
        "width",
        "left",
        "border-bottom-right-radius",
    ] {
        assert!(
            kept.iter().any(|name| name == property),
            "{property} is the name the prune prefers and it was dropped: {kept:?}"
        );
    }
}
