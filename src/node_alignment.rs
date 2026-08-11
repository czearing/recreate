use crate::model::{Node, PageState};
use crate::node_alignment_walk::align;
use std::collections::HashMap;

/// A state child with no counterpart in the baseline.
pub(crate) struct Insertion {
    /// Baseline path of the first surviving element after it, or `None` when it was
    /// appended last. `None` therefore means "genuinely at the end", never "unknown".
    pub before: Option<String>,
    /// Where that survivor lives in the state, which is what says a path is aliased
    /// rather than replaced.
    pub displaced: Option<String>,
}

/// A baseline child the interaction removed.
///
/// The alignment must be total on both sides. A result with one slot per state child can
/// only ever say "this child is that baseline child" or "this child is new", so a baseline
/// child that left has no representation at all and the changed-in-place fallback claims
/// its neighbour instead.
pub(crate) struct Removal {
    /// Baseline path of the element that left, which is the path it still answers to in
    /// the statically rendered recreation.
    pub path: String,
    /// State path of the parent it left, which is where the edit is scoped.
    pub parent: String,
    /// Carried so a consumer that can only address elements can drop text departures
    /// without re-parsing the path.
    pub tag: String,
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
pub(crate) struct Alignment<'s, 'b> {
    counterparts: HashMap<&'s str, &'b Node>,
    insertions: HashMap<&'s str, Insertion>,
    removals: Vec<Removal>,
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

    /// Every baseline element the interaction removed.
    pub fn removals(&self) -> &[Removal] {
        &self.removals
    }
}

/// Aligns the two captures, parent by parent, in a single pre-order pass.
pub(crate) fn of<'s, 'b>(state: &'s PageState, baseline: &'b PageState) -> Alignment<'s, 'b> {
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
    // children whose alignment depends on it. Every node is asked, not only those that
    // still have children: a parent the interaction emptied has no state children to name
    // it, and its departures would otherwise never be reached.
    let empty = Vec::new();
    let mut removals = Vec::new();
    for node in &state.nodes {
        let children = state_children
            .get(node.path.as_str())
            .cloned()
            .unwrap_or_default();
        let previous = counterparts
            .get(node.path.as_str())
            .and_then(|previous| baseline_children.get(previous.path.as_str()))
            .unwrap_or(&empty);
        if children.is_empty() && previous.is_empty() {
            continue;
        }
        let (pairs, departed) = align(&children, previous, state, baseline);
        removals.extend(departed.into_iter().map(|departed| Removal {
            path: departed.path.clone(),
            parent: node.path.clone(),
            tag: departed.tag.clone(),
        }));
        for (index, child) in children.iter().enumerate() {
            match pairs[index] {
                Some(previous) => {
                    counterparts.insert(child.path.as_str(), previous);
                }
                None => {
                    insertions.insert(child.path.as_str(), placement(&children, &pairs, index));
                }
            }
        }
    }
    Alignment {
        counterparts,
        insertions,
        removals,
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
