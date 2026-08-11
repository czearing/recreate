//! The walk that decides which baseline child each state child is, and which baseline
//! children have no state child at all.
use crate::model::{Node, PageState};

/// Pairs each state child with the baseline child it is, and names the baseline children
/// that left.
///
/// One left-to-right walk with lookahead in both directions, the alignment underneath every
/// virtual-DOM diff: pair identical children; otherwise look ahead in the state for the
/// baseline child, and everything skipped over is new; otherwise look ahead in the baseline
/// for the state child, and everything skipped over was removed; otherwise, if the tags
/// agree, pair them anyway, because a child that appears in neither direction is the same
/// element with different attributes.
///
/// The walk must be total on both sides. With only the state-side lookahead, a departure is
/// not merely missed, it is unsayable, and the changed-in-place fallback claims its
/// neighbour: removing the middle of three like siblings paired the survivor to the element
/// that left, rewriting that element's content and leaving the survivor's own copy in place.
/// The state-side lookahead is tried first, so an insertion still reads as an insertion and
/// the new branch fires only where the old one had already run out of answers.
pub(super) fn align<'b>(
    children: &[&Node],
    previous: &[&'b Node],
    state: &PageState,
    baseline: &PageState,
) -> (Vec<Option<&'b Node>>, Vec<&'b Node>) {
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
        } else if let Some(later) = (candidate_index + 1..previous.len())
            .find(|later| same(previous[*later], children[index], baseline, state))
        {
            pairs[index] = Some(previous[later]);
            candidate_index = later + 1;
            index += 1;
        } else if candidate.tag == children[index].tag {
            pairs[index] = Some(*candidate);
            candidate_index += 1;
            index += 1;
        } else {
            index += 1;
        }
    }
    let departed = previous
        .iter()
        .filter(|candidate| {
            !pairs
                .iter()
                .flatten()
                .any(|paired| std::ptr::eq(*paired, **candidate))
        })
        .copied()
        .collect();
    (pairs, departed)
}

/// Whether two captured nodes are the same element seen twice.
///
/// The child signature carries each child's text, not only its tag and role. Without it,
/// sibling elements whose only difference is their content are indistinguishable — an
/// element's own `text` is empty, because the capture records content in child text nodes —
/// so the walk can only pair them positionally, and a departure is then attributed to
/// whichever path fell off the end rather than to the element that left.
fn same(baseline_node: &Node, state_node: &Node, baseline: &PageState, state: &PageState) -> bool {
    baseline_node.tag == state_node.tag
        && baseline_node.text == state_node.text
        && baseline_node.attributes == state_node.attributes
        && child_signature(baseline, &baseline_node.path)
            == child_signature(state, &state_node.path)
}

fn child_signature<'a>(
    state: &'a PageState,
    path: &str,
) -> Vec<(&'a str, Option<&'a String>, &'a str)> {
    state
        .nodes
        .iter()
        .filter(|node| node.parent.as_deref() == Some(path))
        .map(|node| {
            (
                node.tag.as_str(),
                node.attributes.get("role"),
                node.text.as_str(),
            )
        })
        .collect()
}
