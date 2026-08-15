//! The single owner of what a difference between two style maps is allowed to say.
//!
//! Two maps of the same element - one at the base viewport and one in a band, or one in
//! the baseline DOM and one in an interaction state - are compared to produce the
//! declarations that express the second in terms of the first. The comparison has to
//! account for three outcomes, not two: a key can change, appear, or **go back to the
//! value it would have with no author declaration at all**.
//!
//! That third outcome is invisible from the current map alone. The capture stores
//! `authoredStyles(styleMap(...), baselineOf(...))`, which drops every value equal to
//! what the element computes under `all: revert`, so a reverted key is missing from
//! exactly the side a one-sided loop iterates. Omission is not neutral in a cascade: an
//! unmentioned property keeps whatever the base rule said, so a silent difference
//! asserts the value the source withdrew.
//!
//! `revert` is not an approximation of that value, it is the same measurement spelled as
//! a declaration - the capture decided the key was droppable by applying `all: revert`
//! and reading the result. `initial` would give the specification's value, so `display`
//! on a `div` would read `inline`; `unset` never revives the user-agent origin. Only
//! `revert` lands on the origin the recreation also runs under, and it does so by
//! cancelling the whole author origin, which is what stops the base declaration from
//! winning inside the band.
//! A reset cancels a declaration, so the set it is computed against is the set the base
//! rule actually emits, not everything the capture measured. `declared` applies the same
//! sample removal the base rule applies, because a measurement the emitter refused to
//! write out has nothing for a reset to cancel and a reset for it is inert output.
use super::responsive::samples::{remove_resettable_sizes, remove_sampled_origins};
use crate::model::Styles;

/// How a reversion is spelled. A CSS-wide keyword, so it is valid for every property and
/// for both consumers - a declaration in a rule and a value passed to `setProperty`.
pub(in crate::generate) const REVERTED: &str = "revert";

/// What a rule built from `style` actually declares, as opposed to what the capture
/// measured. A reset cancels a declaration, so a key that never reached the output as one
/// has nothing for a reset to cancel and a reset for it is inert.
pub(in crate::generate) fn declared(style: &Styles) -> Styles {
    let mut declared = style.clone();
    remove_resettable_sizes(&mut declared);
    remove_sampled_origins(&mut declared, style);
    declared
}

/// The keys `current` says nothing about because their value returned to the element's
/// baseline. Name-based pruning in the capture is viewport- and state-invariant, so this
/// is the only way a key can be present on one side and absent on the other.
pub(in crate::generate) fn reverted<'a>(
    declared: &'a Styles,
    current: &'a Styles,
) -> impl Iterator<Item = &'a String> {
    declared
        .keys()
        .filter(move |key| !current.contains_key(*key))
}

/// Every declaration needed to express `current` given that `base` is already in force.
/// A key whose value did not move emits nothing, so the result stays a difference rather
/// than a restatement of the whole style map.
pub(in crate::generate) fn changed_styles(base: &Styles, current: &Styles) -> Styles {
    current
        .iter()
        .filter(|(key, value)| base.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// The declarations `changed` still makes that `base` already makes in the same words.
///
/// `changed_styles` compares the two maps as the capture recorded them, but a normalizer
/// runs afterwards and rewrites a value into the spelling the author used - and, where the
/// author declared a property the capture never had to record as a difference, introduces
/// one. Both sides of the comparison would be rewritten identically, so the result is a
/// property that did not move restated inside every band. Comparing once more against the
/// base put through the same normalizers is what keeps the difference a difference in
/// spelling as well as in value.
pub(in crate::generate) fn remove_restatements(changed: &mut Styles, base: &Styles) {
    changed.retain(|key, value| base.get(key) != Some(&*value));
}

/// Applied after every normalizer, so that no later stage can substitute the value the
/// reset exists to withdraw - the same reason sample removal runs last on the base rule.
pub(in crate::generate) fn append_reversions(
    changed: &mut Styles,
    declared: &Styles,
    current: &Styles,
) {
    for key in reverted(declared, current).cloned().collect::<Vec<_>>() {
        changed.insert(key, REVERTED.to_string());
    }
}

#[cfg(test)]
#[path = "style_delta_tests.rs"]
mod tests;
