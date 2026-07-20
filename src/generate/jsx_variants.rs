use super::{jsx, tree};
use crate::model::PageState;
use std::collections::BTreeMap;

pub fn page(
    state: &PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
    let body = state
        .nodes
        .iter()
        .find(|node| node.tag == "body")
        .map(|node| node.path.as_str())
        .unwrap_or("html");
    let root = state
        .nodes
        .iter()
        .find(|node| node.attributes.get("id").is_some_and(|id| id == "root"))
        .or_else(|| state.nodes.iter().find(|node| node.tag == "body"))
        .map(|node| node.path.as_str())
        .unwrap_or("html");
    let content = render_children(root, components, assets, handlers);
    let portals = components
        .children
        .get(body)
        .into_iter()
        .flatten()
        .filter(|path| path.as_str() != root)
        .map(|path| jsx::render(path, components, assets, 2, true, handlers))
        .collect::<String>();
    if portals.is_empty() {
        format!("<>{content}</>")
    } else {
        format!("<>{content}{{createPortal(<>{portals}</>,document.body)}}</>")
    }
}

pub fn selector() -> &'static str {
    "const selectViewport=(width,widths)=>{for(let index=0;index<widths.length-1;index++){if(width>widths[index+1])return index}return widths.length-1};"
}

pub fn fragment(
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    _delay_ms: u64,
    duration_ms: u64,
) -> String {
    let handlers = BTreeMap::new();
    let roots = components
        .nodes
        .values()
        .filter(|node| {
            node.parent
                .as_deref()
                .is_none_or(|parent| !components.nodes.contains_key(parent))
        })
        .map(|node| jsx::render(&node.path, components, assets, 2, true, &handlers))
        .collect::<String>();
    format!(
        "<div className=\"recreateStartupOverlay recreateStartupBlocking\" \
         data-recreate-startup=\"true\" \
         onAnimationEnd={{event=>{{if(event.target===event.currentTarget)onStartupDone()}}}} \
         style={{{{\
         \"--recreate-startup-delay\":\"0ms\",\
         \"--recreate-startup-duration\":\"{duration_ms}ms\"\
         }}}}>{roots}</div>"
    )
}

pub fn widths(states: &[PageState]) -> String {
    states
        .iter()
        .map(|state| state.viewport.width.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn render_children(
    root: &str,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
    jsx::render_children(root, components, assets, 2, handlers)
}
