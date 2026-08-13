use crate::node_alignment::Alignment;

/// Which already-rendered paths this state edits.
///
/// The baseline lookup goes through the alignment, never through the state path: a path
/// present in both captures can denote two different elements, because removing or
/// inserting an element renames every following same-tag sibling. Keying on the path
/// diffs an element against whichever one used to answer to its name.
///
/// A departure is its own reason to edit. The parent that lost a child is reported as
/// changed even when nothing about the surviving children differs, because otherwise the
/// removal reaches no stage that could express it.
pub(super) fn changed_existing_paths(
    state: &crate::model::PageState,
    alignment: &Alignment,
    replacements: &std::collections::HashSet<String>,
) -> std::collections::HashSet<String> {
    let outside = |path: &str| {
        !replacements.iter().any(|root| {
            path == root
                || path
                    .strip_prefix(root)
                    .is_some_and(|suffix| suffix.starts_with('>'))
        })
    };
    state
        .nodes
        .iter()
        .filter(|node| outside(&node.path))
        .filter_map(|node| {
            let previous = alignment.counterpart(&node.path)?;
            if node.tag == "#text" {
                (node.text != previous.text)
                    .then(|| node.parent.clone())
                    .flatten()
            } else {
                (node.style != previous.style
                    || node.attributes != previous.attributes
                    || node.text != previous.text
                    || node.pseudos != previous.pseudos)
                    .then(|| node.path.clone())
            }
        })
        .chain(
            alignment
                .removals()
                .iter()
                .map(|removal| removal.parent.clone())
                .filter(|parent| outside(parent)),
        )
        .collect()
}
