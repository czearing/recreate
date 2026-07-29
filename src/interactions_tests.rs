use super::{
    interactions_discovery::candidate_relevant,
    interactions_evidence::{deduplicate, discovery_differs, responsive_baselines},
    interactions_runtime::restoration_requires_reload,
    interactions_scripts::{CANDIDATES, Candidate, PREFLIGHT},
};
use dedup::state_with_paths;

#[test]
fn repeated_controls_keep_independent_bindings() {
    assert!(!CANDIDATES.contains("values.findIndex"));
    assert!(CANDIDATES.contains("visible(element)"));
    assert!(CANDIDATES.contains("[tabindex]:not([tabindex=\"-1\"])"));
    assert!(CANDIDATES.contains("[data-tabster-dummy]"));
    assert!(CANDIDATES.contains("[role=\"none\"]"));
    assert!(CANDIDATES.contains("element.querySelector('button,a[href],[role=\"button\"]')"));
    assert!(!CANDIDATES.contains("closest('article"));
    assert!(!CANDIDATES.contains("priority"));
    assert!(!CANDIDATES.contains("}).filter(candidate =>"));
    assert!(!CANDIDATES.contains("pub(super)"));
    assert!(PREFLIGHT.contains("getBoundingClientRect"));
}

#[test]
fn embedded_interaction_scripts_parse_as_javascript() {
    for (name, source) in [("candidates", CANDIDATES), ("preflight", PREFLIGHT)] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("{name}.js"));
        std::fs::write(&path, source).unwrap();
        let status = std::process::Command::new("node")
            .args(["--check"])
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success(), "{name} script should parse");
    }
}

#[test]
fn state_controls_remain_relevant_in_successor_states() {
    let state = empty_state();
    let candidate = Candidate {
        path: "html>body>button".into(),
        tag: "button".into(),
        label: "List view".into(),
        occurrence: 0,
        disabled: false,
        navigates: false,
        state_control: true,
    };
    assert!(candidate_relevant(&candidate, &state, &state, None, false));
}

#[test]
fn popup_states_do_not_cross_product_unrelated_state_controls() {
    let mut baseline = empty_state();
    let mut trigger = empty_node("html>body>button:nth-of-type(1)");
    trigger.attributes.insert("role".into(), "tab".into());
    baseline.nodes.push(trigger.clone());
    let mut popup = empty_node("html>body>div:nth-of-type(2)");
    popup.attributes.insert("role".into(), "menu".into());
    let mut reached = baseline.clone();
    reached.nodes.push(popup);
    let candidate = Candidate {
        path: trigger.path.clone(),
        tag: "button".into(),
        label: "Grid".into(),
        occurrence: 0,
        disabled: false,
        navigates: false,
        state_control: true,
    };
    assert!(super::interactions_discovery::popup_state(
        &reached, &baseline
    ));
    assert!(!candidate_relevant(
        &candidate,
        &baseline,
        &reached,
        Some("html>body>button:nth-of-type(2)"),
        true
    ));
}

#[test]
fn repeated_collection_actions_share_a_semantic_family() {
    assert_eq!(
        super::interactions_evidence::action_family("Change icon for Project Alpha"),
        Some("Change icon")
    );
    assert_eq!(
        super::interactions_evidence::action_family("Create notebook"),
        None
    );
}

#[test]
fn plain_invoke_controls_force_clean_reload() {
    let mut baseline = empty_state();
    let voice = empty_node("html>body>button:nth-of-type(1)");
    baseline.nodes.push(voice.clone());
    let candidate = Candidate {
        path: voice.path,
        tag: voice.tag,
        label: "Use voice".into(),
        occurrence: 0,
        disabled: false,
        navigates: false,
        state_control: false,
    };
    assert!(super::interactions_capture::requires_clean_reload(
        &candidate, &baseline
    ));
}

