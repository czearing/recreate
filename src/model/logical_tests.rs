use super::{Physical, WritingMode, physical_property};

fn named(mode: WritingMode, rtl: bool, name: &str) -> String {
    match physical_property(mode, rtl, name) {
        Physical::Named(physical) => physical,
        other => panic!("{name} under {mode:?} rtl={rtl} resolved to {other:?}"),
    }
}

/// The declaration this whole function exists for. Two spellings of one meaning under the
/// default writing mode must reach the emitter as the same physical name, because the
/// authored value cannot be recovered from a sample: `getComputedStyle` resolves a margin
/// percentage to its used pixel, so a dropped `margin-inline-start: 12%` is replaced by a
/// constant that is right at the captured width and wrong at every other one.
#[test]
fn a_logical_box_property_resolves_to_the_physical_one_it_is_synonymous_with() {
    let mode = WritingMode::default();
    for (logical, physical) in [
        ("margin-inline-start", "margin-left"),
        ("margin-inline-end", "margin-right"),
        ("margin-block-start", "margin-top"),
        ("margin-block-end", "margin-bottom"),
        ("padding-inline-start", "padding-left"),
        ("padding-block-end", "padding-bottom"),
        ("border-inline-start-width", "border-left-width"),
        ("border-block-end-color", "border-bottom-color"),
        ("scroll-margin-inline-start", "scroll-margin-left"),
    ] {
        assert_eq!(named(mode, false, logical), physical, "{logical}");
        assert_eq!(
            physical_property(mode, false, physical),
            Physical::Same,
            "{physical}"
        );
    }
}

/// The inset family drops its prefix instead of keeping it. A grammar that renamed only
/// the axis and edge would emit `inset-left`, which no engine implements, so the
/// declaration would survive translation and still be discarded downstream.
#[test]
fn the_inset_family_resolves_to_a_bare_edge() {
    for (logical, physical) in [
        ("inset-inline-start", "left"),
        ("inset-inline-end", "right"),
        ("inset-block-start", "top"),
        ("inset-block-end", "bottom"),
    ] {
        assert_eq!(named(WritingMode::default(), false, logical), physical);
    }
}

/// `direction` reaches the inline axis and nothing else. A resolver that applied it to
/// both axes would flip top and bottom on every right-to-left page, which is a mirroring
/// no author asked for and which no sample would contradict at the captured width.
#[test]
fn direction_reverses_the_inline_axis_and_leaves_the_block_axis_alone() {
    let mode = WritingMode::default();
    assert_eq!(named(mode, true, "margin-inline-start"), "margin-right");
    assert_eq!(named(mode, true, "margin-inline-end"), "margin-left");
    assert_eq!(named(mode, true, "margin-block-start"), "margin-top");
    assert_eq!(named(mode, true, "margin-block-end"), "margin-bottom");
}

/// The vertical modes agree on which axis is which and disagree on which end starts it.
/// `sideways-lr` is the trap: the sizing table groups it with the other vertical modes and
/// is right to, because both make the inline axis vertical — but its inline flow runs
/// bottom to top, so reusing that grouping for edges mirrors every box on the page.
#[test]
fn each_mode_places_the_axes_on_its_own_pair_of_edges() {
    for (keyword, block_start, inline_start) in [
        ("horizontal-tb", "margin-top", "margin-left"),
        ("vertical-rl", "margin-right", "margin-top"),
        ("vertical-lr", "margin-left", "margin-top"),
        ("sideways-rl", "margin-right", "margin-top"),
        ("sideways-lr", "margin-left", "margin-bottom"),
    ] {
        let mode = WritingMode::from(keyword.to_string());
        assert_eq!(
            named(mode, false, "margin-block-start"),
            block_start,
            "{keyword}"
        );
        assert_eq!(
            named(mode, false, "margin-inline-start"),
            inline_start,
            "{keyword}"
        );
    }
}

/// A corner names one edge from each axis, and the physical spelling always writes the
/// vertical edge first. Under a vertical writing mode the block answer IS the horizontal
/// edge, so ordering the pair by the axis that produced it emits `border-left-top-radius`,
/// a name that does not exist.
#[test]
fn a_logical_corner_is_ordered_by_the_edge_rather_than_by_the_axis() {
    assert_eq!(
        named(WritingMode::default(), false, "border-start-start-radius"),
        "border-top-left-radius"
    );
    assert_eq!(
        named(WritingMode::default(), false, "border-end-start-radius"),
        "border-bottom-left-radius"
    );
    assert_eq!(
        named(WritingMode::VerticalRl, false, "border-start-start-radius"),
        "border-top-right-radius"
    );
    assert_eq!(
        physical_property(WritingMode::default(), false, "border-top-left-radius"),
        Physical::Same
    );
}

/// The case the empty-string sentinel used to hide. A logical shorthand carries one or two
/// values across both edges of an axis, so it cannot be renamed into a single physical
/// declaration. Reporting it as already-physical hands an unimplementable name to the
/// allow-list, which rejects it without a word; reporting it as unsupported is a condition
/// this test can hold.
#[test]
fn a_logical_shorthand_over_both_edges_is_reported_rather_than_renamed() {
    for name in [
        "margin-inline",
        "margin-block",
        "padding-inline",
        "inset-block",
        "border-inline",
        "border-block-width",
        "border-inline-color",
    ] {
        assert_eq!(
            physical_property(WritingMode::default(), false, name),
            Physical::Unsupported,
            "{name}"
        );
        assert_eq!(
            physical_property(WritingMode::default(), false, name).into_name(name),
            None,
            "{name}"
        );
    }
}

/// A page authoring only physical properties must come out byte-identical, which means no
/// name that merely resembles a logical one may be rewritten.
#[test]
fn a_name_that_is_not_logical_stands_for_itself() {
    for name in [
        "margin-left",
        "padding",
        "color",
        "display",
        "border-radius",
        "text-decoration-line",
        "inset",
        "line-height",
    ] {
        for mode in [WritingMode::default(), WritingMode::SidewaysLr] {
            assert_eq!(
                physical_property(mode, true, name),
                Physical::Same,
                "{name}"
            );
            assert_eq!(
                physical_property(mode, true, name).into_name(name),
                Some(name.to_string()),
                "{name}"
            );
        }
    }
}

/// The sizing table this grammar subsumes. Its answers must survive the merge unchanged,
/// under every mode, and must keep ignoring `direction` — an axis is not an edge.
#[test]
fn the_logical_sizes_keep_resolving_as_the_sizing_table_did() {
    for mode in [
        WritingMode::default(),
        WritingMode::VerticalRl,
        WritingMode::SidewaysLr,
    ] {
        for logical in [
            "inline-size",
            "min-inline-size",
            "max-inline-size",
            "block-size",
            "min-block-size",
            "max-block-size",
        ] {
            assert_eq!(
                named(mode, false, logical),
                mode.physical_size(logical),
                "{logical}"
            );
            assert_eq!(named(mode, true, logical), mode.physical_size(logical));
        }
    }
}
