use super::source_with_sheets;

/// The capture script as the page receives it when it can read its own stylesheets.
fn source() -> String {
    source_with_sheets(&[])
}

#[test]
fn preserves_media_scope_for_dynamic_state_rules() {
    let script = source();
    assert!(script.contains("media,"));
    assert!(script.contains("rule.type === CSSRule.MEDIA_RULE"));
    assert!(script.contains("`(${media}) and (${rule.conditionText})`"));
    assert!(!script.contains("media: reduced ? media : null"));
}

#[test]
fn preserves_dynamic_pseudo_element_selectors() {
    let script = source();
    assert!(script.contains("const pseudoElement = base.match(/::[\\w-]+$/)?.[0] || ''"));
    assert!(script.contains("document.querySelectorAll(query)"));
    assert!(script.contains("target_pseudo: scoped"));
    assert!(script.contains("`${tailStates.join('')}${pseudoElement}`"));
}

#[test]
fn captures_attribute_sequences() {
    let script = source();
    assert!(script.contains("window.__recreateAttributeMutations"));
    assert!(script.contains("attribute_sequences: attributeSequences"));
    assert!(script.contains("interval_ms"));
}

/// The activation primitive lives in its own module, so this asserts the composed script
/// actually carries it and that emission consults it. The behaviour itself is proven by
/// `rule_activation_tests`, which executes this logic against a scripted CSSOM.
#[test]
fn keeps_only_active_conditional_children_as_direct_authored_rules() {
    let script = source();
    assert!(!script.contains("__RULE_ACTIVATION__"));
    assert!(script.contains("for (const entry of probes) entry.active = probeMatches(entry)"));
    assert!(script.contains("active || rule.type === CSSRule.MEDIA_RULE"));
    assert!(!script.contains("matchMedia(media).matches"));
}

#[test]
fn captures_physical_dom_and_complete_computed_styles() {
    let script = source();
    assert!(script.contains("scan(document.documentElement)"));
    assert!(script.contains("element.shadowRoot"));
    assert!(script.contains("physical_parent"));
    assert!(script.contains("assigned_slot"));
    assert!(script.contains("client_rects"));
    assert!(script.contains("computed_style_dictionary"));
    assert!(!script.contains("'HEAD'"));
}

#[test]
fn caches_dom_paths_and_sibling_indexes() {
    let script = source();
    assert!(script.contains("const pathCache = new WeakMap"));
    assert!(script.contains("const siblingIndexes = new WeakMap"));
    assert!(!script.contains("peers.indexOf"));
}

/// Both node-record producers must ask the engine for the disabled state rather than
/// re-derive it, and both must be kept in step: a control disabled by an ancestor
/// `<fieldset>` carries no attribute of its own, and the `disabled` DOM property only
/// reflects that absent attribute, so either substitute answers `false` for it.
#[test]
fn records_the_engine_answered_disabled_state_in_every_node_record() {
    for script in [source(), crate::interaction_script::source()] {
        assert!(script.contains("disabled: element.matches(':disabled')"));
        assert!(!script.contains("disabled: element.disabled"));
    }
}

#[test]
fn generated_capture_script_parses() {
    let script = super::source_without_assets();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("capture.js");
    std::fs::write(&path, &script).unwrap();
    let status = std::process::Command::new("node")
        .args(["--check"])
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}
