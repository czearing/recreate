use super::source_item_dedupe::{GeneratedItem, extract};
use std::collections::BTreeSet;

/// Four instances of one repeated item, differing only in the meta line. Every other literal is
/// deliberately identical, so the meta line is the sole candidate varying field: give the cards
/// distinct titles and the extraction guard is satisfied for the wrong reason and the fixture
/// stops testing anything.
fn board(metas: [&str; 4]) -> String {
    metas
        .iter()
        .map(|meta| {
            format!(
                "  <article data-testid={{\"task-card\"}} className={{\"r71a6eeabc0\"}}>\n\
                 \x20   <h3 className={{\"r1a2b3c4d5e\"}}>\n\
                 \x20     {{\"Review the quarterly capacity plan\"}}\n\
                 \x20   </h3>\n\
                 \x20   <p className={{\"r9f8e7d6c5b\"}}>\n\
                 \x20     {{\"{meta}\"}}\n\
                 \x20   </p>\n\
                 \x20 </article>"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn lift(metas: [&str; 4]) -> (String, Vec<GeneratedItem>) {
    let mut source = board(metas);
    let generated = extract(&mut [&mut source], &BTreeSet::new());
    (source, generated)
}

const COINCIDENT: [&str; 4] = ["2 minutes ago"; 4];
const DISTINCT: [&str; 4] = [
    "2 minutes ago",
    "5 minutes ago",
    "9 minutes ago",
    "14 minutes ago",
];

fn occurrences(source: &str, needle: &str) -> usize {
    source.matches(needle).count()
}

/// The metamorphic relation, and the whole point of the pair: changing only the VALUES inside a
/// repeated item must not move the emitted STRUCTURE. Whether the item is a component is a
/// property of its shape, which both members share; only the payload at the call sites may
/// differ. Asserted in both directions so neither member can pass by extracting nothing.
#[test]
fn lifts_a_repeated_item_whose_values_coincide_exactly_as_one_whose_values_differ() {
    let (coincident, coincident_items) = lift(COINCIDENT);
    let (distinct, distinct_items) = lift(DISTINCT);

    assert_eq!(coincident_items.len(), 1, "coincident emitted no component");
    assert_eq!(distinct_items.len(), 1);
    assert_eq!(coincident_items[0].name, distinct_items[0].name);
    assert_eq!(
        coincident_items[0].source, distinct_items[0].source,
        "the component must not depend on the values captured"
    );

    for source in [&coincident, &distinct] {
        assert_eq!(occurrences(source, "<CollectionItems."), 4);
        assert_eq!(occurrences(source, "data-testid={\"task-card\"}"), 0);
    }

    // The payload is the only licensed difference: normalising it collapses the twins.
    let normalized = DISTINCT.iter().fold(distinct.clone(), |source, meta| {
        source.replace(&format!("{{\"{meta}\"}}"), "{\"META\"}")
    });
    let expected = coincident.replace("{\"2 minutes ago\"}", "{\"META\"}");
    assert_eq!(normalized, expected);
}

/// The value the coincident twin carries must survive as a prop at each call site rather than
/// being frozen into the component, or the component could not render the page it came from.
#[test]
fn passes_a_coinciding_value_as_a_prop_rather_than_inlining_it() {
    let (source, items) = lift(COINCIDENT);
    assert_eq!(occurrences(&source, "updatedTime={\"2 minutes ago\"}"), 4);
    assert!(
        !items[0].source.contains("2 minutes ago"),
        "the captured value was baked into the component"
    );
    assert!(items[0].source.contains("{updatedTime}"));
}

/// A field that differs between instances must reach the props even when it is not the kind of
/// field the semantic rule selects. Generated class names are excluded from that rule because a
/// constant one is noise, but a per-instance one is the instance's own identity: inlining it would
/// freeze the first card's appearance onto all four.
#[test]
fn parameterises_a_field_that_varies_without_being_semantic() {
    let classes = ["ra1a1a1a1a1", "rb2b2b2b2b2", "rc3c3c3c3c3", "rd4d4d4d4d4"];
    let mut source = classes
        .iter()
        .map(|class| {
            format!(
                "  <article data-testid={{\"task-card\"}} className={{\"{class}\"}}>\n\
                 \x20   <h3 className={{\"r1a2b3c4d5e\"}}>\n\
                 \x20     {{\"Review the quarterly capacity plan\"}}\n\
                 \x20   </h3>\n\
                 \x20   <p className={{\"r9f8e7d6c5b\"}}>\n\
                 \x20     {{\"2 minutes ago\"}}\n\
                 \x20   </p>\n\
                 \x20 </article>"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let generated = extract(&mut [&mut source], &BTreeSet::new());

    assert_eq!(generated.len(), 1);
    for class in classes {
        assert_eq!(
            occurrences(&source, &format!("{{\"{class}\"}}")),
            1,
            "{class} did not reach its own call site"
        );
        assert!(
            !generated[0].source.contains(class),
            "{class} was frozen into the component"
        );
    }
}

/// A repeated item carrying exactly one prop is still worth lifting: four copies of a row become
/// one component and four values. Only a group with nothing at all to parameterise is declined, so
/// the guard is a test for emptiness and not a threshold that a minimal row could fall under.
#[test]
fn lifts_a_repeated_item_that_yields_a_single_prop() {
    let mut source = ["Backlog", "Backlog", "Backlog"]
        .iter()
        .map(|label| {
            format!(
                "  <article data-testid={{\"task-card\"}} className={{\"r71a6eeabc0\"}}>\n\
                 \x20   <i className={{\"r5c5c5c5c5c\"}} aria-hidden={{\"true\"}} />\n\
                 \x20   <span className={{\"r1a2b3c4d5e\"}} data-role={{\"label\"}}>\n\
                 \x20     {{\"{label}\"}}\n\
                 \x20   </span>\n\
                 \x20 </article>"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let generated = extract(&mut [&mut source], &BTreeSet::new());

    assert_eq!(generated.len(), 1, "a single-prop item was declined");
    assert_eq!(occurrences(&source, "<CollectionItems."), 3);
    assert!(!generated[0].source.contains("Backlog"));
}

/// The guard being relaxed is not vestigial: it stops a parameterless component, which is only a
/// renamed copy of the block. A repeated item whose every literal is a generated class name has
/// nothing to parameterise, so it must still be left inline.
#[test]
fn declines_a_repeated_item_with_nothing_to_parameterise() {
    let block = "  <article data-testid={\"task-card\"} className={\"r71a6eeabc0\"}>\n\
                 \x20   <span className={\"r1a2b3c4d5e\"}>\n\
                 \x20     <i className={\"r9f8e7d6c5b\"} />\n\
                 \x20   </span>\n\
                 \x20 </article>";
    let mut source = [block, block, block].join("\n");
    let generated = extract(&mut [&mut source], &BTreeSet::new());
    assert!(generated.is_empty(), "emitted a component with no props");
    assert_eq!(occurrences(&source, "<CollectionItems."), 0);
}
