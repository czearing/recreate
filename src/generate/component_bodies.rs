//! Which candidate groups would emit the same component.
//!
//! Split from `tree.rs`, which owns finding the groups; this owns deciding that two of
//! them are one component. It depends on nothing ambient, so it reads as data in, data out.

use crate::model::Node;
use std::collections::{BTreeMap, HashMap};

/// Collapses candidate groups that would emit byte-identical components.
///
/// A generated component is a styled wrapper — its body is only the tag and the
/// class, because children are always rendered at the call site. So two groups
/// sharing a tag and a class are the same component by construction, however
/// their subtrees differ, and emitting both produces pure duplication.
pub(super) fn merge_identical_bodies(
    candidates: Vec<(Vec<String>, usize)>,
    nodes: &BTreeMap<String, &Node>,
    classes: &BTreeMap<String, String>,
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
        let body = format!(
            "{}|{}",
            node.tag,
            classes.get(root).map(String::as_str).unwrap_or_default()
        );
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
