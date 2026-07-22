use super::*;

fn candidate<'a>(path: &'a str, tag: &'a str, label: &'a str) -> TriggerCandidate<'a> {
    TriggerCandidate { path, tag, label }
}

#[test]
fn behavior_trigger_prefers_exact_identity() {
    let values = [
        candidate("search", "button", "Search"),
        candidate("avatar", "button", "Open profile"),
    ];
    let key = TriggerKey {
        path: "avatar".into(),
        tag: "button".into(),
        label: "Open profile".into(),
        occurrence: None,
    };
    assert_eq!(resolve_trigger(&key, &values), Some("avatar"));
}

#[test]
fn behavior_trigger_rejects_ambiguous_semantic_fallback() {
    let values = [
        candidate("item-a", "button", "Open actions"),
        candidate("item-b", "button", "Open actions"),
    ];
    let key = TriggerKey {
        path: "stale".into(),
        tag: "button".into(),
        label: "Open actions".into(),
        occurrence: None,
    };
    assert_eq!(resolve_trigger(&key, &values), None);
}

#[test]
fn behavior_trigger_resolves_repeated_occurrence() {
    let values = [
        candidate("item-a", "button", "Open actions"),
        candidate("item-b", "button", "Open actions"),
        candidate("item-c", "button", "Open actions"),
        candidate("item-d", "button", "Open actions"),
    ];
    for (index, expected) in ["item-a", "item-b", "item-c", "item-d"]
        .into_iter()
        .enumerate()
    {
        let key = TriggerKey {
            path: "stale".into(),
            tag: "button".into(),
            label: "Open actions".into(),
            occurrence: Some(index),
        };
        assert_eq!(resolve_trigger(&key, &values), Some(expected));
    }
}

#[test]
fn behavior_trigger_rejects_wrong_tag_and_missing_occurrence() {
    let values = [candidate("avatar", "button", "Open profile")];
    let wrong_tag = TriggerKey {
        path: "avatar".into(),
        tag: "div".into(),
        label: "Open profile".into(),
        occurrence: None,
    };
    let missing = TriggerKey {
        occurrence: Some(4),
        ..wrong_tag.clone()
    };
    assert_eq!(resolve_trigger(&wrong_tag, &values), None);
    assert_eq!(resolve_trigger(&missing, &values), None);
}