#[test]
fn hover_candidates_deduplicate_identical_visual_evidence() {
    let mut baseline = empty_state();
    let first = empty_node("html>body>button:nth-of-type(1)");
    let second = empty_node("html>body>button:nth-of-type(2)");
    for node in [&first, &second] {
        baseline.state_styles.push(crate::model::StateStyle {
            target: node.path.clone(),
            scope: None,
            pseudo: Some(":hover".into()),
            target_pseudo: None,
            media: None,
            declarations: "color: red;".into(),
        });
    }
    let candidate = |node: &crate::model::Node| Candidate {
        path: node.path.clone(),
        tag: node.tag.clone(),
        label: node.path.clone(),
        occurrence: 0,
        disabled: false,
        navigates: false,
        state_control: false,
    };
    let representatives = super::interactions_hover::representative_candidates(
        vec![candidate(&first), candidate(&second)],
        &baseline,
    );
    assert_eq!(representatives.len(), 1);
}

#[test]
fn interactions_capture_every_recorded_viewport() {
    let mut baselines = Vec::new();
    for width in [1920, 1440, 768, 390, 320] {
        let mut state = crate::model::PageState {
            url: String::new(),
            title: String::new(),
            viewport: crate::model::Viewport::default(),
            dom: Default::default(),
            capture_blockers: Vec::new(),
            nodes: Vec::new(),
            startup_nodes: Vec::new(),
            startup_delay_ms: 0,
            startup_duration_ms: 0,
            animations: Vec::new(),
            state_styles: Vec::new(),
            attribute_sequences: Vec::new(),
            css_rules: Vec::new(),
            asset_urls: Vec::new(),
            asset_data: Default::default(),
        };
        state.viewport.width = width;
        baselines.push(state);
    }

    let baseline = baselines[0].clone();
    let state = baseline.clone();
    assert!(
        responsive_baselines(
            false, false, &state, &baseline, "button", "Action", &baselines
        )
        .is_empty()
    );
    assert_eq!(
        responsive_baselines(
            true, false, &state, &baseline, "textarea", "Prompt", &baselines
        )
        .len(),
        4
    );
    assert_eq!(
        responsive_baselines(
            false, true, &state, &baseline, "button", "Activity", &baselines
        )
        .len(),
        4
    );
}

fn empty_state() -> crate::model::PageState {
    crate::model::PageState {
        url: String::new(),
        title: String::new(),
        viewport: crate::model::Viewport::default(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes: Vec::new(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

fn empty_node(path: &str) -> crate::model::Node {
    crate::model::Node {
        path: path.into(),
        parent: path.rsplit_once('>').map(|(parent, _)| parent.into()),
        tag: path
            .rsplit_once('>')
            .map_or(path, |(_, node)| node)
            .split(':')
            .next()
            .unwrap_or("div")
            .into(),
        text: String::new(),
        attributes: Default::default(),
        rect: crate::model::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Default::default(),
        before: None,
        after: None,
    }
}

#[test]
fn discovers_text_entry_controls() {
    assert!(CANDIDATES.contains("input:not([type=\"hidden\"])"));
    assert!(CANDIDATES.contains("textarea,select"));
    assert!(CANDIDATES.contains("element.getAttribute('role') === 'tab'"));
    assert!(CANDIDATES.contains("element.hasAttribute('aria-pressed')"));
}

#[test]
fn incomplete_interaction_teardown_requires_reload() {
    let baseline = state_with_paths(&["html>body", "html>body>main"]);
    let restored = state_with_paths(&["html>body", "html>body>search"]);
    assert!(restoration_requires_reload(&restored, &baseline));
    let mut shifted = baseline.clone();
    shifted.nodes[1].tag = "button".into();
    shifted.nodes[1].rect.y = 48.0;
    assert!(restoration_requires_reload(&shifted, &baseline));
    assert!(!restoration_requires_reload(&baseline, &baseline));
}

#[test]
fn selective_baseline_does_not_force_a_reload() {
    let baseline = state_with_paths(&["html", "html>body", "html>body>main"]);
    let restored = state_with_paths(&["html", "html>body"]);
    assert!(!restoration_requires_reload(&restored, &baseline));
}

#[test]
fn unrelated_rotating_text_does_not_fake_a_surface() {
    let baseline = state_with_paths(&["html>body", "html>body>prompt"]);
    let mut changed = baseline.clone();
    changed.nodes[1].text = "A different rotating prompt".into();

    assert!(!discovery_differs(
        "Open actions",
        "html>body>trigger",
        &changed,
        &baseline,
    ));
}

#[path = "interactions_dedup_tests.rs"]
mod dedup;
