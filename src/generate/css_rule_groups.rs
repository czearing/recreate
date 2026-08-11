//! The single owner of consolidating rules that share a declaration block.
//!
//! Position is a value. When two declarations reach one element with equal importance, origin and
//! specificity, the later one wins, so a stylesheet is a sequence rather than a set. Merging two
//! rules that share a block is a real optimisation, but it moves the second rule back to where
//! the first stands, and a rule that moves back past a rule it was authored to override loses.
//!
//! The repair is not to relocate the merged group. A group carries several selectors, and moving
//! it moves all of them — including members whose authored position was earlier than the rule
//! being absorbed, which merely trades one wrong answer for another. The condition is the one a
//! CSS optimiser checks before restructuring: merge only where nothing in between can disagree.
//!
//! Where that fails the rule keeps its authored position as a new group, so the same block may be
//! written twice. That repetition is what the author asked for; collapsing it is the defect.

use std::collections::BTreeSet;

/// What makes two rules interchangeable: the state they apply in, the band they apply under, and
/// what they declare.
pub type RuleKey = (String, Option<String>, String);

struct Group {
    key: RuleKey,
    selectors: BTreeSet<String>,
    properties: BTreeSet<String>,
}

/// Rules in emission order, holding the invariant that no selector was moved back past a rule
/// that would then outrank it.
///
/// The invariant lives here rather than at the call sites because rules arrive in several passes
/// over one collection, and only the collection can see every group already recorded — which is
/// exactly what the check needs to read.
#[derive(Default)]
pub struct Groups(Vec<Group>);

impl Groups {
    /// Records `selector` under `key`, merging it into an existing group only where doing so
    /// cannot change which declaration wins for any element.
    pub fn add(&mut self, key: RuleKey, selector: String) {
        let properties = super::css_declaration::properties(&key.2);
        match self.mergeable(&key, &selector, &properties) {
            Some(index) => {
                self.0[index].selectors.insert(selector);
            }
            None => self.0.push(Group {
                key,
                selectors: BTreeSet::from([selector]),
                properties,
            }),
        }
    }

    /// The group `selector` may join, searched from the end because a new rule stands last and
    /// merging can only move it earlier.
    ///
    /// Walking backwards answers both questions in one pass. Reaching a group with the same key
    /// first means every group after it has already been cleared, so the merge is safe and lands
    /// as late as possible. Reaching a group that both matches this selector and sets a property
    /// this block sets first means the rule cannot pass it, and no earlier group is reachable
    /// either, so it keeps its own position.
    fn mergeable(
        &self,
        key: &RuleKey,
        selector: &str,
        properties: &BTreeSet<String>,
    ) -> Option<usize> {
        for (index, group) in self.0.iter().enumerate().rev() {
            if &group.key == key {
                return Some(index);
            }
            if group.selectors.contains(selector) && !group.properties.is_disjoint(properties) {
                return None;
            }
        }
        None
    }
}

impl IntoIterator for Groups {
    type Item = (RuleKey, BTreeSet<String>);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .map(|group| (group.key, group.selectors))
            .collect::<Vec<_>>()
            .into_iter()
    }
}
