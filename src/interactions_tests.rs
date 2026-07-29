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
    assert!(candidate_relevant(&candidate, &state, &state, None));
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
