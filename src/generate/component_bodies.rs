//! Which candidate groups would emit the same component.
//!
//! Split from `tree.rs`, which owns finding the groups; this owns deciding that two of
//! them are one component. It depends on nothing ambient, so it reads as data in, data out.

use crate::model::Node;
use std::collections::{BTreeMap, HashMap};

/// Collapses candidate groups that would emit byte-identical components.
///
/// A generated component is a styled wrapper — its body is only the tag, because
/// both the children and the class are supplied at the call site. So two groups
/// sharing a tag are the same component by construction, however their subtrees
/// or their styling differ, and emitting both produces pure duplication.
pub(super) fn merge_identical_bodies(
    candidates: Vec<(Vec<String>, usize)>,
    nodes: &BTreeMap<String, &Node>,
) -> Vec<(Vec<String>, usize)> {
    let mut order: Vec<String> = Vec::new();
    let mut merged: HashMap<String, (Vec<String>, usize)> = HashMap::new();
    for (roots, size) in candidates {
        let Some(root) = roots.first() else {
            continue;
        };
        let Some(node) = nodes.get(root) else {
            continue;
        };
        // The emitted body is `<tag className={className}>{children}</tag>`: children arrive as a
        // prop and the class arrives as a prop, so the only thing left that can make two bodies
        // differ is the tag. Keying on the class as well produced one component per distinct style
        // bucket — 21 `Radialgradient` components on a real capture whose bodies were identical
        // apart from a hashed class name, which is a rename rather than a component.
        let body = node.tag.clone();
        match merged.get_mut(&body) {
            Some((existing, _)) => existing.extend(roots),
            None => {
                order.push(body.clone());
                merged.insert(body, (roots, size));
            }
        }
    }
    order
        .into_iter()
        .filter_map(|body| merged.remove(&body))
        .collect()
}
