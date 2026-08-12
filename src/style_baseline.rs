/// The one place that decides which declarations a capture records. It names no
/// property: a declaration is recorded when its value differs from what the element
/// would compute with no author CSS, measured by the engine rather than compared
/// against a table. Both capture scripts render this same source, so the rule cannot
/// drift between the resting capture and an interaction capture.
pub const SOURCE: &str = include_str!("style_baseline_script.js");

#[cfg(test)]
mod tests {
    use super::SOURCE;

    /// `initial` and `unset` both discard the user-agent origin, which the recreation
    /// still runs under. Measuring against either would re-emit every user-agent
    /// default on every node.
    #[test]
    fn measures_against_the_user_agent_origin() {
        assert!(SOURCE.contains("`${property}:revert !important`"));
        assert!(!SOURCE.contains(":initial"));
        assert!(!SOURCE.contains(":unset"));
    }

    /// `all` is specified to leave `direction` and `unicode-bidi` alone, so a baseline
    /// taken under `all: revert` reports the element's own live value back for them and
    /// the comparison prunes them at every value — `rtl` as surely as `ltr`. The
    /// comparison never had discriminating power there, so the omitted longhands have to
    /// be reverted beside the shorthand. The domain is read out of the source rather than
    /// restated here, so this asserts that the probe covers a declared set rather than
    /// that one spelling survives somewhere in a string.
    #[test]
    fn reverts_the_longhands_the_all_shorthand_omits() {
        let (_, rest) = SOURCE
            .split_once("const EXCLUDED_FROM_ALL = [")
            .expect("the probe must declare which properties `all` leaves alone");
        let (declared, _) = rest.split_once(']').expect("unterminated declaration");
        for property in ["direction", "unicode-bidi"] {
            assert!(
                declared.contains(property),
                "`all` does not reset {property}, so the probe must revert it: {declared}"
            );
        }
        assert!(SOURCE.contains("['all', ...EXCLUDED_FROM_ALL]"));
    }

    /// The element pass and the pseudo-element pass revert to the same origin, so they
    /// must revert the same properties. They once built that list separately, and a
    /// property added to one would have silently missed the other. Asserting that the
    /// declarations are declared once and read by both consumers is what keeps the two
    /// measurements comparable.
    #[test]
    fn both_passes_revert_through_one_declaration_list() {
        assert_eq!(
            SOURCE.matches("REVERT_TO_USER_AGENT").count(),
            3,
            "{SOURCE}"
        );
    }

    /// The property set comes from the engine's own enumeration. A named list here is
    /// the defect this module exists to remove.
    #[test]
    fn enumerates_properties_from_the_engine() {
        assert!(SOURCE.contains("for (const property of style)"));
        assert!(!SOURCE.contains("'list-style-type'"));
        assert!(!SOURCE.contains("'margin-top'"));
    }

    /// Reverting a whole document at once would collapse every inherited baseline to
    /// the user-agent default, so each descendant of a styled ancestor would re-record
    /// the inherited value. Depth batching is what keeps inherited values pruned.
    #[test]
    fn reverts_one_depth_level_at_a_time() {
        assert!(SOURCE.contains("collect(root, 0"));
        assert!(SOURCE.contains("levels[depth] = levels[depth] || []"));
        assert!(SOURCE.contains("for (const level of levels)"));
        assert!(!SOURCE.contains("querySelectorAll('*')"));
    }

    /// A baseline pass that does not put the page back leaves every later stage
    /// reading a stripped document.
    #[test]
    fn restores_the_style_attribute_it_overwrote() {
        assert!(SOURCE.contains("element.getAttribute('style')"));
        assert!(SOURCE.contains("removeAttribute('style')"));
        assert!(SOURCE.contains("sheet.remove()"));
    }
}

/// Colour-tracking properties are found by measurement, not by name. The capture must
/// compare each property against `color` in both the live and the baseline map; any
/// spelling of `border-top-color` or `caret-color` in the source would be the same
/// criterion-free list this work removed.
#[test]
fn colour_tracking_properties_are_measured_against_color() {
    assert!(SOURCE.contains("live[property] === live.color"), "{SOURCE}");
    assert!(
        SOURCE.contains("baseline[property] === baseline.color"),
        "{SOURCE}"
    );
    for name in [
        "caret-color",
        "outline-color",
        "border-top-color",
        "currentcolor",
    ] {
        assert!(!SOURCE.contains(name), "{name} is named in {SOURCE}");
    }
}

/// Logical aliases are recognised by their flow-relative name segments, not by a table
/// of alias/physical pairs. `border-end-end-radius` duplicates
/// `border-bottom-right-radius` and uses neither `inline` nor `block` to say so.
#[test]
fn logical_aliases_are_recognised_by_flow_relative_segments() {
    for segment in ["'inline'", "'block'", "'start'", "'end'"] {
        assert!(SOURCE.contains(segment), "{segment} missing from {SOURCE}");
    }
    let code = SOURCE
        .split("/*")
        .map(|part| part.split_once("*/").map_or(part, |(_, code)| code))
        .collect::<String>();
    for pair in ["padding-left", "border-bottom-right-radius", "inline-size"] {
        assert!(!code.contains(pair), "{pair} is named in code: {code}");
    }
}
