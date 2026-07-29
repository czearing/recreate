use super::project_test_support::{node, state};
use crate::model::{Interaction, Specification};

pub fn text_entry_specification() -> Specification {
    let mut baseline = state(1200);
    baseline.nodes.truncate(3);
    let container_path = "html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type(1)";
    let textarea_path = format!("{container_path}>textarea:nth-of-type(1)");
    let mut container = node(
        container_path,
        Some("html>body:nth-of-type(1)>div:nth-of-type(1)"),
        "",
        None,
    );
    container.rect.width = 866.0;
    let mut textarea = node(&textarea_path, Some(container_path), "", None);
    textarea.tag = "textarea".into();
    textarea
        .attributes
        .insert("placeholder".into(), "Draft a lunch plan".into());
    let mut hidden_action = node(
        &format!("{container_path}>div:nth-of-type(1)"),
        Some(container_path),
        "",
        None,
    );
    hidden_action.rect.width = 0.0;
    hidden_action.rect.height = 0.0;
    hidden_action
        .style
        .insert("position".into(), "absolute".into());
    let placeholder = node(
        &format!("{container_path}>div:nth-of-type(2)"),
        Some(container_path),
        "Draft a lunch plan",
        None,
    );
    baseline
        .nodes
        .extend([container, textarea, hidden_action, placeholder]);

    let mut active = baseline.clone();
    active.nodes[3].rect.width = 946.0;
    active.nodes[4].rect.height = 40.0;
    active.nodes[5].rect.width = 20.0;
    active.nodes[5].rect.height = 20.0;
    active.nodes.pop();
    let action_path = format!("{container_path}>button:nth-of-type(1)");
    let mut action = node(&action_path, Some(container_path), "Create item", None);
    action.tag = "button".into();
    active.nodes.push(action);
    Specification {
        schema_version: 1,
        requested_url: baseline.url.clone(),
        captured_url: baseline.url.clone(),
        states: vec![baseline],
        interactions: vec![Interaction {
            trigger_path: textarea_path,
            trigger_tag: "textarea".into(),
            trigger_label: "Draft a lunch plan".into(),
            trigger_occurrence: None,
            focused_path: None,
            states: vec![active],
        }],
        transitions: Vec::new(),
    }
}
