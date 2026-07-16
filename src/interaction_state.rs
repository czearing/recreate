use crate::model::PageState;
use std::collections::HashSet;

pub fn differs(left: &PageState, right: &PageState) -> bool {
    left.nodes != right.nodes
}

pub fn compact(state: &mut PageState, baseline: &PageState, settled: bool) {
    if settled {
        state.animations.clear();
    }
    let css: HashSet<_> = baseline.css_rules.iter().map(String::as_str).collect();
    state.css_rules.retain(|rule| !css.contains(rule.as_str()));
    let assets: HashSet<_> = baseline.asset_urls.iter().map(String::as_str).collect();
    state
        .asset_urls
        .retain(|url| !assets.contains(url.as_str()));
    state
        .asset_data
        .retain(|url, data| baseline.asset_data.get(url) != Some(data));
    state
        .state_styles
        .retain(|style| !baseline.state_styles.contains(style));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, Rect, StateStyle, Viewport};
    use std::collections::BTreeMap;

    fn state(nodes: usize) -> PageState {
        PageState {
            url: "https://example.test".into(),
            title: "Fixture".into(),
            viewport: Viewport::default(),
            nodes: (0..nodes)
                .map(|index| Node {
                    path: format!("html>body>div:nth-of-type({index})"),
                    parent: Some("html>body".into()),
                    tag: "div".into(),
                    text: index.to_string(),
                    attributes: BTreeMap::new(),
                    rect: Rect {
                        x: 0.0,
                        y: index as f64,
                        width: 100.0,
                        height: 20.0,
                    },
                    style: BTreeMap::new(),
                    before: None,
                    after: None,
                })
                .collect(),
            animations: Vec::new(),
            state_styles: vec![StateStyle {
                target: "html>body".into(),
                pseudo: Some(":focus".into()),
                media: None,
                declarations: "outline:1px solid".into(),
            }],
            css_rules: vec!["body{margin:0}".into()],
            asset_urls: vec!["https://example.test/logo.svg".into()],
            asset_data: BTreeMap::from([("blob:logo".into(), "data:image/png;base64,AA==".into())]),
        }
    }

    #[test]
    fn scales_to_large_states_without_serializing() {
        let baseline = state(10_000);
        let mut changed = baseline.clone();
        assert!(!differs(&baseline, &changed));
        changed.nodes[9_999]
            .attributes
            .insert("aria-expanded".into(), "true".into());
        assert!(differs(&baseline, &changed));
    }

    #[test]
    fn removes_only_metadata_already_in_baseline() {
        let baseline = state(1);
        let mut changed = baseline.clone();
        changed
            .css_rules
            .push("[role=dialog]{display:block}".into());
        changed
            .asset_urls
            .push("https://example.test/dialog.svg".into());
        compact(&mut changed, &baseline, true);
        assert_eq!(changed.css_rules, ["[role=dialog]{display:block}"]);
        assert_eq!(changed.asset_urls, ["https://example.test/dialog.svg"]);
        assert!(changed.asset_data.is_empty());
        assert!(changed.state_styles.is_empty());
    }

    #[test]
    fn compaction_reduces_repeated_output_size() {
        let mut baseline = state(20);
        baseline.asset_data.insert(
            "blob:large".into(),
            format!("data:image/png;base64,{}", "A".repeat(100_000)),
        );
        let mut changed = baseline.clone();
        let before = serde_json::to_vec(&changed).unwrap().len();
        compact(&mut changed, &baseline, true);
        let after = serde_json::to_vec(&changed).unwrap().len();
        assert!(after * 4 < before, "before={before} after={after}");
    }

    #[test]
    fn preserves_running_animation_metadata_at_safety_cap() {
        let baseline = state(1);
        let mut changed = baseline.clone();
        changed.animations.push(crate::model::Animation {
            target: "html>body>div:nth-of-type(0)".into(),
            keyframes: vec![
                serde_json::json!({"opacity":"0"}),
                serde_json::json!({"opacity":"1"}),
            ],
            timing: serde_json::json!({"duration":2000,"playState":"running"}),
        });
        compact(&mut changed, &baseline, false);
        assert_eq!(changed.animations.len(), 1);
        compact(&mut changed, &baseline, true);
        assert!(changed.animations.is_empty());
    }
}
