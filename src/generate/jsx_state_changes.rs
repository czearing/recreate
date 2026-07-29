pub(super) fn changed_existing_paths(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    replacements: &std::collections::HashSet<String>,
) -> std::collections::HashSet<String> {
    let baseline = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    state
        .nodes
        .iter()
        .filter(|node| {
            !replacements.iter().any(|root| {
                node.path == *root
                    || node
                        .path
                        .strip_prefix(root)
                        .is_some_and(|suffix| suffix.starts_with('>'))
            })
        })
        .filter_map(|node| {
            let previous = baseline.get(node.path.as_str())?;
            if node.tag == "#text" {
                (node.text != previous.text)
                    .then(|| node.parent.clone())
                    .flatten()
            } else {
                (node.style != previous.style
                    || node.attributes != previous.attributes
                    || node.text != previous.text
                    || node.before != previous.before
                    || node.after != previous.after)
                    .then(|| node.path.clone())
            }
        })
        .collect()
}
