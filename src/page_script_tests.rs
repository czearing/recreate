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
    assert!(script.contains("const box = generatedBoxOf(member)"));
    assert!(script.contains("document.querySelectorAll(relation.query)"));
    assert!(script.contains("target_pseudo: scoped"));
    assert!(script.contains("`${tailStates.join('')}${pseudoElement}`"));
}

/// Every stage that reads a selector reads it through the one reader.
///
/// A selector list is separated by top-level commas only, so `split(',')` cuts through
/// `:is(.a, .b)`, `:not([a],[b])` and `[title="a,b"]` alike. What comes out is usually still
/// a selector, so the stage matches a population the author never named or throws where the
/// throw is caught — either way the rule leaves no record and nothing reports it. The rule
/// is stated over the whole script rather than over any one stage, because the defect was a
/// second copy of the split surviving the repair of the first.
#[test]
fn no_stage_divides_a_selector_by_a_bare_comma() {
    let script = source();
    assert!(script.contains("const selectorMembers = text =>"));
    assert!(
        !script.contains("selectorText.split(','"),
        "a selector list is still being divided by a bare comma"
    );
    assert!(
        !script.contains("selector.split(','"),
        "a selector is still being divided by a bare comma"
    );
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
    assert!(script.contains("const carrier = prelude =>"));
    assert!(script.contains("for (const { rule, media, active, carriers, base } of entries)"));
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

/// The generated boxes a capture records are one list, read by both producers, by the probe
/// that measures their baselines and by every consumer of the record. Spelling the list at
/// each site is what let `::backdrop` be absent from one while the others were ready for it.
/// The conditions themselves belong to `generated_boxes`; what is asserted here is that the
/// composed capture script carries that one table and derives its revert sheet from it
/// rather than from a second literal list beside it.
///
/// The revert sheet must also emit one rule per name. A selector list is invalid as a whole
/// if any part of it is, and the table now holds names discovered from the page rather than
/// only names written here, so a single list would revert nothing on a page naming a
/// pseudo-element this engine does not implement — and every baseline would come back live.
#[test]
fn derives_every_generated_box_it_reverts_from_the_one_recorded_list() {
    let script = source();
    assert!(script.contains(crate::generated_boxes::SOURCE.trim()));
    assert!(script.contains("generatedBoxTests()"));
    assert!(script.contains(".map(([name]) => `*${name}{${REVERT_TO_USER_AGENT}}`)"));
    assert!(!script.contains("'*::before,*::after"));
    assert!(!script.contains("['::before', '::after']"));
    assert!(!script.contains(".join(',')}{${REVERT_TO_USER_AGENT}"));
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

/// The occlusion verdict must be reached while the page is open, because the record the
/// script writes is an authored diff and cannot answer it afterwards. This asserts the rule
/// reached the composed script and is asked per node; the verdicts themselves are proven by
/// locking_overlay_tests.
#[test]
fn records_the_occlusion_verdict_the_authored_diff_cannot_hold() {
    let script = source();
    assert!(!script.contains("__BLOCKING_OVERLAY__"));
    assert!(script.contains("blocking_overlay: isBlockingOverlay(element)"));
    assert!(script.contains("element.checkVisibility({"));
}

/// A page that never reports itself settled is read anyway, so the doubt has to reach the
/// artifact. The capture script is the single owner of capture_blockers, and the settle
/// script is the only thing that knows the ceiling released it, so the fact is handed over
/// rather than re-derived on the Rust side where it is no longer observable.
#[test]
fn an_unsettled_page_records_the_doubt_on_the_artifact() {
    let script = source();
    assert!(script.contains("window.__recreateUnsettled"));
    assert!(script.contains("it was read at the stability ceiling"));
    assert!(
        crate::capture_settle::source(true, true).contains("window.__recreateUnsettled = true")
    );
}
