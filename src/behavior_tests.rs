use super::*;

fn candidate<'a>(path: &'a str, tag: &'a str, label: &'a str) -> TriggerCandidate<'a> {
    TriggerCandidate { path, tag, label }
}

#[test]
fn behavior_trigger_prefers_exact_identity() {
    let values = [
        candidate("search", "button", "Search"),
        candidate("avatar", "button", "Open account menu"),
    ];
    let key = TriggerKey {
        path: "avatar".into(),
        tag: "button".into(),
        label: "Open account menu".into(),
        occurrence: None,
    };
    assert_eq!(resolve_trigger(&key, &values), Some("avatar"));
}

#[test]
fn behavior_trigger_rejects_ambiguous_semantic_fallback() {
    let values = [
        candidate("card-a", "button", "More options"),
        candidate("card-b", "button", "More options"),
    ];
    let key = TriggerKey {
        path: "stale".into(),
        tag: "button".into(),
        label: "More options".into(),
        occurrence: None,
    };
    assert_eq!(resolve_trigger(&key, &values), None);
}

#[test]
fn behavior_trigger_resolves_repeated_occurrence() {
    let values = [
        candidate("card-a", "button", "More options"),
        candidate("card-b", "button", "More options"),
        candidate("card-c", "button", "More options"),
        candidate("card-d", "button", "More options"),
    ];
    for (index, expected) in ["card-a", "card-b", "card-c", "card-d"]
        .into_iter()
        .enumerate()
    {
        let key = TriggerKey {
            path: "stale".into(),
            tag: "button".into(),
            label: "More options".into(),
            occurrence: Some(index),
        };
        assert_eq!(resolve_trigger(&key, &values), Some(expected));
    }
}

#[test]
fn behavior_trigger_rejects_wrong_tag_and_missing_occurrence() {
    let values = [candidate("avatar", "button", "Open account menu")];
    let wrong_tag = TriggerKey {
        path: "avatar".into(),
        tag: "div".into(),
        label: "Open account menu".into(),
        occurrence: None,
    };
    let missing = TriggerKey {
        occurrence: Some(4),
        ..wrong_tag.clone()
    };
    assert_eq!(resolve_trigger(&wrong_tag, &values), None);
    assert_eq!(resolve_trigger(&missing, &values), None);
}
