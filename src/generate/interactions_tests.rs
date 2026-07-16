use super::*;
use crate::model::{Attributes, Rect, Styles};

fn node(tag: &str) -> Node {
    Node {
        path: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
        parent: Some("html>body:nth-of-type(1)".into()),
        tag: tag.into(),
        text: String::new(),
        attributes: Attributes::new(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    }
}

#[test]
fn custom_controls_get_keyboard_semantics() {
    let binding = trigger_binding(Some(&node("div")), "event=>onReset(event)", None);
    assert!(binding.contains("role=\"button\""));
    assert!(binding.contains("tabIndex={0}"));
    assert!(binding.contains("onKeyDown"));
}

#[test]
fn native_controls_keep_browser_keyboard_behavior() {
    let binding = trigger_binding(Some(&node("button")), "event=>onReset(event)", None);
    assert!(!binding.contains("tabIndex"));
    assert!(!binding.contains("onKeyDown"));
}

#[test]
fn listbox_gets_deliberate_programmatic_focus() {
    let mut listbox = node("div");
    listbox.attributes.insert("role".into(), "listbox".into());
    let binding = focus_binding(&listbox);
    assert!(is_popup(&listbox));
    assert!(binding.contains("focus({preventScroll:true})"));
    assert!(binding.contains("tabIndex={-1}"));
}
