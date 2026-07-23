use crate::model::{Node, Specification};
use std::collections::BTreeMap;

pub const EFFECT: &str = r#"useEffect(()=>{if(!inferredCarousel)return;const previous=document.querySelector(inferredCarousel.previous);const next=document.querySelector(inferredCarousel.next);const target=document.querySelector(inferredCarousel.target);if(!previous||!next||!target)return;const update=advanced=>{previous.disabled=!advanced;next.disabled=advanced;animateScroll(target,advanced?inferredCarousel.extent:0,0)};const forward=()=>update(true);const reverse=()=>update(false);next.addEventListener('click',forward);previous.addEventListener('click',reverse);return()=>{next.removeEventListener('click',forward);previous.removeEventListener('click',reverse)}},[]);"#;

pub fn javascript(specification: &Specification, captured: bool) -> String {
    let value = (!captured)
        .then(|| specification.states.first().and_then(infer))
        .flatten()
        .map(|(previous, next, target, extent)| {
            serde_json::json!({
                "previous": previous,
                "next": next,
                "target": target,
                "extent": extent
            })
        })
        .unwrap_or(serde_json::Value::Null);
    format!("const inferredCarousel={value};")
}

fn infer(state: &crate::model::PageState) -> Option<(String, String, String, i64)> {
    let mut groups = BTreeMap::<&str, Vec<&Node>>::new();
    for node in &state.nodes {
        if matches!(node.tag.as_str(), "button" | "input")
            && let Some(parent) = node.parent.as_deref()
        {
            groups.entry(parent).or_default().push(node);
        }
    }
    groups
        .values()
        .filter_map(|siblings| {
            let previous = siblings.iter().find(|node| disabled(node))?;
            let next = siblings
                .iter()
                .find(|node| node.path != previous.path && !disabled(node))?;
            let parent = state
                .nodes
                .iter()
                .find(|node| previous.parent.as_deref() == Some(node.path.as_str()))?;
            let target = state
                .nodes
                .iter()
                .filter_map(|node| {
                    let dom = state.dom.get(&node.path)?;
                    let overflow = dom.scroll_width - dom.client_width;
                    let below = node.rect.y >= parent.rect.y + parent.rect.height - 1.0;
                    let aligned = node.rect.x <= parent.rect.x + parent.rect.width
                        && node.rect.x + node.rect.width >= parent.rect.x;
                    (overflow > 20.0 && below && aligned).then_some((
                        node,
                        overflow,
                        node.rect.y - parent.rect.y,
                    ))
                })
                .min_by(|left, right| left.2.total_cmp(&right.2))?;
            Some((
                previous.path.clone(),
                next.path.clone(),
                target.0.path.clone(),
                target.1.round() as i64,
            ))
        })
        .min_by_key(|(_, _, _, extent)| *extent)
}

fn disabled(node: &Node) -> bool {
    node.attributes.contains_key("disabled")
        || node
            .attributes
            .get("aria-disabled")
            .is_some_and(|value| value == "true")
}

#[cfg(test)]
mod tests {
    use crate::model::{DomNode, Node, PageState, Rect, Specification, Viewport};
    use std::collections::BTreeMap;

    #[test]
    fn infers_controls_and_nearest_horizontal_overflow() {
        let mut state = empty_state();
        state.nodes = vec![
            node("html>body>section", None, "section", 10.0, 200.0),
            node(
                "html>body>section>button:nth-of-type(1)",
                Some("html>body>section"),
                "button",
                10.0,
                32.0,
            ),
            node(
                "html>body>section>button:nth-of-type(2)",
                Some("html>body>section"),
                "button",
                10.0,
                32.0,
            ),
            node("html>body>div", Some("html>body"), "div", 60.0, 200.0),
        ];
        state.nodes[1]
            .attributes
            .insert("disabled".into(), String::new());
        state.dom.insert(
            "html>body>div".into(),
            DomNode {
                scroll_width: 420.0,
                client_width: 200.0,
                ..Default::default()
            },
        );
        let specification = Specification {
            schema_version: 1,
            requested_url: String::new(),
            captured_url: String::new(),
            states: vec![state],
            interactions: Vec::new(),
        };
        let output = super::javascript(&specification, false);
        assert!(output.contains("\"extent\":220"));
        assert!(output.contains("\"previous\":\"html>body>section>button:nth-of-type(1)\""));
        assert!(output.contains("\"target\":\"html>body>div\""));
        assert_eq!(
            super::javascript(&specification, true),
            "const inferredCarousel=null;"
        );
    }

    fn node(path: &str, parent: Option<&str>, tag: &str, y: f64, width: f64) -> Node {
        Node {
            path: path.into(),
            parent: parent.map(str::to_owned),
            tag: tag.into(),
            text: String::new(),
            attributes: BTreeMap::new(),
            rect: Rect {
                x: 0.0,
                y,
                width,
                height: 32.0,
            },
            style: BTreeMap::new(),
            before: None,
            after: None,
        }
    }

    fn empty_state() -> PageState {
        PageState {
            url: String::new(),
            title: String::new(),
            viewport: Viewport::default(),
            nodes: Vec::new(),
            dom: BTreeMap::new(),
            capture_blockers: Vec::new(),
            startup_nodes: Vec::new(),
            startup_delay_ms: 0,
            startup_duration_ms: 0,
            animations: Vec::new(),
            state_styles: Vec::new(),
            attribute_sequences: Vec::new(),
            css_rules: Vec::new(),
            asset_urls: Vec::new(),
            asset_data: BTreeMap::new(),
        }
    }
}
