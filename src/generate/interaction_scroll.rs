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
    let vertical = anchor(state).and_then(|current| {
        let baseline_node = baseline
            .nodes
            .iter()
            .find(|node| node.path == current.path)?;
        let left = (baseline_node.rect.x - current.rect.x).max(0.0).round() as i64;
        let top = (baseline_node.rect.y - current.rect.y).max(0.0).round() as i64;
        Some((current, left, top))
    });
    let horizontal = horizontal_scroll(baseline, state);
    if vertical.is_none() && horizontal.is_none() {
        return "null".into();
    }
    let mut window = (0, 0);
    let mut elements: Vec<(String, i64, i64)> = Vec::new();
    if let Some((current, left, top)) = vertical {
        if let Some(path) = scroll_owner(baseline, state, current) {
            elements.push((path.into(), 0, top));
        } else {
            window = (left, top);
        }
    }
    if let Some((path, left)) = horizontal {
        if let Some(element) = elements.iter_mut().find(|element| element.0 == path) {
            element.1 = left;
        } else {
            elements.push((path.into(), left, 0));
        }
    }
    let elements = elements
        .into_iter()
        .map(|(path, left, top)| {
            format!("[{}, {left},{top}]", serde_json::to_string(&path).unwrap())
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{window:[{},{}],elements:[{elements}]}}",
        window.0, window.1
    )
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

fn horizontal_scroll<'a>(baseline: &'a PageState, state: &PageState) -> Option<(&'a str, i64)> {
    let current = state
        .nodes
        .iter()
        .filter_map(|node| {
            let baseline_node = baseline.nodes.iter().find(|item| item.path == node.path)?;
            let shift = baseline_node.rect.x - node.rect.x;
            (node.tag != "#text" && shift > 1.0).then_some((node, shift))
        })
        .max_by(|(left, left_shift), (right, right_shift)| {
            left_shift.total_cmp(right_shift).then_with(|| {
                (left.rect.width * left.rect.height)
                    .total_cmp(&(right.rect.width * right.rect.height))
            })
        })?;
    let content_shift = current.1;
    let mut parent = current.0.parent.as_deref();
    while let Some(path) = parent {
        let baseline_node = baseline.nodes.iter().find(|node| node.path == path)?;
        let state_node = state.nodes.iter().find(|node| node.path == path)?;
        let owner_shift = baseline_node.rect.x - state_node.rect.x;
        if baseline_node
            .style
            .get("overflow-x")
            .is_some_and(|value| matches!(value.as_str(), "auto" | "scroll" | "hidden"))
            && content_shift - owner_shift > 1.0
        {
            return Some((
                baseline_node.path.as_str(),
                (content_shift - owner_shift).round() as i64,
            ));
        }
        parent = baseline_node.parent.as_deref();
    }
    None
}
