use super::WritingMode;

/// The whole point of recording the keyword instead of a flag. Both vertical values
/// transpose the axes identically, so a `horizontal: bool` would look sufficient here —
/// and it would erase the distinction the two carry everywhere else, since `vertical-rl`
/// and `vertical-lr` place block-start on opposite physical edges. Losing that is the
/// same collapse this fact was added to end, one level down.
#[test]
fn the_two_vertical_modes_agree_on_the_axis_and_stay_distinguishable() {
    let rl = WritingMode::from("vertical-rl".to_string());
    let lr = WritingMode::from("vertical-lr".to_string());
    assert_eq!(rl.physical_size("inline-size"), "height");
    assert_eq!(lr.physical_size("inline-size"), "height");
    assert_ne!(rl, lr);
}

/// The axes exchange places, so every logical size maps to the opposite physical family
/// from the one it maps to in horizontal text. A mapping that merely dropped the vertical
/// case, or that mapped `inline-size` to `height` while leaving `block-size` on `height`
/// too, would satisfy a test that checked only one property.
#[test]
fn a_vertical_mode_transposes_every_logical_size() {
    let vertical = WritingMode::VerticalRl;
    for (logical, horizontal_physical, vertical_physical) in [
        ("inline-size", "width", "height"),
        ("min-inline-size", "min-width", "min-height"),
        ("max-inline-size", "max-width", "max-height"),
        ("block-size", "height", "width"),
        ("min-block-size", "min-height", "min-width"),
        ("max-block-size", "max-height", "max-width"),
    ] {
        assert_eq!(
            WritingMode::default().physical_size(logical),
            horizontal_physical,
            "{logical}"
        );
        assert_eq!(
            vertical.physical_size(logical),
            vertical_physical,
            "{logical}"
        );
    }
}

/// `sideways-rl` and `sideways-lr` rotate glyphs, not axes, so they size exactly as the
/// `vertical-*` pair does. Treating them as horizontal because their name does not begin
/// with `vertical` is the mistake a substring test invites.
#[test]
fn the_sideways_modes_size_as_vertical_ones() {
    for keyword in ["sideways-rl", "sideways-lr"] {
        let mode = WritingMode::from(keyword.to_string());
        assert!(!mode.horizontal(), "{keyword}");
        assert_eq!(mode.physical_size("block-size"), "width", "{keyword}");
    }
}

/// A property that is not a logical size has no mapping under any mode. Returning a
/// physical name for one would rename an unrelated declaration.
#[test]
fn a_property_that_is_not_a_logical_size_maps_to_nothing() {
    for name in ["width", "inline-start", "padding-inline", "color"] {
        assert_eq!(WritingMode::VerticalLr.physical_size(name), "", "{name}");
        assert_eq!(WritingMode::default().physical_size(name), "", "{name}");
    }
}

/// Specs written before this fact existed carry no keyword, and a build that meets a
/// keyword it does not implement must still lay the page out. Both resolve to the initial
/// value, which is what keeps every existing capture reading identically.
#[test]
fn an_absent_or_unknown_keyword_resolves_to_the_initial_value() {
    for keyword in ["", "horizontal-tb", "tb-rl", "sideways"] {
        let mode = WritingMode::from(keyword.to_string());
        assert!(mode.horizontal(), "{keyword}");
        assert_eq!(mode, WritingMode::default(), "{keyword}");
    }
    assert_eq!(
        serde_json::from_str::<WritingMode>("\"vertical-lr\"").unwrap(),
        WritingMode::VerticalLr
    );
    assert_eq!(
        serde_json::to_string(&WritingMode::VerticalRl).unwrap(),
        "\"vertical-rl\""
    );
}
