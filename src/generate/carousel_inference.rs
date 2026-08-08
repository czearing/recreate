//! Handlers for a carousel the tool never watched being used.
//!
//! Reconstructing a widget from what it looks like is unsound in a way that reconstructing it
//! from what it declares is not. Two sibling controls with exactly one disabled, sitting above
//! something that overflows horizontally, describes a form dialog with a greyed `Save` beside
//! `Cancel` as exactly as it describes a carousel; no threshold separates them, because the
//! two classes coincide in every dimension that can be measured. An omitted carousel is a
//! static page — wrong, inert, and obvious to a reader. A fabricated one is a page where
//! `Cancel` scrolls a panel, which is indistinguishable from a feature the source never had.
//!
//! So the admission criterion is the author's own statement. `aria-roledescription` exists
//! only to rename a role for the user, which makes its value a declaration of intent rather
//! than an accident of layout, and nothing but a carousel says `carousel`. Geometry is kept,
//! but only to locate the parts of a widget already known to be one.

use crate::model::{Node, PageState, Specification};
use std::collections::BTreeMap;

/// The value that admits an inference, per the ARIA authoring practices for the carousel
/// pattern.
const DECLARATION: (&str, &str) = ("aria-roledescription", "carousel");

pub const EFFECT: &str = r#"useEffect(()=>{if(!inferredCarousel)return;const previous=document.querySelector(inferredCarousel.previous);const next=document.querySelector(inferredCarousel.next);const target=document.querySelector(inferredCarousel.target);if(!previous||!next||!target)return;const extent=inferredCarousel.extent;const update=advanced=>{previous.disabled=!advanced;next.disabled=advanced;animateScroll(target,advanced?extent:0,0);if(!advanced){target.scrollTo(0,0);for(const child of target.querySelectorAll('*')){const matrix=new DOMMatrixReadOnly(getComputedStyle(child).transform);if(Math.abs(matrix.m41)>100)child.style.transform='translateX(0px)'}}};const forward=()=>update(true);const reverse=()=>update(false);next.addEventListener('click',forward);previous.addEventListener('click',reverse);return()=>{next.removeEventListener('click',forward);previous.removeEventListener('click',reverse)}},[]);"#;

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

/// The nearest enclosing element that declares itself a carousel, `node` included.
fn declared<'a>(state: &'a PageState, node: &'a Node) -> Option<&'a Node> {
    let by_path = state
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut current = Some(node);
    while let Some(node) = current {
        if node
            .attributes
            .get(DECLARATION.0)
            .is_some_and(|value| value.trim().eq_ignore_ascii_case(DECLARATION.1))
        {
            return Some(node);
        }
        current = node
            .parent
            .as_deref()
            .and_then(|path| by_path.get(path))
            .copied();
    }
    None
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
            let container = declared(state, parent)?;
            let inside = format!("{}>", container.path);
            let target = state
                .nodes
                .iter()
                .filter_map(|node| {
                    let dom = state.dom.get(&node.path)?;
                    let overflow = dom.scroll_width - dom.client_width;
                    let below = node.rect.y >= parent.rect.y + parent.rect.height - 1.0;
                    let aligned = node.rect.x <= parent.rect.x + parent.rect.width
                        && node.rect.x + node.rect.width >= parent.rect.x;
                    (node.path.starts_with(&inside) && overflow > 20.0 && below && aligned)
                        .then_some((node, overflow, node.rect.y - parent.rect.y))
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
