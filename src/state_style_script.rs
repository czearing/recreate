//! The capture pass that records every authored rule and the state styles it carries.
//!
//! The body lives beside this module as JavaScript so it reads as the browser source it is;
//! the two placeholders are filled by the caller with the rule walk and the shorthand split.

pub const SOURCE: &str = include_str!("state_style_script.js");

#[cfg(test)]
mod tests {
    /// The wiring these tests exist to hold, stated as invariants rather than as the spelling
    /// of any one mechanism. What a state rule's declarations *are* is asserted behaviourally
    /// in `state_style_var_tests`, against a scripted CSSOM.
    #[test]
    fn resolves_custom_properties_in_dynamic_state_rules() {
        assert!(super::SOURCE.contains("computed.getPropertyValue(name)"));
        assert!(super::SOURCE.contains("declarations: resolveVariables(rule.style, element)"));
    }

    /// Which longhands a shorthand sets is a per-family grammar, and any spelling of it is a
    /// table of families: it answers for whoever was listed and silently withholds the rest.
    /// The block's own text needs no table, so the presence of one is the defect.
    #[test]
    fn names_no_shorthand_it_has_to_know_the_family_of() {
        for family in ["'background'", "'border'", "'padding'", "'outline'"] {
            assert!(
                !super::SOURCE.contains(family),
                "the capture enumerates shorthand families again: {family}"
            );
        }
    }
}
