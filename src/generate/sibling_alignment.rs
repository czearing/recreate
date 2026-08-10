use crate::model::{Node, PageState};
use std::collections::HashMap;

/// A state child with no counterpart in the baseline.
pub(super) struct Insertion {
    /// Baseline path of the first surviving element after it, or `None` when it was
    /// appended last. `None` therefore means "genuinely at the end", never "unknown".
    pub before: Option<String>,
    /// Where that survivor lives in the state, which is what says a path is aliased
    /// rather than replaced.
    pub displaced: Option<String>,
}

/// Which element of the baseline each element of the state is, and which are new.
///
/// Node identity in a capture is positional — a path is a chain of `:nth-of-type()`
/// ordinals — so inserting an element renames every following same-tag sibling and a path
/// present in both captures may denote two different elements. Every stage that asks "what
/// did this element look like before?" must ask it here; keying a baseline lookup on the
/// state path diffs an inserted element against the one it displaced.
///
/// Position is not missing from the evidence: the capture walks `childNodes` in document
/// order, so a parent's children are recorded in the order the page laid them out. What
/// must be recovered is the correspondence between the two child lists, and the position
/// follows from it.
pub(super) struct Alignment<'s, 'b> {
    counterparts: HashMap<&'s str, &'b Node>,
    insertions: HashMap<&'s str, Insertion>,
}

impl<'b> Alignment<'_, 'b> {
    /// The baseline node `path` names in the state, or `None` when the element is new.
    pub fn counterpart(&self, path: &str) -> Option<&'b Node> {
        self.counterparts.get(path).copied()
    }

    /// Where an inserted element belongs, or `None` when `path` names a survivor.
    pub fn insertion(&self, path: &str) -> Option<&Insertion> {
        self.insertions.get(path)
    }
}

/// Aligns the two captures, parent by parent, in a single pre-order pass.
pub(super) fn of<'s, 'b>(state: &'s PageState, baseline: &'b PageState) -> Alignment<'s, 'b> {
    let state_children = index(state);
    let baseline_children = index(baseline);
    let mut counterparts = HashMap::new();
    let mut insertions = HashMap::new();
    for node in state.nodes.iter().filter(|node| node.parent.is_none()) {
        if let Some(root) = baseline.nodes.iter().find(|root| root.path == node.path) {
            counterparts.insert(node.path.as_str(), root);
        }
    }
    // The capture walks the tree depth-first, so a parent is always resolved before the
    // children whose alignment depends on it.
    let mut aligned = std::collections::HashSet::new();
    let empty = Vec::new();
    for parent in state.nodes.iter().filter_map(|node| node.parent.as_deref()) {
        if !aligned.insert(parent) {
            continue;
        }
        let Some(children) = state_children.get(parent) else {
            continue;
        };
        let previous = counterparts
            .get(parent)
            .and_then(|parent| baseline_children.get(parent.path.as_str()))
            .unwrap_or(&empty);
        let pairs = align(children, previous, state, baseline);
        for (index, child) in children.iter().enumerate() {
            match pairs[index] {
                Some(previous) => {
                    counterparts.insert(child.path.as_str(), previous);
                }
                None => {
                    insertions.insert(child.path.as_str(), placement(children, &pairs, index));
                }
            }
        }
    }
    Alignment {
        counterparts,
        insertions,
    }
}

/// Where an inserted child belongs: before the next surviving element, in the baseline
/// path that element still answers to in the statically rendered recreation.
///
/// The anchor must be a survivor, not the immediate next sibling: an anchor that is itself
/// an insertion is not in the document when the portal attaches, so the lookup fails and
/// the element silently appends. Successive insertions against one survivor stack in call
/// order, which is document order, so their relative order needs nothing further.
fn placement(children: &[&Node], pairs: &[Option<&Node>], index: usize) -> Insertion {
    let survivor = children
        .iter()
        .zip(pairs)
        .skip(index + 1)
        .filter(|(node, _)| !node.tag.starts_with('#'))
        .find_map(|(node, previous)| previous.map(|previous| (node, previous)));
    Insertion {
        before: survivor.map(|(_, previous)| previous.path.clone()),
        displaced: survivor.map(|(node, _)| node.path.clone()),
    }
}

/// Every node's children, in the document order the capture recorded them in.
fn index(state: &PageState) -> HashMap<&str, Vec<&Node>> {
    let mut children: HashMap<&str, Vec<&Node>> = HashMap::new();
    for node in &state.nodes {
        if let Some(parent) = node.parent.as_deref() {
            children.entry(parent).or_default().push(node);
        }
    }
    children
}

/// Pairs each state child with the baseline child it is, or `None` when it is new.
///
/// One left-to-right walk with lookahead, the alignment underneath every virtual-DOM diff:
/// pair identical children; otherwise look ahead for the baseline child, and everything
/// skipped over is new; otherwise, if the tags agree, pair them anyway, because a child
/// that appears nowhere ahead is the same element with different attributes. That last
/// step is what keeps a sibling the interaction also annotated from reading as a second
/// insertion, and requiring the tag to agree is what stops it swallowing a genuine
/// insertion of a different element type.
fn align<'b>(
    children: &[&Node],
    previous: &[&'b Node],
    state: &PageState,
    baseline: &PageState,
) -> Vec<Option<&'b Node>> {
    let mut pairs = vec![None; children.len()];
    let mut candidate_index = 0;
    let mut index = 0;
    while index < children.len() {
        let Some(candidate) = previous.get(candidate_index) else {
            break;
        };
        if same(candidate, children[index], baseline, state) {
            pairs[index] = Some(*candidate);
            candidate_index += 1;
            index += 1;
        } else if let Some(later) = (index + 1..children.len())
            .find(|later| same(candidate, children[*later], baseline, state))
        {
            pairs[later] = Some(*candidate);
            candidate_index += 1;
            index = later + 1;
        } else if candidate.tag == children[index].tag {
            pairs[index] = Some(*candidate);
            candidate_index += 1;
            index += 1;
        } else {
            index += 1;
        }
    }
    pairs
}

/// Whether two captured nodes are the same element seen twice.
fn same(baseline_node: &Node, state_node: &Node, baseline: &PageState, state: &PageState) -> bool {
    baseline_node.tag == state_node.tag
        && baseline_node.text == state_node.text
        && baseline_node.attributes == state_node.attributes
        && child_signature(baseline, &baseline_node.path) == child_signature(state, &state_node.path)
}

fn child_signature(state: &PageState, path: &str) -> Vec<(String, Option<String>)> {
    state
        .nodes
        .iter()
        .filter(|node| node.parent.as_deref() == Some(path))
        .map(|node| (node.tag.clone(), node.attributes.get("role").cloned()))
        .collect()
}
