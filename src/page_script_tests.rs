use super::source;

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

#[test]
fn keeps_only_active_media_children_as_direct_authored_rules() {
    let script = source();
    assert!(script.contains("const activeMedia = !media || matchMedia(media).matches"));
    assert!(script.contains("activeMedia || rule.type === CSSRule.MEDIA_RULE"));
}
