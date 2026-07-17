use crate::model::{Node, PageState, Specification};

pub fn targets(specification: &Specification) -> String {
    let interactions = specification
        .interactions
        .iter()
        .map(|interaction| {
            let values = interaction
                .states
                .iter()
                .map(|state| {
                    specification
                        .states
                        .iter()
                        .find(|baseline| baseline.viewport.width == state.viewport.width)
                        .map(|baseline| scroll_snapshot(baseline, state))
                        .unwrap_or_else(|| "null".into())
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("[{values}]")
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[null,{interactions}]")
}

fn scroll_snapshot(baseline: &PageState, state: &PageState) -> String {
    let anchor = anchor(state);
    let Some(current) = anchor else {
        return "null".into();
    };
    let Some(baseline_node) = baseline.nodes.iter().find(|node| node.path == current.path) else {
        return "null".into();
    };
    let left = (baseline_node.rect.x - current.rect.x).max(0.0).round() as i64;
    let top = (baseline_node.rect.y - current.rect.y).max(0.0).round() as i64;
    let owner = scroll_owner(baseline, state, current);
    match owner {
        Some(path) => format!(
            "{{window:[0,0],elements:[[{}, {left},{top}]]}}",
            serde_json::to_string(path).unwrap()
        ),
        None => format!("{{window:[{left},{top}],elements:[]}}"),
    }
}

pub fn owner_path<'a>(baseline: &'a PageState, state: &PageState) -> Option<&'a str> {
    scroll_owner(baseline, state, anchor(state)?)
}

fn anchor(state: &PageState) -> Option<&Node> {
    state
        .nodes
        .iter()
        .filter(|node| eligible(node, state))
        .max_by(|left, right| {
            let left = left.rect.width * left.rect.height;
            let right = right.rect.width * right.rect.height;
            left.total_cmp(&right)
        })
}

fn eligible(node: &Node, state: &PageState) -> bool {
    node.tag != "#text"
        && node.rect.height > f64::from(state.viewport.height)
        && !node
            .style
            .get("position")
            .is_some_and(|value| matches!(value.as_str(), "fixed" | "sticky"))
}

fn scroll_owner<'a>(baseline: &'a PageState, state: &PageState, current: &Node) -> Option<&'a str> {
    let mut parent = current.parent.as_deref();
    while let Some(path) = parent {
        let node = baseline.nodes.iter().find(|node| node.path == path)?;
        if node.tag != "html"
            && node.tag != "body"
            && node.rect.height <= f64::from(state.viewport.height) * 1.2
            && node
                .style
                .get("overflow-y")
                .is_some_and(|value| matches!(value.as_str(), "auto" | "scroll"))
        {
            return Some(node.path.as_str());
        }
        parent = node.parent.as_deref();
    }
    None
}
