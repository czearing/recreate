use super::css::ScopeCache;
use crate::model::{PageState, Specification};
use std::collections::BTreeMap;

pub fn append(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
    classes: &BTreeMap<String, String>,
    css: &mut String,
    timing: &impl Fn(&str),
) -> Vec<BTreeMap<String, String>> {
    let mut outputs = Vec::new();
    let mut cache = ScopeCache::default();
    for (index, interaction) in specification.interactions.iter().enumerate() {
        if !super::interactions::rendered(interaction, &specification.states) {
            outputs.push(BTreeMap::new());
            timing(&format!("interaction_{}", index + 1));
            continue;
        }
        let shared = super::interactions::shared_trigger(interaction, &specification.states);
        let states = states(interaction, specification, shared);
        let interaction_spec = Specification {
            schema_version: specification.schema_version,
            requested_url: specification.requested_url.clone(),
            captured_url: specification.captured_url.clone(),
            states,
            interactions: Vec::new(),
            transitions: Vec::new(),
        };
        let surface_paths = shared.then(|| {
            crate::interaction_surface::paths(&interaction_spec.states, &specification.states)
        });
        let output = super::css::build_scoped(
            &interaction_spec,
            assets,
            "s",
            false,
            Some((&specification.states, classes)),
            Some(&mut cache),
            surface_paths.as_ref(),
        );
        css.push_str(&output.css);
        outputs.push(output.classes);
        timing(&format!("interaction_{}", index + 1));
    }
    outputs
}

fn states(
    interaction: &crate::model::Interaction,
    specification: &Specification,
    shared: bool,
) -> Vec<PageState> {
    interaction
        .states
        .iter()
        .map(|state| {
            let baseline = specification
                .states
                .iter()
                .find(|baseline| baseline.viewport.width == state.viewport.width)
                .unwrap_or(&specification.states[0]);
            let state = if shared {
                shared_state(state, baseline)
            } else {
                state.clone()
            };
            super::css_state_helpers::with_baseline_css(state, baseline)
        })
        .collect()
}

fn shared_state(state: &PageState, baseline: &PageState) -> PageState {
    let roots = crate::interaction_surface::roots(state, baseline);
    PageState {
        url: state.url.clone(),
        title: state.title.clone(),
        viewport: state.viewport.clone(),
        nodes: state
            .nodes
            .iter()
            .filter(|node| roots.iter().any(|root| within(&node.path, root)))
            .cloned()
            .collect(),
        dom: state
            .dom
            .iter()
            .filter(|(path, _)| roots.iter().any(|root| within(path, root)))
            .map(|(path, node)| (path.clone(), node.clone()))
            .collect(),
        capture_blockers: state.capture_blockers.clone(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: state.state_styles.clone(),
        attribute_sequences: Vec::new(),
        css_rules: state.css_rules.clone(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

fn within(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('>'))
}
