use crate::model::PageState;
use std::collections::BTreeMap;

pub fn append(states: &[PageState], classes: &mut BTreeMap<String, String>, css: &mut String) {
    let overlays: Vec<_> = states
        .iter()
        .flat_map(|state| {
            state
                .startup_nodes
                .iter()
                .filter(|node| node.parent.is_none())
                .map(move |node| (state, node))
        })
        .collect();
    if overlays.is_empty() {
        return;
    }
    css.push_str(
        "@keyframes recreateStartupOverlay{0%,94%{opacity:1;visibility:visible;\
         pointer-events:auto}100%{opacity:0;visibility:hidden;pointer-events:none}}\
         .recreateStartupOverlay{opacity:0;visibility:hidden;pointer-events:none;\
         animation-name:recreateStartupOverlay!important;animation-timing-function:linear!important;\
         animation-duration:var(--recreate-startup-duration,1ms)!important;\
         animation-delay:var(--recreate-startup-delay,0ms)!important;\
         animation-fill-mode:forwards!important}\
         @media(prefers-reduced-motion:reduce){.recreateStartupOverlay{animation:none!important;\
         display:none!important;opacity:0!important;visibility:hidden!important;pointer-events:none!important}}\
         .recreateStartupBlocking{position:fixed!important;inset:0!important;width:100vw!important;\
         height:100vh!important;max-width:100vw!important;max-height:100vh!important;\
         overflow:hidden!important}\n",
    );
    for (state, node) in overlays {
        let class = classes.entry(node.path.clone()).or_default();
        if !class.contains("recreateStartupOverlay") {
            class.push_str(" recreateStartupOverlay");
        }
        if node.rect.width * node.rect.height
            >= f64::from(state.viewport.width * state.viewport.height) * 0.5
            && !class.contains("recreateStartupBlocking")
        {
            class.push_str(" recreateStartupBlocking");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, PageState, Rect, Viewport};

    #[test]
    fn marks_captured_startup_root() {
        let root = Node {
            path: "startup".into(),
            parent: None,
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        };
        let state = PageState {
            url: String::new(),
            title: String::new(),
            viewport: Viewport::default(),
            nodes: Vec::new(),
            startup_nodes: vec![root],
            startup_delay_ms: 100,
            startup_duration_ms: 500,
            animations: Vec::new(),
            state_styles: Vec::new(),
            css_rules: Vec::new(),
            asset_urls: Vec::new(),
            asset_data: Default::default(),
        };
        let mut classes = BTreeMap::from([("startup".into(), "base".into())]);
        let mut css = String::new();
        append(&[state], &mut classes, &mut css);
        assert!(classes["startup"].contains("recreateStartupOverlay"));
        assert!(css.contains("--recreate-startup-duration"));
    }
}
