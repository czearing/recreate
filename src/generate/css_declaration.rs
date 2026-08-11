//! The single owner of "what does this declaration block declare?".
//!
//! Several stages need the property names a block sets — one to decide which authored rule won,
//! one to decide whether two rules can disagree — and a second private copy of the parse is how
//! the two would drift apart.
//!
//! The split is deliberately naive, and its inaccuracy is one-directional. A block carrying a
//! `data:` URL contains both separators inside a value, so a fragment of that value can be read
//! as a further declaration. Such a fragment can only ever *add* a property name, never remove
//! one, and every caller here is one for which an extra name costs a missed optimisation while a
//! missing name would cost correctness.

use std::collections::BTreeSet;

/// The declarations in a block, as trimmed name and value.
pub fn parsed(block: &str) -> impl DoubleEndedIterator<Item = (&str, &str)> {
    block
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .map(|(name, value)| (name.trim(), value.trim()))
}

/// The property names a block sets, which is what decides whether two blocks can disagree.
pub fn properties(block: &str) -> BTreeSet<String> {
    parsed(block).map(|(name, _)| name.to_string()).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_the_property_names_a_block_sets() {
        let properties = super::properties("color: red; letter-spacing: 2px;");

        assert_eq!(properties.len(), 2);
        assert!(properties.contains("color"));
        assert!(properties.contains("letter-spacing"));
    }

    /// A value carrying the separators must not cost the property it belongs to. Losing
    /// `background` here would let a rule that does set it be judged unable to disagree.
    #[test]
    fn keeps_a_property_whose_value_contains_the_separators() {
        let properties = super::properties("background: url(data:image/gif;base64,AAAA);");

        assert!(
            properties.contains("background"),
            "the property was lost to its own value: {properties:?}"
        );
    }
}
